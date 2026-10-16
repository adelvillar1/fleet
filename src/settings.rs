//! The settings surface: one writer over `<FLEET_HOME>/config.json`,
//! `<FLEET_HOME>/routing.json`, and `<FLEET_HOME>/fleet.env`, with the
//! TUI pane and `fleet settings set` as two views over it.
//!
//! The knobs fleet reads live in three files and the environment, and
//! until now the only way to change one was to hand-edit a file. This
//! module is the writer the GUI and the CLI share, so a scripted change
//! and a GUI change land identically — which is what lets a desktop
//! session build on a finished contract rather than invent one.
//!
//! Three rules carry the whole design:
//!
//! - **The loader's tolerance, and strictly more strictness on write.**
//!   A missing file is the defaults, exactly as `route::load_roster`
//!   reads it. But a write validates against the same schema the loader
//!   enforces — `deny_unknown_fields` on a provider entry, a confidence
//!   in `0..1`, a similarity clamped into range — and refuses before
//!   the rename rather than after. The GUI must never be a way around
//!   the file's own validation.
//! - **Unknown fields survive.** The writer reads the file as a
//!   `serde_json::Value` and edits the keys it owns, so an owner's
//!   hand-written comment-shaped extras are preserved rather than
//!   dropped by a round-trip through a stricter struct.
//! - **Atomic, and never a half-written config.** The write goes
//!   through a unique temp file and a rename, so a crash mid-edit
//!   cannot leave a config that fails the next load. A key's VALUE is
//!   never rendered — the surface shows set/unset beside the name, per
//!   the standing credential rule.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// The env-var file's name, beside `.env` rather than inside it: the
/// credential resolution already walks `<FLEET_HOME>/.env`, and a
/// second file the GUI owns keeps a hand-edited `.env` from being
/// rewritten underneath its owner.
const ENV_FILE: &str = "fleet.env";

/// One setting the surface can address, as the writer sees it. The
/// key is what `fleet settings set` takes and what the TUI pane lists;
/// the kind is what decides the editor shape and the validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Free text — a path, a url, a model id.
    Text,
    /// A boolean toggle.
    Toggle,
    /// One of a fixed set of words.
    Choice(&'static [&'static str]),
    /// A number in `0..1` — the confidences and similarities.
    Ratio,
    /// A whole number — the session counts and turn budgets.
    Count,
}

/// One addressable setting: its key, its kind, the file it lives in,
/// and the sentence the surface renders beside it.
#[derive(Debug, Clone, Copy)]
pub struct Setting {
    pub key: &'static str,
    pub kind: Kind,
    pub file: File,
    pub summary: &'static str,
}

/// Which of the home's files a setting writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum File {
    /// `<FLEET_HOME>/config.json`.
    Config,
    /// `<FLEET_HOME>/routing.json`.
    Routing,
    /// `<FLEET_HOME>/fleet.env` — the env-var counterpart.
    Env,
}

/// Every setting the surface addresses, in the order the pane lists
/// them. One table, so the TUI, the CLI, and the tests can never
/// disagree about what exists.
pub const SETTINGS: &[Setting] = &[
    // --- config.json: the roster and the run's own knobs ---------------
    Setting {
        key: "pin",
        kind: Kind::Text,
        file: File::Config,
        summary: "The provider every workload walks first. Empty is no pin.",
    },
    Setting {
        key: "approvals",
        kind: Kind::Choice(&["ask", "yes", "yes-guarded"]),
        file: File::Config,
        summary: "Whether tool calls ask. `yes-guarded` still asks about the destructive class.",
    },
    Setting {
        key: "calib.log_root",
        kind: Kind::Text,
        file: File::Config,
        summary: "Where the calibration stream lives. Empty is dev-decisions' own default.",
    },
    Setting {
        key: "calib.informs_failover",
        kind: Kind::Toggle,
        file: File::Config,
        summary: "Whether a fit, wrong-heavy judged sample may skip a provider in the failover.",
    },
    Setting {
        key: "skills.retire_after_sessions",
        kind: Kind::Count,
        file: File::Config,
        summary: "Sessions a skill may sit unused before curation proposes retiring it.",
    },
    Setting {
        key: "skills.merge_similarity",
        kind: Kind::Ratio,
        file: File::Config,
        summary: "Description similarity at or above which curation proposes merging two skills.",
    },
    // --- routing.json: the preference table and its gates ---------------
    Setting {
        key: "gates.min_confidence",
        kind: Kind::Ratio,
        file: File::Routing,
        summary: "The lane's own confidence below which its judgment is a fallback, not a route.",
    },
    Setting {
        key: "gates.default_workload",
        kind: Kind::Text,
        file: File::Routing,
        summary: "The workload a gated or absent judgment falls back to.",
    },
    // --- fleet.env: the env-var counterparts ----------------------------
    Setting {
        key: "FLEET_EMBEDDING_ENDPOINT",
        kind: Kind::Text,
        file: File::Env,
        summary:
            "The embedding endpoint recall's vector channel probes. Empty is the loopback default.",
    },
    Setting {
        key: "FLEET_CUA_DRIVER",
        kind: Kind::Text,
        file: File::Env,
        summary:
            "The cua driver binary the desktop tool launches. Empty resolves the tools directory.",
    },
];

/// The setting a key addresses, or `None` for a key the surface does
/// not own. A caller that gets `None` says so by name rather than
/// writing a key nothing reads.
pub fn find(key: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|setting| setting.key == key)
}

