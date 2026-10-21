//! Local document extraction: the cheap half of the attachment path.
//!
//! Office files are ZIP containers of XML and PDFs are text streams, so a
//! clean document extracts on this machine with no key, no upload and no
//! bill. The extractor itself is the Python lane at
//! `services/documents/extract.py` — one process, one JSON answer, a
//! bounded wait — because the tools it wraps (MarkItDown, LibreOffice,
//! pdftotext) are already Python's or the system's.
//!
//! This module decides *whether* to read locally and reads it; the
//! caller decides whether the answer is good enough or whether the file
//! is one of the cases (a scan, a layout-heavy PDF, media) that earns a
//! hosted parse. It never touches the network and never executes a
//! macro: Office documents are parsed as their XML parts, and legacy
//! binaries are converted by LibreOffice under `--safe-mode`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// The largest file this path will read. The hosted parser has its own
/// page and size bounds; this is the local lane's answer to a file that
/// is not a document any more.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// How long one local extraction may take before the child is killed and
/// the attachment is refused by name.
const LOCAL_TIMEOUT: Duration = Duration::from_secs(120);

/// What the local path should do with one file, by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// A format the local lane reads directly.
    Local,
    /// A PDF: try the text layer, and escalate when there is none.
    Pdf,
    /// A format local extraction does not handle well; the caller may
    /// escalate it to the hosted parser.
    Escalate,
    /// No supported parser at all.
    Unsupported,
}

/// One successful local read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalText {
    /// The lane's own name for the engine (`local:docx`,
    /// `local:markitdown`, `local:pdftotext`, `local:libreoffice`).
    pub parser: String,
    pub markdown: String,
    pub pages: Option<u64>,
}

/// What the caller should do with a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Extracted(LocalText),
    /// Local reading was tried and could not produce text; the reason is
    /// the lane's sentence and is safe to show.
    Escalate {
        reason: String,
    },
    /// The local path refused the file outright.
    Refused {
        code: String,
        reason: String,
    },
}

/// The route for one path's extension.
pub fn route(path: &Path) -> Route {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "docx" | "docm" | "pptx" | "pptm" | "xlsx" | "xlsm" | "csv" | "html" | "htm" | "md"
        | "txt" | "log" | "xml" | "json" | "doc" | "xls" | "ppt" => Route::Local,
        "pdf" => Route::Pdf,
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "webp" | "bmp" | "eml" | "msg" | "oft"
        | "mp3" | "wav" | "m4a" | "flac" | "mpga" | "oga" | "ogg" | "mp4" | "mov" | "avi"
        | "mkv" | "vsdx" | "vsd" | "parquet" => Route::Escalate,
        _ => Route::Unsupported,
    }
}

/// Read one document locally, under the lane's bounds.
///
/// `services_root` is the directory holding the bundled services; the
/// extractor is `documents/extract.py` under it. A home without the lane
/// is a named refusal, not a crash.
pub fn extract(path: &Path, services_root: &Path) -> Outcome {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Outcome::Refused {
                code: "unreadable".to_string(),
                reason: format!("{}: {error}", path.display()),
            }
        }
    };
    if metadata.len() > MAX_BYTES {
        return Outcome::Refused {
            code: "too_large".to_string(),
            reason: format!(
                "{} is larger than the local extraction bound of {} MB",
                path.display(),
                MAX_BYTES / (1024 * 1024)
            ),
        };
    }
    let lane = services_root.join("documents").join("extract.py");
    if !lane.is_file() {
        return Outcome::Refused {
            code: "missing_tool".to_string(),
            reason: format!("the documents lane is not installed at {}", lane.display()),
        };
    }
    let python = std::env::var("FLEET_DOCUMENTS_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let mut command = Command::new(python);
    command
        .arg(&lane)
        .arg("--path")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (stdout, stderr, timed_out) = match run_bounded(command, LOCAL_TIMEOUT) {
        Ok(result) => result,
        Err(reason) => {
            return Outcome::Refused {
                code: "missing_tool".to_string(),
                reason,
            }
        }
    };
    if timed_out {
        return Outcome::Refused {
            code: "timeout".to_string(),
            reason: format!(
                "local extraction did not finish within {}s",
                LOCAL_TIMEOUT.as_secs()
            ),
        };
    }
    let value: Value = match serde_json::from_str(stdout.trim()) {
        Ok(value) => value,
        Err(error) => {
            return Outcome::Refused {
                code: "unsupported".to_string(),
                reason: format!(
                    "the documents lane answered with no JSON ({error}){}",
                    if stderr.trim().is_empty() {
                        String::new()
                    } else {
                        format!(": {}", stderr.trim())
                    }
                ),
            }
        }
    };
    if value.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let markdown = value
            .get("markdown")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if markdown.trim().is_empty() {
            return Outcome::Escalate {
                reason: "the local lane produced no text".to_string(),
            };
        }
        return Outcome::Extracted(LocalText {
            parser: value
                .get("parser")
                .and_then(Value::as_str)
                .unwrap_or("local")
                .to_string(),
            markdown,
            pages: value.get("pages").and_then(Value::as_u64),
        });
    }
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("unsupported")
        .to_string();
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("the local lane refused the file")
        .to_string();
    // A PDF with no text layer is not a refusal: it is the scan the
    // hosted parser exists for. Every other refusal is the file's own.
    if code == "empty" && route(path) == Route::Pdf {
        return Outcome::Escalate { reason };
    }
    Outcome::Refused { code, reason }
}

