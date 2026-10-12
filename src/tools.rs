//! The built-in tool set: `bash`, `read_file`, `write_file`, `list_dir`.
//!
//! Every tool takes and returns plain values: a parsed JSON arguments object
//! in, a bounded UTF-8 `ToolOutcome` out. Tool failures are values the model
//! can read and react to, not control-flow errors.

use crate::api::ToolSpec;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Hard cap on the output text of any single tool execution.
pub const MAX_TOOL_OUTPUT: usize = 64 * 1024;

/// `bash` wall-clock cap before the child is killed.
pub const BASH_TIMEOUT: Duration = Duration::from_secs(120);

/// `list_dir` entry cap.
pub const MAX_DIR_ENTRIES: usize = 1_000;

/// The closed tool set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Bash,
    ReadFile,
    WriteFile,
    ListDir,
}

/// All tools, in declaration order.
pub const ALL_TOOLS: [Tool; 4] = [Tool::Bash, Tool::ReadFile, Tool::WriteFile, Tool::ListDir];

impl Tool {
    /// The tool named `name`, or `None` for anything else.
    pub fn from_name(name: &str) -> Option<Tool> {
        ALL_TOOLS.into_iter().find(|tool| tool.name() == name)
    }

    /// The API-visible name.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Bash => "bash",
            Tool::ReadFile => "read_file",
            Tool::WriteFile => "write_file",
            Tool::ListDir => "list_dir",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Tool::Bash => "Run a shell command and return its combined output. Working directory is the user's current directory.",
            Tool::ReadFile => "Read a UTF-8 text file. Fails for binary files; use bash for those.",
            Tool::WriteFile => "Write a UTF-8 text file, replacing its contents. The parent directory must already exist.",
            Tool::ListDir => "List a directory's entries, directories marked with a trailing slash.",
        }
    }

    /// JSON Schema for this tool's arguments object.
    pub fn parameters(self) -> serde_json::Value {
        match self {
            Tool::Bash => serde_json::json!({
                "type": "object",
                "properties": { "command": { "type": "string" } },
                "required": ["command"],
            }),
            Tool::ReadFile => serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"],
            }),
            Tool::WriteFile => serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" },
                },
                "required": ["path", "content"],
            }),
            Tool::ListDir => serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"],
            }),
        }
    }

    /// The request declaration for this tool.
    pub fn spec(self) -> ToolSpec {
        ToolSpec::function(self.name(), self.description(), self.parameters())
    }

    /// One-line summary of the call for progress output and prompts.
    pub fn summarize(self, arguments: &serde_json::Value, limit: usize) -> String {
        let key = match self {
            Tool::Bash => "command",
            Tool::ReadFile | Tool::WriteFile | Tool::ListDir => "path",
        };
        let value = arguments
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let mut summary: String = value.chars().take(limit).collect();
        if value.chars().count() > limit {
            summary.push('\u{2026}');
        }
        summary
    }

    /// Execute the tool with a parsed arguments object. `Ok` carries the
    /// bounded outcome; `Err` is a user-readable argument failure the agent
    /// relays to the model verbatim.
    pub fn execute(self, arguments: &serde_json::Value) -> Result<ToolOutcome, String> {
        let outcome = match self {
            Tool::Bash => run_bash(&string_arg(arguments, "command")?)?,
            Tool::ReadFile => run_read_file(&string_arg(arguments, "path")?)?,
            Tool::WriteFile => run_write_file(arguments)?,
            Tool::ListDir => run_list_dir(&string_arg(arguments, "path")?)?,
        };
        Ok(bound(outcome))
    }
}

fn string_arg(arguments: &serde_json::Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("the `{key}` argument is required and must be a string"))
}

/// What a tool produced, before and after bounding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutcome {
    pub output: String,
    pub truncated: bool,
}