/// The home's three files' paths, resolved once.
fn paths(home: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        home.join("config.json"),
        home.join("routing.json"),
        home.join(ENV_FILE),
    )
}

/// Read one of the home's JSON files as a value. A missing file is an
/// empty object — the defaults — exactly as the loader reads it. A
/// malformed one is an error the caller names, because a config too
/// broken to parse has already failed the loader loud and a writer
/// that silently overwrote it would destroy the evidence.
fn read_json(path: &Path) -> Result<Value, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Value::Object(Map::new()));
        }
        Err(error) => return Err(format!("{} is unreadable: {error}", path.display())),
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value) if value.is_object() => Ok(value),
        Ok(_) => Err(format!("{} is not a JSON object", path.display())),
        Err(error) => Err(format!("{} is not valid JSON: {error}", path.display())),
    }
}

/// Validate a proposed value against its setting's kind, returning the
/// value to store. A refusal names the setting and the rule — the GUI
/// must never be a way around the file's own validation, so the checks
/// here are the loader's checks, not a second opinion.
fn validate(setting: &Setting, value: &str) -> Result<Value, String> {
    let trimmed = value.trim();
    match setting.kind {
        Kind::Text => Ok(Value::String(trimmed.to_string())),
        Kind::Toggle => match trimmed.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok(Value::Bool(true)),
            "false" | "off" | "no" | "0" => Ok(Value::Bool(false)),
            _ => Err(format!(
                "`{}` is a toggle: true or false, not `{trimmed}`",
                setting.key
            )),
        },
        Kind::Choice(words) => {
            let lowered = trimmed.to_ascii_lowercase();
            words
                .iter()
                .find(|word| **word == lowered)
                .map(|word| Value::String((*word).to_string()))
                .ok_or_else(|| {
                    format!(
                        "`{}` is one of {}, not `{trimmed}`",
                        setting.key,
                        words.join(", ")
                    )
                })
        }
        Kind::Ratio => {
            let number: f64 = trimmed
                .parse()
                .map_err(|_| format!("`{}` is a number in 0..1, not `{trimmed}`", setting.key))?;
            if !(0.0..=1.0).contains(&number) {
                return Err(format!(
                    "`{}` is a ratio in 0..1, and {number} is outside it",
                    setting.key
                ));
            }
            Ok(serde_json::Number::from_f64(number)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(trimmed.to_string())))
        }
        Kind::Count => {
            let number: u64 = trimmed
                .parse()
                .map_err(|_| format!("`{}` is a whole number, not `{trimmed}`", setting.key))?;
            Ok(Value::Number(number.into()))
        }
    }
}

/// The JSON pointer a dotted key addresses, as a path of segments.
/// `calib.log_root` is `["calib", "log_root"]`; a bare key is itself.
fn segments(key: &str) -> Vec<&str> {
    key.split('.').collect()
}

/// Write one setting through its file. The file is read as a value, the
/// one key is set, and the whole object is written back atomically —
/// so an owner's hand-written extras survive and a crash mid-edit
/// cannot leave a half-written config.
pub fn set(home: &Path, key: &str, value: &str) -> Result<(), String> {
    let Some(setting) = find(key) else {
        return Err(format!(
            "`{key}` is not a setting; `fleet settings` lists the ones that are"
        ));
    };
    let stored = validate(setting, value)?;
    let (config_path, routing_path, env_path) = paths(home);
    let path = match setting.file {
        File::Config => config_path,
        File::Routing => routing_path,
        File::Env => {
            // The env file is `NAME=value` lines, not JSON: a different
            // writer over the same atomic rule.
            return write_env(&env_path, key, stored.as_str().unwrap_or_default());
        }
    };
    let mut document = read_json(&path)?;
    let segments = segments(key);
    let Some(object) = document.as_object_mut() else {
        return Err(format!("{} is not a JSON object", path.display()));
    };
    // Walk to the parent, creating the section objects on the way, then
    // set the leaf. A missing `calib` section is created rather than
    // refused: the loader reads an absent section as the defaults, so
    // writing one is the same claim.
    let mut cursor = object;
    for segment in &segments[..segments.len() - 1] {
        let entry = cursor
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            return Err(format!(
                "`{}` is not a section; `{}` names a value",
                segments[..segments.len() - 1].join("."),
                key
            ));
        }
        cursor = entry.as_object_mut().expect("just checked it is an object");
    }
    cursor.insert(segments[segments.len() - 1].to_string(), stored);
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|e| e.to_string())?,
    )
}

/// Write one `NAME=value` line into the env file, preserving every
/// other line and its order. A key's value is never logged by this
/// module; the caller decides what to render.
fn write_env(path: &Path, name: &str, value: &str) -> Result<(), String> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            lines.push(line.to_string());
            continue;
        }
        match trimmed.split_once('=') {
            Some((key, _)) if key.trim() == name => {
                lines.push(format!("{name}={value}"));
                replaced = true;
            }
            _ => lines.push(line.to_string()),
        }
    }
    if !replaced {
        lines.push(format!("{name}={value}"));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    write_atomic(path, &text)
}

/// Write `text` to `path` through a unique temp file and a rename, so a
/// reader never sees a half-written file and two fleet processes
/// cannot collide on one temp. Best-effort and silent about the temp's
/// own cleanup: a failed rename leaves the original intact.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("{} cannot be created: {error}", parent.display()))?;
    }
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temp, text)
        .map_err(|error| format!("{} cannot be written: {error}", temp.display()))?;
    fs::rename(&temp, path)
        .map_err(|error| format!("{} cannot be replaced: {error}", path.display()))?;
    Ok(())
}