/// Run a command with a deadline. Returns `(stdout, stderr, timed_out)`;
/// the error case is a spawn failure (a missing interpreter).
fn run_bounded(mut command: Command, timeout: Duration) -> Result<(String, String, bool), String> {
    let mut child = command.spawn().map_err(|error| {
        format!(
            "the documents lane could not start ({}): {error}",
            command.get_program().to_string_lossy()
        )
    })?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stream) = stdout.as_mut() {
            let _ = stream.read_to_string(&mut text);
        }
        text
    });
    let err_reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stream) = stderr.as_mut() {
            let _ = stream.read_to_string(&mut text);
        }
        text
    });
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "the documents lane could not be waited on: {error}"
                ));
            }
        }
    }
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok((stdout, stderr, timed_out))
}

/// The fixtures that ship with the lane, for its own tests and for a
/// caller that wants to check the toolchain without a document.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("services/documents/fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("services")
    }

    fn fixture(name: &str) -> PathBuf {
        fixtures_dir().join(name)
    }

    #[test]
    fn clean_office_formats_extract_locally() {
        let cases = [
            ("simple.docx", "local:docx", "Ada"),
            ("simple.pptx", "local:pptx", "Speaker note here"),
            ("simple.xlsx", "local:xlsx", "=1+1"),
            ("simple.csv", "local:csv", "alpha"),
            ("simple.html", "local:html", "Body text"),
            ("text.pdf", "local:pdftotext", "fixture document"),
        ];
        for (name, parser, needle) in cases {
            match extract(&fixture(name), &lane()) {
                Outcome::Extracted(text) => {
                    assert_eq!(text.parser, parser, "{name}");
                    assert!(text.markdown.contains(needle), "{name}: {text:?}");
                }
                other => panic!("{name}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_pdf_with_no_text_layer_is_the_escalation_case() {
        match extract(&fixture("blank.pdf"), &lane()) {
            Outcome::Escalate { reason } => assert!(reason.contains("scan"), "{reason}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_format_with_no_local_reader_is_an_escalation_not_a_failure() {
        assert_eq!(route(&fixture("pixel.png")), Route::Escalate);
        assert_eq!(route(Path::new("x.eml")), Route::Escalate);
        assert_eq!(route(Path::new("x.mp4")), Route::Escalate);
        assert_eq!(route(Path::new("x.unknown")), Route::Unsupported);
    }

    #[test]
    fn a_missing_lane_is_named_rather_than_crashed() {
        let empty = std::env::temp_dir().join(format!("fleet-docs-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).expect("the scratch dir");
        match extract(&fixture("simple.docx"), &empty) {
            Outcome::Refused { code, reason } => {
                assert_eq!(code, "missing_tool");
                assert!(reason.contains("documents lane"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn a_macro_enabled_document_is_read_as_xml_not_executed() {
        let dir = std::env::temp_dir().join(format!(
            "fleet-docm-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("the dir");
        let path = dir.join("probe.docm");
        std::fs::copy(fixture("simple.docx"), &path).expect("the copy");
        match extract(&path, &lane()) {
            Outcome::Extracted(text) => assert!(text.markdown.contains("Ada"), "{text:?}"),
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_oversized_file_is_refused_before_the_lane_runs() {
        let path = std::env::temp_dir().join(format!("fleet-big-{}", std::process::id()));
        let file = std::fs::File::create(&path).expect("the file");
        file.set_len(MAX_BYTES + 1).expect("the length");
        match extract(&path, &lane()) {
            Outcome::Refused { code, reason } => {
                assert_eq!(code, "too_large");
                assert!(reason.contains("larger"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_copy_names_the_recipients() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let js = std::fs::read_to_string(root.join("src/gui/assets/app.js")).expect("app.js");
        let html =
            std::fs::read_to_string(root.join("src/gui/assets/index.html")).expect("index.html");
        assert!(
            !js.contains("Nothing leaves this machine"),
            "app.js still claims it"
        );
        assert!(
            !html.contains("Nothing leaves this machine"),
            "index.html still claims it"
        );
        assert!(
            js.contains("providers that serve them"),
            "app.js does not name the recipients"
        );
    }

    #[test]
    fn the_route_names_the_office_and_media_classes() {
        assert_eq!(route(Path::new("a.docx")), Route::Local);
        assert_eq!(route(Path::new("a.docm")), Route::Local);
        assert_eq!(route(Path::new("a.xlsm")), Route::Local);
        assert_eq!(route(Path::new("a.pdf")), Route::Pdf);
    }
}