/// Cap an outcome at `MAX_TOOL_OUTPUT` bytes on a char boundary, marking the
/// cut so the model knows the text is partial.
fn bound(outcome: ToolOutcome) -> ToolOutcome {
    if outcome.output.len() <= MAX_TOOL_OUTPUT {
        return outcome;
    }
    let mut end = MAX_TOOL_OUTPUT;
    while !outcome.output.is_char_boundary(end) {
        end -= 1;
    }
    ToolOutcome {
        output: format!(
            "{}\n\u{2026}[truncated {} of {} bytes]",
            &outcome.output[..end],
            outcome.output.len() - end,
            outcome.output.len()
        ),
        truncated: true,
    }
}

/// Run `command` through the platform shell and collect its output.
///
/// Stdout and stderr are drained on their own threads from the start so a
/// child writing past the pipe buffer cannot deadlock the wait; the wait
/// itself polls so the deadline can kill a hung child.
fn run_bash(command: &str) -> Result<ToolOutcome, String> {
    let mut child = shell_command(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not spawn the shell: {error}"))?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || drain(&mut stdout_pipe));
    let stderr_reader = std::thread::spawn(move || drain(&mut stderr_pipe));

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= BASH_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    let stdout = stdout_reader.join().unwrap_or_default();
                    let stderr = stderr_reader.join().unwrap_or_default();
                    return Ok(ToolOutcome {
                        output: format!(
                            "timed out after {}s\n--- stdout ---\n{}--- stderr ---\n{}",
                            BASH_TIMEOUT.as_secs(),
                            stdout,
                            stderr
                        ),
                        truncated: false,
                    });
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(format!("waiting for the command failed: {error}")),
        }
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    let code = status.code().unwrap_or(-1);
    let mut output = format!("exit {code}\n");
    if !stdout.is_empty() {
        output.push_str("--- stdout ---\n");
        output.push_str(&stdout);
    }
    if !stderr.is_empty() {
        output.push_str("--- stderr ---\n");
        output.push_str(&stderr);
    }
    if stdout.is_empty() && stderr.is_empty() {
        output.push_str("(no output)\n");
    }
    Ok(ToolOutcome { output, truncated: false })
}

/// The platform shell invocation for `command`.
fn shell_command(command: &str) -> Command {
    #[cfg(unix)]
    {
        let mut shell = Command::new("sh");
        shell.arg("-c").arg(command);
        shell
    }
    #[cfg(windows)]
    {
        let mut shell = Command::new("cmd");
        shell.arg("/C").arg(command);
        shell
    }
}

/// Read a pipe to the end, lossily decoding UTF-8.
fn drain<R: Read>(pipe: &mut Option<R>) -> String {
    let mut bytes = Vec::new();
    if let Some(pipe) = pipe {
        let _ = pipe.read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn run_read_file(path: &str) -> Result<ToolOutcome, String> {
    let bytes = fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| format!("{path}: not valid UTF-8; use bash to inspect binary files"))?;
    Ok(ToolOutcome { output: text, truncated: false })
}

fn run_write_file(arguments: &serde_json::Value) -> Result<ToolOutcome, String> {
    let path = string_arg(arguments, "path")?;
    let content = string_arg(arguments, "content")?;
    let target = Path::new(&path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(format!("{path}: parent directory does not exist; create it with bash first"));
        }
    }
    let length = content.len();
    fs::write(target, content).map_err(|error| format!("{path}: {error}"))?;
    Ok(ToolOutcome { output: format!("wrote {length} bytes to {path}\n"), truncated: false })
}