/// One setting's current state, as the surface renders it. A key's
/// VALUE is never carried: the env file's entries report set/unset
/// beside the name, per the standing credential rule, and the JSON
/// files' values are shown because they are configuration rather than
/// secrets.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub key: &'static str,
    pub kind: Kind,
    pub file: File,
    pub summary: &'static str,
    /// The current value as a display string, or `None` when unset.
    pub value: Option<String>,
    /// True when the value came from the environment rather than the
    /// file — the surface says which, because a file write would not
    /// take effect until the env var is unset.
    pub from_env: bool,
}

impl State {
    /// The value as the pane renders it: the word `unset` for an absent
    /// one, and never a raw secret.
    pub fn rendered(&self) -> String {
        match &self.value {
            Some(_) if self.file == File::Env && is_secret_name(self.key) => "set".to_string(),
            Some(value) => value.clone(),
            None => "unset".to_string(),
        }
    }

    /// Where the value comes from, as one word: `file`, `env`, or
    /// `default`.
    pub fn source(&self) -> &'static str {
        if self.from_env {
            "env"
        } else if self.value.is_some() {
            "file"
        } else {
            "default"
        }
    }
}

/// The env-var names whose values are never rendered. A provider key is
/// a secret; the endpoint and the driver path are configuration.
fn is_secret_name(name: &str) -> bool {
    name.ends_with("_KEY") || name.ends_with("_TOKEN") || name.ends_with("_SECRET")
}

/// Read every setting's current state for the surface. The env file is
/// read beside the process environment, and the environment wins — the
/// same precedence the credential resolution runs on, so what the pane
/// shows is what a run would actually read.
pub fn states(home: &Path) -> Vec<State> {
    let (config_path, routing_path, env_path) = paths(home);
    let config = read_json(&config_path).unwrap_or(Value::Object(Map::new()));
    let routing = read_json(&routing_path).unwrap_or(Value::Object(Map::new()));
    let env_file = read_env_file(&env_path);
    let process_env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());

    SETTINGS
        .iter()
        .map(|setting| {
            // An env-file setting's value comes from the environment or
            // the env file — never from a JSON document — read through
            // the same precedence a run uses: the process environment
            // first, the file second, and `from_env` says which answered.
            let (value, from_env) = if setting.file == File::Env {
                match process_env(setting.key) {
                    Some(value) => (Some(value), true),
                    None => (env_file.get(setting.key).cloned(), false),
                }
            } else {
                let document = match setting.file {
                    File::Config => &config,
                    File::Routing => &routing,
                    File::Env => unreachable!("the env branch returned above"),
                };
                let found = lookup(document, setting.key).map(|value| render_value(&value));
                (found, false)
            };
            State {
                key: setting.key,
                kind: setting.kind,
                file: setting.file,
                summary: setting.summary,
                value,
                from_env,
            }
        })
        .collect()
}

/// One dotted key's value out of a JSON document, or `None` when the
/// path is absent or names a section rather than a leaf.
fn lookup(document: &Value, key: &str) -> Option<Value> {
    let segments = segments(key);
    let mut cursor = document;
    for segment in &segments {
        cursor = cursor.get(*segment)?;
    }
    (!cursor.is_object() && !cursor.is_array()).then(|| cursor.clone())
}

/// One JSON value as the pane's display string. A toggle renders as its
/// own word; a ratio renders with two decimals; anything else as its
/// text.
fn render_value(value: &Value) -> String {
    match value {
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => {
            if let Some(float) = number.as_f64() {
                if float.fract() == 0.0 && float.abs() < 1e15 {
                    return format!("{}", float as i64);
                }
                format!("{float:.2}")
            } else {
                number.to_string()
            }
        }
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The env file's entries, in file order, as a map. A malformed line is
/// skipped the way `home::read_dotenv` skips one — the file is the
/// GUI's, and a stray line is not a reason to refuse the whole read.
fn read_env_file(path: &Path) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let Ok(text) = fs::read_to_string(path) else {
        return values;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, value)) = trimmed.split_once('=') {
            values.insert(name.trim().to_string(), value.trim().to_string());
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-settings-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_setting_round_trips_through_its_file() {
        let home = scratch("round-trip");
        // One value per kind, written and read back.
        for (key, written, expected) in [
            ("pin", "stepfun", "stepfun"),
            ("approvals", "yes-guarded", "yes-guarded"),
            ("calib.informs_failover", "false", "false"),
            ("skills.merge_similarity", "0.8", "0.80"),
            ("skills.retire_after_sessions", "12", "12"),
            ("gates.min_confidence", "0.7", "0.70"),
            (
                "FLEET_EMBEDDING_ENDPOINT",
                "http://127.0.0.1:8901/v1/embeddings",
                "http://127.0.0.1:8901/v1/embeddings",
            ),
        ] {
            set(&home, key, written).unwrap_or_else(|error| panic!("{key}: {error}"));
            let state = states(&home)
                .into_iter()
                .find(|state| state.key == key)
                .unwrap_or_else(|| panic!("{key} is not a setting"));
            assert_eq!(state.value.as_deref(), Some(expected), "{key}");
            assert_eq!(state.source(), "file", "{key}");
        }
        let _ = fs::remove_dir_all(&home);
    }

    /// A refused edit changes nothing — the GUI must never be a way
    /// around the file's own validation.
    #[test]
    fn a_refused_edit_changes_nothing() {
        let home = scratch("refused");
        set(&home, "pin", "stepfun").unwrap();
        let before = fs::read_to_string(home.join("config.json")).unwrap();
        for (key, value) in [
            ("approvals", "yolo"),
            ("gates.min_confidence", "1.5"),
            ("skills.merge_similarity", "high"),
            ("skills.retire_after_sessions", "many"),
            ("not-a-setting", "x"),
        ] {
            let error = set(&home, key, value).expect_err(&format!("{key}={value} must refuse"));
            assert!(error.contains(key) || error.contains("setting"), "{error}");
        }
        assert_eq!(
            fs::read_to_string(home.join("config.json")).unwrap(),
            before,
            "a refused edit left the file untouched"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// An owner's hand-written extras survive a GUI edit: the writer
    /// reads the file as a value and edits the one key it owns.
    #[test]
    fn an_owners_hand_written_extras_survive_an_edit() {
        let home = scratch("extras");
        fs::write(
            home.join("config.json"),
            r#"{"pin":"deepseek","a_note_i_wrote":"keep me","providers":[{"id":"x"}]}"#,
        )
        .unwrap();
        set(&home, "approvals", "yes").unwrap();
        let text = fs::read_to_string(home.join("config.json")).unwrap();
        assert!(text.contains("a_note_i_wrote"), "{text}");
        assert!(text.contains("\"providers\""), "{text}");
        assert!(text.contains("\"approvals\": \"yes\""), "{text}");
        // And the original pin is still there.
        assert!(text.contains("\"pin\": \"deepseek\""), "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    /// The env writer preserves every other line and its order, and a
    /// key's value never rides a rendered state.
    #[test]
    fn the_env_writer_preserves_the_file_and_never_renders_a_secret() {
        let home = scratch("env");
        fs::write(
            home.join(ENV_FILE),
            "# the GUI's own env file\nSTEPFUN_API_KEY=sk-live-value\nFLEET_HOME=/x\n",
        )
        .unwrap();
        set(
            &home,
            "FLEET_EMBEDDING_ENDPOINT",
            "http://127.0.0.1:8901/v1/embeddings",
        )
        .unwrap();
        let text = fs::read_to_string(home.join(ENV_FILE)).unwrap();
        assert!(text.contains("STEPFUN_API_KEY=sk-live-value"), "{text}");
        assert!(text.contains("# the GUI's own env file"), "{text}");
        assert!(text.contains("FLEET_HOME=/x"), "{text}");
        assert!(
            text.contains("FLEET_EMBEDDING_ENDPOINT=http://127.0.0.1:8901/v1/embeddings"),
            "{text}"
        );
        // A second write to the same key replaces rather than appends.
        set(
            &home,
            "FLEET_EMBEDDING_ENDPOINT",
            "http://127.0.0.1:9999/v1/embeddings",
        )
        .unwrap();
        let text = fs::read_to_string(home.join(ENV_FILE)).unwrap();
        assert_eq!(
            text.matches("FLEET_EMBEDDING_ENDPOINT=").count(),
            1,
            "{text}"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// The atomic write leaves no temp file behind on success.
    #[test]
    fn the_atomic_write_leaves_no_temp_behind() {
        let home = scratch("atomic");
        set(&home, "pin", "stepfun").unwrap();
        let leftovers: Vec<String> = fs::read_dir(&home)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        let _ = fs::remove_dir_all(&home);
    }

    /// A malformed file is a named error rather than a silent
    /// overwrite: a config too broken to parse has already failed the
    /// loader loud, and a writer that destroyed it would destroy the
    /// evidence.
    #[test]
    fn a_malformed_file_is_a_named_error_not_a_silent_overwrite() {
        let home = scratch("malformed");
        fs::write(home.join("config.json"), "{ not json at all").unwrap();
        let error = set(&home, "pin", "stepfun").expect_err("a malformed file must refuse");
        assert!(error.contains("not valid JSON"), "{error}");
        assert_eq!(
            fs::read_to_string(home.join("config.json")).unwrap(),
            "{ not json at all",
            "the malformed file is preserved as evidence"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// The settings table is the single source of what exists, and
    /// every entry names a real file and a real kind.
    #[test]
    fn the_table_is_the_one_source_of_what_exists() {
        assert!(SETTINGS.len() >= 10, "the inventory the plan audited");
        for setting in SETTINGS {
            assert!(!setting.key.is_empty());
            assert!(!setting.summary.is_empty());
            assert!(find(setting.key).is_some(), "{}", setting.key);
        }
        assert!(find("not-a-setting").is_none());
    }
}

// --- the surfaces ----------------------------------------------------------

/// The card width, matching `lanes`, `providers`, and `calib`.
const CARD_WIDTH: usize = 72;

mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

use palette::{ACCENT, BOLD, DIM, RESET, TEXT, WARN};

/// `fleet settings`: every addressable setting as cards — never a
/// plain-text table, the house UX rule. Each card names the key, its
/// current value, where that value came from, and the sentence saying
/// what it does. A value from the environment is marked, because a file
/// write would not take effect until the env var is unset.
pub fn render(home: &Path) -> String {
    let states = states(home);
    let mut out = format!(
        "{BOLD}fleet settings{RESET} {DIM}\u{2014} every knob the surface can address{RESET}\n\n"
    );
    for state in &states {
        let title = state.key;
        let value = state.rendered();
        let right = format!("{value} \u{b7} {source}", source = state.source());
        let header = format!(
            "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{title}{RESET} {DIM}{dashes} {color}{right}{RESET} {DIM}\u{2510}{RESET}",
            dashes = dashes_for(&format!(" {title} "), &right),
            color = if state.from_env { WARN } else { TEXT },
        );
        out.push_str(&header);
        out.push('\n');
        for line in wrap_detail(state.summary, CARD_WIDTH - 6) {
            out.push_str(&format!("{DIM}\u{2502}{RESET} {TEXT}{line}{RESET}\n"));
        }
        out.push_str(&format!(
            "{DIM}\u{2502}{RESET} {DIM}{} \u{b7} {}{RESET}\n",
            file_name(state.file),
            kind_name(state.kind)
        ));
        out.push_str(&format!(
            "{DIM}\u{2514}{}\u{2518}{RESET}\n",
            "\u{2500}".repeat(CARD_WIDTH - 2)
        ));
    }
    out.push_str(&format!(
        "\n{DIM}{} settings \u{b7} `fleet settings set <key> <value>` writes one \u{b7} a key's \\
         value is never printed{RESET}\n",
        states.len()
    ));
    out
}

/// The file a setting lives in, as the card names it.
fn file_name(file: File) -> &'static str {
    match file {
        File::Config => "config.json",
        File::Routing => "routing.json",
        File::Env => ENV_FILE,
    }
}

/// The kind a setting takes, as the card names it — the editor shape
/// the GUI builds from.
fn kind_name(kind: Kind) -> String {
    match kind {
        Kind::Text => "text".to_string(),
        Kind::Toggle => "toggle".to_string(),
        Kind::Choice(words) => format!("one of {}", words.join(", ")),
        Kind::Ratio => "ratio 0..1".to_string(),
        Kind::Count => "whole number".to_string(),
    }
}

/// The dash run that pads a card header so its right edge aligns.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// Wrap one detail line to `width`, breaking on whitespace. A word
/// longer than the width is broken rather than overflowing the card.
fn wrap_detail(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod list_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fleet-settings-list-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A provider override validates through the roster's own schema, so
    /// a misspelled field is a named refusal rather than a silently
    /// ignored key.
    #[test]
    fn a_provider_entry_validates_through_the_rosters_own_schema() {
        let home = scratch("provider-schema");
        let error = set_provider(&home, r#"{"id":"x","keyenv":"STEPFUN_API_KEY"}"#)
            .expect_err("a misspelled field must refuse");
        assert!(
            error.contains("unknown field") || error.contains("keyenv"),
            "{error}"
        );
        // And a well-formed one lands.
        let id = set_provider(
            &home,
            r#"{"id":"blend1","base_url":"http://127.0.0.1:8420/v1","key_env":"BLEND1_BEARER_TOKEN","kind":"lane"}"#,
        )
        .unwrap();
        assert_eq!(id, "blend1");
        let _ = fs::remove_dir_all(&home);
    }

    /// The destructive rule: removing an override of a compiled-in
    /// provider reverts it; removing one the file added removes the
    /// provider from the roster entirely, and needs `force`.
    #[test]
    fn removing_an_added_provider_needs_force_and_a_compiled_in_one_does_not() {
        let home = scratch("provider-remove");
        set_provider(&home, r#"{"id":"stepfun","enabled":false}"#).unwrap();
        set_provider(
            &home,
            r#"{"id":"my-own-gateway","base_url":"http://127.0.0.1:9/v1"}"#,
        )
        .unwrap();
        // The compiled-in one reverts without a fight.
        remove_provider(&home, "stepfun", false).unwrap();
        // The added one refuses, and the refusal names why.
        let error = remove_provider(&home, "my-own-gateway", false)
            .expect_err("removing an added provider must refuse without force");
        assert!(error.contains("--force"), "{error}");
        assert!(
            error.contains("removes the provider from every walk"),
            "{error}"
        );
        // And it is still there.
        let text = fs::read_to_string(home.join("config.json")).unwrap();
        assert!(text.contains("my-own-gateway"), "{text}");
        // With force, it goes.
        remove_provider(&home, "my-own-gateway", true).unwrap();
        let text = fs::read_to_string(home.join("config.json")).unwrap();
        assert!(!text.contains("my-own-gateway"), "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    /// A workload's candidates must be `provider/model` pairs — the same
    /// rule `load_routing` enforces, so a typo fails here rather than at
    /// the next startup.
    #[test]
    fn a_workloads_candidates_must_be_provider_model_pairs() {
        let home = scratch("workload-shape");
        let error = set_workload(&home, "quick", &["stepfun".to_string()])
            .expect_err("a bare provider is not a pair");
        assert!(error.contains("provider/model"), "{error}");
        // An empty list is refused: a workload nothing can serve.
        let error = set_workload(&home, "quick", &[]).expect_err("no candidates must refuse");
        assert!(error.contains("at least one"), "{error}");
        // And a well-formed one lands, replacing in place.
        set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()]).unwrap();
        set_workload(
            &home,
            "quick",
            &["zai-coding-plan/GLM-5.3-Flash".to_string()],
        )
        .unwrap();
        let text = fs::read_to_string(home.join("routing.json")).unwrap();
        assert_eq!(text.matches("\"quick\"").count(), 1, "{text}");
        assert!(text.contains("GLM-5.3-Flash"), "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    /// The same destructive rule on the workloads: a compiled-in
    /// workload's preference reverts, an added one needs force.
    #[test]
    fn removing_an_added_workload_needs_force() {
        let home = scratch("workload-remove");
        set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()]).unwrap();
        set_workload(
            &home,
            "my-own-shape",
            &["stepfun/step-3.7-flash".to_string()],
        )
        .unwrap();
        remove_workload(&home, "quick", false).unwrap();
        let error = remove_workload(&home, "my-own-shape", false)
            .expect_err("removing an added workload must refuse without force");
        assert!(error.contains("--force"), "{error}");
        remove_workload(&home, "my-own-shape", true).unwrap();
        let _ = fs::remove_dir_all(&home);
    }

    /// The compiled-in sets are the rule's source, and they are read
    /// from the crate rather than duplicated here.
    #[test]
    fn the_compiled_in_sets_are_the_rules_source() {
        let providers = compiled_in_providers();
        assert!(providers.contains(&"stepfun".to_string()), "{providers:?}");
        assert!(providers.contains(&"deepseek".to_string()), "{providers:?}");
        let workloads = compiled_in_workloads();
        assert!(workloads.contains(&"quick".to_string()), "{workloads:?}");
        assert!(workloads.contains(&"composed".to_string()), "{workloads:?}");
    }

    /// `list_states` reads the home's two lists as card states, and
    /// classifies each entry against the crate's own compiled-in sets —
    /// the same ones the writer's destructive rule reads, so the pane
    /// can never disagree with it about which class an entry is in.
    #[test]
    fn list_states_reads_the_homes_two_lists_and_classifies_them() {
        let home = scratch("list-states");
        // An empty home carries no cards at all.
        assert!(list_states(&home).is_empty(), "no files, no cards");
        set_provider(&home, r#"{"id":"stepfun","enabled":false}"#).unwrap();
        set_provider(
            &home,
            r#"{"id":"my-own-gateway","base_url":"http://127.0.0.1:9/v1"}"#,
        )
        .unwrap();
        set_workload(&home, "quick", &["stepfun/step-3.7-flash".to_string()]).unwrap();
        set_workload(
            &home,
            "my-own-shape",
            &["stepfun/step-3.7-flash".to_string()],
        )
        .unwrap();
        let states = list_states(&home);
        let providers: Vec<&ListState> = states
            .iter()
            .filter(|entry| entry.kind == ListKind::Provider)
            .collect();
        let workloads: Vec<&ListState> = states
            .iter()
            .filter(|entry| entry.kind == ListKind::Workload)
            .collect();
        assert_eq!(providers.len(), 2, "{states:?}");
        assert_eq!(workloads.len(), 2, "{states:?}");
        // The compiled-in override reverts; the file-added one is the
        // destructive class. Read from the crate, not decided here.
        let stepfun = providers
            .iter()
            .find(|entry| entry.key == "stepfun")
            .expect("the stepfun override");
        assert!(stepfun.reverts, "a compiled-in id's override reverts");
        assert!(stepfun.detail.contains("disabled"), "{}", stepfun.detail);
        let own = providers
            .iter()
            .find(|entry| entry.key == "my-own-gateway")
            .expect("the added provider");
        assert!(
            !own.reverts,
            "an added provider's removal is the destructive one"
        );
        assert!(
            own.detail.contains("http://127.0.0.1:9/v1"),
            "{}",
            own.detail
        );
        // The seed is the buffer the writer parses: writing it back is
        // the same entry, in place, not a second one.
        assert_eq!(set_provider(&home, &own.seed).unwrap(), "my-own-gateway");
        assert_eq!(list_states(&home).len(), 4, "the seed round-trips in place");
        // The workloads classify the same way, and the seed is the
        // comma-joined list the writer's own branch reads.
        let quick = workloads
            .iter()
            .find(|entry| entry.key == "quick")
            .expect("the quick preference");
        assert!(quick.reverts, "a compiled-in workload's preference reverts");
        assert_eq!(quick.seed, "stepfun/step-3.7-flash");
        assert!(
            quick.detail.contains("stepfun/step-3.7-flash"),
            "{}",
            quick.detail
        );
        let own_shape = workloads
            .iter()
            .find(|entry| entry.key == "my-own-shape")
            .expect("the added workload");
        assert!(
            !own_shape.reverts,
            "an added workload's removal is the destructive one"
        );
        let _ = fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-settings-render-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The surface is cards, never a table: every setting renders inside
    /// a box, and no line is a padded column row.
    #[test]
    fn the_surface_is_cards_and_never_a_table() {
        let home = scratch("cards");
        set(&home, "pin", "stepfun").unwrap();
        set(&home, "approvals", "yes-guarded").unwrap();
        let rendered = render(&home);
        assert!(rendered.contains("fleet settings"), "{rendered}");
        // One box per setting, plus the header's own count line.
        assert_eq!(
            rendered.matches('\u{250c}').count(),
            SETTINGS.len(),
            "one card per setting"
        );
        assert!(rendered.contains("pin"), "{rendered}");
        assert!(rendered.contains("yes-guarded"), "{rendered}");
        // The value's source is named, so a file write that would be
        // shadowed by the environment says so.
        assert!(rendered.contains("file"), "{rendered}");
        // And the footer names the writer.
        assert!(rendered.contains("fleet settings set"), "{rendered}");
        let _ = fs::remove_dir_all(&home);
    }

    /// An empty home renders every setting as `unset`, which is the
    /// honest reading of a machine that has configured nothing.
    #[test]
    fn an_empty_home_renders_every_setting_as_unset() {
        let home = scratch("empty");
        let rendered = render(&home);
        assert!(rendered.contains("unset"), "{rendered}");
        assert_eq!(
            rendered.matches("unset \u{b7} default").count(),
            SETTINGS.len(),
            "every unset setting names its source as the default"
        );
        let _ = fs::remove_dir_all(&home);
    }
}

// --- the list-shaped surfaces ----------------------------------------------

/// The ids the compiled-in roster carries, before any config merges
/// over them. The settings writer's whole destructive rule reads this
/// set: an override whose id is in here can be removed (it reverts to
/// the compiled-in default), and an id that is not in here is a
/// provider the config file itself added — removing it removes the
/// provider from the roster entirely.
pub fn compiled_in_providers() -> Vec<String> {
    crate::route::default_roster()
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect()
}

/// The workload names the compiled-in preference table carries. The
/// same rule as the providers: removing an override reverts, removing
/// an added workload deletes it.
pub fn compiled_in_workloads() -> Vec<String> {
    crate::route::default_preferences()
        .into_iter()
        .map(|preference| preference.workload.to_string())
        .collect()
}

/// One list-shaped entry as the surface renders it: a provider override
/// or a workload preference the home's own files carry. The scalars are
/// [`State`]s and stay exactly the table they are; these are the two
/// lists beside them, and each one carries its destructive class so the
/// pane never re-derives the rule the writer already enforces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListState {
    /// Which of the two lists this entry belongs to.
    pub kind: ListKind,
    /// The entry's key material: the provider's id, or the workload's
    /// name — what `remove_provider` and `remove_workload` address.
    pub key: String,
    /// The card's one line: a provider's base_url and key env, or a
    /// workload's candidates in walk order.
    pub detail: String,
    /// True when removing this entry reverts it to the compiled-in
    /// default, because its key is one the compiled-in roster or
    /// preference table carries. False when the file added the entry,
    /// so removing it takes the entry out of every walk that names it.
    pub reverts: bool,
    /// The buffer the editor seeds with, in the shape the writer's own
    /// branch parses: a provider entry's JSON, or a workload's
    /// comma-joined candidates.
    pub seed: String,
}

/// Which of the home's two list-shaped surfaces an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// A provider override in config.json's `providers` list.
    Provider,
    /// A workload preference in routing.json's `workloads` list.
    Workload,
}

/// Read the home's two list-shaped surfaces as card states: the provider
/// overrides config.json carries, then the workload preferences
/// routing.json carries. A sibling of [`states`], which keeps the
/// scalars — the lists are their own source, read from the very files
/// the writer writes.
///
/// Each entry's destructive class is read from the crate's compiled-in
/// roster and preference table through the same helpers the writer's
/// own rule reads, never a copy of them: an id the compiled-in set
/// carries is an override whose removal reverts, and an id it does not
/// carry was added by the file, so its removal is the destructive one.
/// A pane that classified these itself would drift from the writer the
/// first time the roster moved.
pub fn list_states(home: &Path) -> Vec<ListState> {
    let (config_path, routing_path, _) = paths(home);
    let compiled_in = compiled_in_providers();
    let compiled_in_shapes = compiled_in_workloads();
    let mut out: Vec<ListState> = Vec::new();
    // A file too broken to parse reads as empty, the way `states` reads
    // one: the roster load has already failed loud on it, and a pane
    // that guessed at a broken file would be a second opinion nobody
    // asked for.
    let config = read_json(&config_path).unwrap_or_else(|_| Value::Object(Map::new()));
    if let Some(providers) = config.get("providers").and_then(|list| list.as_array()) {
        for entry in providers {
            let Some(id) = entry.get("id").and_then(|id| id.as_str()) else {
                continue;
            };
            out.push(ListState {
                kind: ListKind::Provider,
                key: id.to_string(),
                detail: provider_detail(entry),
                reverts: compiled_in.iter().any(|known| known == id),
                // The entry's own JSON, compact: the buffer the writer's
                // `set_provider` parses, so an edit is a change to that
                // entry rather than a second pane-local writer.
                seed: entry.to_string(),
            });
        }
    }
    let routing = read_json(&routing_path).unwrap_or_else(|_| Value::Object(Map::new()));
    if let Some(workloads) = routing.get("workloads").and_then(|list| list.as_array()) {
        for entry in workloads {
            let Some(name) = entry.get("workload").and_then(|name| name.as_str()) else {
                continue;
            };
            let candidates: Vec<String> = entry
                .get("candidates")
                .and_then(|list| list.as_array())
                .map(|list| {
                    list.iter()
                        .filter_map(|candidate| candidate.as_str())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let detail = if candidates.is_empty() {
                "no candidates — the loader would walk this workload into its default".to_string()
            } else {
                candidates.join(" → ")
            };
            out.push(ListState {
                kind: ListKind::Workload,
                key: name.to_string(),
                detail,
                reverts: compiled_in_shapes.iter().any(|known| known == name),
                seed: candidates.join(","),
            });
        }
    }
    out
}

/// A provider card's one line: the base_url and the key's env var name
/// the entry carries, with the compiled-in default named wherever the
/// entry does not override it, and its enabled state when the entry
/// names one — an override that disables a provider is the most common
/// edit there is, so the card says so. A key's VALUE is never rendered:
/// the env var's name is configuration, the value behind it is a secret.
fn provider_detail(entry: &Value) -> String {
    let base = entry.get("base_url").and_then(|value| value.as_str());
    let key_env = entry.get("key_env").and_then(|value| value.as_str());
    let mut parts = vec![
        base.unwrap_or("the compiled-in base_url").to_string(),
        key_env.unwrap_or("the compiled-in key env").to_string(),
    ];
    if let Some(enabled) = entry.get("enabled").and_then(|value| value.as_bool()) {
        parts.push(if enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        });
    }
    parts.join(" · ")
}

/// Add or replace one provider override. The entry validates through
/// the roster's own schema — the same `deny_unknown_fields` struct
/// `load_roster` reads — so a proposal cannot introduce a field the
/// roster would refuse, and a misspelled key fails here, by name,
/// before anything is written. An entry that already exists is
/// replaced in place, preserving the file's order.
pub fn set_provider(home: &Path, entry: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(entry)
        .map_err(|error| format!("the entry is not valid JSON: {error}"))?;
    // The loader's own validator: a misspelled field is a named refusal
    // rather than a silently ignored key.
    let parsed = crate::route::validate_provider_entry(&value)?;
    let id = parsed.id().to_string();
    if id.is_empty() {
        return Err("the entry names no provider id".to_string());
    }
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let providers = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .entry("providers")
        .or_insert_with(|| Value::Array(Vec::new()));
    let array = providers
        .as_array_mut()
        .ok_or_else(|| "`providers` is not a list".to_string())?;
    // Replace in place when the id is already there, so a re-configuration
    // does not reorder the file underneath its owner.
    let position = array
        .iter()
        .position(|existing| existing.get("id").and_then(|id| id.as_str()) == Some(id.as_str()));
    match position {
        Some(index) => array[index] = value,
        None => array.push(value),
    }
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )?;
    Ok(id)
}

/// Remove one provider override. The destructive rule, and the reason
/// this is not a bare delete: an id the compiled-in roster carries is
/// an override, and removing it reverts the provider to its compiled-in
/// default. An id the roster does NOT carry was added by this file, and
/// removing it removes the provider from the roster entirely — which is
/// the edit that can break every walk that names it, so it is refused
/// unless the owner says `force`.
pub fn remove_provider(home: &Path, id: &str, force: bool) -> Result<(), String> {
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let providers = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .get_mut("providers")
        .and_then(|providers| providers.as_array_mut())
        .ok_or_else(|| "`providers` is not a list".to_string())?;
    let position = providers
        .iter()
        .position(|existing| existing.get("id").and_then(|id| id.as_str()) == Some(id));
    let Some(index) = position else {
        return Err(format!(
            "`{id}` has no override in config.json; the compiled-in roster carries {}",
            compiled_in_providers().join(", ")
        ));
    };
    if !compiled_in_providers().iter().any(|known| known == id) && !force {
        return Err(format!(
            "`{id}` is not in the compiled-in roster, so removing its override removes the provider from every walk that names it; pass --force to do it anyway"
        ));
    }
    providers.remove(index);
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
}

/// Write one workload's preference list. Each candidate must be a
/// `provider/model` pair — the same rule `load_routing` enforces, so a
/// typo fails here rather than at the next startup. An empty list is
/// refused: a workload with no candidates is a workload nothing can
/// serve, and the loader would walk it into the fallback.
pub fn set_workload(home: &Path, workload: &str, candidates: &[String]) -> Result<(), String> {
    if candidates.is_empty() {
        return Err(format!(
            "`{workload}` needs at least one `provider/model` candidate"
        ));
    }
    for candidate in candidates {
        if !candidate.contains('/') {
            return Err(format!(
                "`{candidate}` is not a `provider/model` pair, the shape routing.json reads"
            ));
        }
    }
    let path = home.join("routing.json");
    let mut document = read_json(&path)?;
    let workloads = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .entry("workloads")
        .or_insert_with(|| Value::Array(Vec::new()));
    let array = workloads
        .as_array_mut()
        .ok_or_else(|| "`workloads` is not a list".to_string())?;
    let entry = serde_json::json!({
        "workload": workload,
        "candidates": candidates,
    });
    let position = array
        .iter()
        .position(|existing| existing.get("workload").and_then(|w| w.as_str()) == Some(workload));
    match position {
        Some(index) => array[index] = entry,
        None => array.push(entry),
    }
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
}

/// Remove one workload's preference. The same destructive rule as the
/// providers: a compiled-in workload's override reverts to the
/// compiled-in table, and a workload this file added is deleted — which
/// the loader would then walk into its default, so it needs `force`.
pub fn remove_workload(home: &Path, workload: &str, force: bool) -> Result<(), String> {
    let path = home.join("routing.json");
    let mut document = read_json(&path)?;
    let workloads = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .get_mut("workloads")
        .and_then(|workloads| workloads.as_array_mut())
        .ok_or_else(|| "`workloads` is not a list".to_string())?;
    let position = workloads
        .iter()
        .position(|existing| existing.get("workload").and_then(|w| w.as_str()) == Some(workload));
    let Some(index) = position else {
        return Err(format!(
            "`{workload}` has no preference in routing.json; the compiled-in table carries {}",
            compiled_in_workloads().join(", ")
        ));
    };
    if !compiled_in_workloads()
        .iter()
        .any(|known| known == workload)
        && !force
    {
        return Err(format!(
            "`{workload}` is not in the compiled-in table, so removing its preference leaves the workload with no candidates at all; pass --force to do it anyway"
        ));
    }
    workloads.remove(index);
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
}