fn run_list_dir(path: &str) -> Result<ToolOutcome, String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .map_err(|error| format!("{path}: {error}"))?
        .map(|entry| {
            let entry = entry.map_err(|error| format!("{path}: {error}"))?;
            let mut name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                name.push('/');
            }
            Ok(name)
        })
        .collect::<Result<_, String>>()?;
    names.sort();
    let total = names.len();
    names.truncate(MAX_DIR_ENTRIES);
    let mut output = names.join("\n");
    if total > MAX_DIR_ENTRIES {
        output.push_str(&format!("\n\u{2026}[{total} entries, showing {}]", MAX_DIR_ENTRIES));
    }
    output.push('\n');
    Ok(ToolOutcome {
        truncated: total > MAX_DIR_ENTRIES,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for tool in ALL_TOOLS {
            assert_eq!(Tool::from_name(tool.name()), Some(tool));
        }
        assert_eq!(Tool::from_name("rm_rf"), None);
    }

    #[test]
    fn bound_truncates_on_a_char_boundary() {
        let text = "\u{4e2d}".repeat(50_000);
        let outcome = bound(ToolOutcome { output: text, truncated: false });
        assert!(outcome.truncated);
        assert!(outcome.output.len() <= MAX_TOOL_OUTPUT + 64);
        assert!(outcome.output.contains("[truncated"));
    }

    #[test]
    fn bound_leaves_small_output_alone() {
        let outcome = bound(ToolOutcome { output: "tiny".into(), truncated: false });
        assert!(!outcome.truncated);
        assert_eq!(outcome.output, "tiny");
    }

    #[test]
    fn bash_captures_exit_code_and_streams() {
        let outcome = Tool::Bash
            .execute(&serde_json::json!({ "command": "echo out; echo err 1>&2; exit 3" }))
            .unwrap();
        assert!(outcome.output.starts_with("exit 3\n"));
        assert!(outcome.output.contains("out"));
        assert!(outcome.output.contains("err"));
    }

    #[cfg(unix)]
    #[test]
    fn bash_reports_missing_commands_as_exit_127() {
        let outcome = Tool::Bash
            .execute(&serde_json::json!({ "command": "definitely-not-a-real-command-xyz" }))
            .unwrap();
        assert!(outcome.output.starts_with("exit 127"));
    }

    #[test]
    fn bash_requires_a_string_command() {
        let error = Tool::Bash.execute(&serde_json::json!({ "command": 7 })).unwrap_err();
        assert!(error.contains("command"));
    }

    #[test]
    fn write_and_read_file_round_trip_with_parent_enforced() {
        let dir = std::env::temp_dir().join(format!("fleet-tools-test-{}", std::process::id()));
        let nested = dir.join("existing");
        fs::create_dir_all(&nested).unwrap();
        let path = nested.join("note.txt");

        Tool::WriteFile
            .execute(&serde_json::json!({ "path": path.display().to_string(), "content": "hello" }))
            .unwrap();
        let read = Tool::ReadFile
            .execute(&serde_json::json!({ "path": path.display().to_string() }))
            .unwrap();
        assert_eq!(read.output, "hello");

        let missing = dir.join("nope").join("file.txt");
        let error = Tool::WriteFile
            .execute(&serde_json::json!({ "path": missing.display().to_string(), "content": "x" }))
            .unwrap_err();
        assert!(error.contains("parent directory"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_file_rejects_binary() {
        let dir = std::env::temp_dir().join(format!("fleet-tools-bin-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blob.bin");
        fs::write(&path, [0xff, 0xfe, 0x00, 0x01]).unwrap();
        let error = Tool::ReadFile
            .execute(&serde_json::json!({ "path": path.display().to_string() }))
            .unwrap_err();
        assert!(error.contains("UTF-8"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_dir_marks_directories_and_sorts() {
        let dir = std::env::temp_dir().join(format!("fleet-tools-ls-{}", std::process::id()));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.txt"), "x").unwrap();
        let outcome = Tool::ListDir
            .execute(&serde_json::json!({ "path": dir.display().to_string() }))
            .unwrap();
        let lines: Vec<&str> = outcome.output.lines().collect();
        assert_eq!(lines, vec!["a.txt", "sub/"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn summarize_cuts_long_values() {
        let summary =
            Tool::Bash.summarize(&serde_json::json!({ "command": "x".repeat(200) }), 60);
        assert_eq!(summary.chars().count(), 61);
        assert!(summary.ends_with('\u{2026}'));
    }
}
