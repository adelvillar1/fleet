//! The agent loop: task in, model turns and approved tool calls out, until
//! the model answers without tools or the turn cap is reached.
//!
//! Every message is recorded to the session log as it happens, so the log
//! always contains exactly what the model saw.

use crate::api::{ChatRequest, Client, ClientError, Message, Role, ToolCall, Usage};
use crate::session::{Record, Session};
use crate::tools::{Tool, ALL_TOOLS};
use std::io::Write;

/// System prompt sent with every request.
pub const SYSTEM_PROMPT: &str = "You are Fleet, a native Rust coding agent based on the DeepSeek Harness design. Work in the user's current directory. Use the provided tools to inspect and change files, run commands, and verify your work. Read before you write. When a task is done, reply with a short summary of the outcome; if part of it failed, say exactly what failed and what you tried.";

/// Turn cap when the caller does not choose one.
pub const DEFAULT_MAX_TURNS: usize = 25;

/// Options for one run.
#[derive(Debug, Clone)]
pub struct AgentOptions {
    pub model: String,
    pub max_turns: usize,
}

impl Default for AgentOptions {
    fn default() -> AgentOptions {
        AgentOptions { model: crate::api::DEFAULT_MODEL.to_string(), max_turns: DEFAULT_MAX_TURNS }
    }
}

/// Why the loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The model answered without requesting tools.
    Completed,
    /// The turn cap was reached first.
    MaxTurns,
}

/// What one run produced.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub stop_reason: StopReason,
    pub turns: u64,
    pub usage: Usage,
    /// The final assistant text, when the stop produced one.
    pub final_message: Option<String>,
}

/// The loop's fatal failures. Tool failures are not fatal: they are recorded
/// and shown to the model.
#[derive(Debug)]
pub enum AgentError {
    Api(ClientError),
    Io(std::io::Error),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Api(error) => write!(f, "{error}"),
            AgentError::Io(error) => write!(f, "session log write failed: {error}"),
        }
    }
}

impl std::error::Error for AgentError {}

/// Decides whether one tool call may execute.
pub trait Approver {
    /// True when the call may run.
    fn approve(&mut self, call: &ToolCall) -> bool;
}

/// Approves everything (`--yes`).
#[derive(Debug, Default)]
pub struct ApproveAll;

impl Approver for ApproveAll {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        true
    }
}

/// Denies everything; tests and non-interactive dry runs.
#[derive(Debug, Default)]
pub struct DenyAll;

impl Approver for DenyAll {
    fn approve(&mut self, _call: &ToolCall) -> bool {
        false
    }
}

/// Prompts on the console; `y`/`yes` allows one call, `a`/`always` latches
/// approval for the rest of the run, anything else denies. End of input
/// denies.
pub struct ConsoleApprover {
    input: std::io::Stdin,
    always: bool,
}

impl ConsoleApprover {
    /// A fresh prompter that never allows until asked.
    pub fn new() -> ConsoleApprover {
        ConsoleApprover { input: std::io::stdin(), always: false }
    }
}

impl Default for ConsoleApprover {
    fn default() -> ConsoleApprover {
        ConsoleApprover::new()
    }
}

impl Approver for ConsoleApprover {
    fn approve(&mut self, call: &ToolCall) -> bool {
        if self.always {
            return true;
        }
        let tool = Tool::from_name(&call.function.name);
        let summary = tool
            .map(|tool| tool.summarize(&parse_arguments(&call.function.arguments), 72))
            .unwrap_or_else(|| call.function.name.clone());
        eprint!("fleet: allow {} `{summary}`? [y/N/a] ", call.function.name);
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if self.input.read_line(&mut line).unwrap_or(0) == 0 {
            eprintln!();
            return false;
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => true,
            "a" | "always" => {
                self.always = true;
                true
            }
            _ => false,
        }
    }
}

/// Runs tasks against the API with the built-in tools.
pub struct Agent<'a> {
    client: &'a Client,
    options: AgentOptions,
    session: &'a mut Session,
    messages: Vec<Message>,
    approver: Box<dyn Approver + 'a>,
    progress: &'a mut dyn Write,
    usage: Usage,
    turns: u64,
}

impl<'a> Agent<'a> {
    /// An agent over `session`; an existing log's records seed the
    /// conversation so resumed runs continue in context.
    pub fn new(
        client: &'a Client,
        options: AgentOptions,
        session: &'a mut Session,
        approver: Box<dyn Approver + 'a>,
        progress: &'a mut dyn Write,
    ) -> Result<Agent<'a>, AgentError> {
        let records = crate::session::replay(session.path()).map_err(AgentError::Io)?;
        Ok(Agent {
            client,
            options,
            session,
            messages: crate::session::messages(&records),
            approver,
            progress,
            usage: Usage::default(),
            turns: 0,
        })
    }

    /// Token accounting accumulated across the run's requests.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// Run `task` to a stop. The session log receives every message and the
    /// `end` record; an API error leaves the log without `end`, which is the
    /// documented interrupted-log shape.
    pub fn run(&mut self, task: &str) -> Result<RunReport, AgentError> {
        self.push_message(Message::text(Role::User, task))?;
        while self.turns < self.options.max_turns as u64 {
            self.turns += 1;
            let _ = writeln!(self.progress, "\u{2014} turn {}", self.turns);
            let _ = self.progress.flush();

            // The system prompt is a compiled-in constant, like the tool
            // declarations: model-visible but not session-specific, so it is
            // reconstructed per request rather than logged.
            let mut request_messages = Vec::with_capacity(self.messages.len() + 1);
            request_messages.push(Message::text(Role::System, SYSTEM_PROMPT));
            request_messages.extend(self.messages.iter().cloned());
            let request = ChatRequest {
                model: &self.options.model,
                messages: &request_messages,
                tools: ALL_TOOLS.iter().map(|tool| tool.spec()).collect(),
            };
            let response = self.client.chat(&request).map_err(AgentError::Api)?;
            self.usage.prompt_tokens += response.usage.prompt_tokens;
            self.usage.completion_tokens += response.usage.completion_tokens;
            let choice = response.choices.into_iter().next().ok_or_else(|| {
                AgentError::Api(ClientError::Decode("response had no choices".to_string()))
            })?;

            let mut message = choice.message;
            for call in &message.tool_calls {
                let _ = writeln!(self.progress, "\u{2192} {}", describe_call(call));
            }
            let _ = self.progress.flush();

            let tool_calls = std::mem::take(&mut message.tool_calls);
            if tool_calls.is_empty() {
                let content = message.content.clone().unwrap_or_default();
                self.push_message(message)?;
                let turns = self.turns;
                self.record(&Record::End {
                    stop_reason: "completed".to_string(),
                    turns,
                })?;
                return Ok(RunReport {
                    stop_reason: StopReason::Completed,
                    turns,
                    usage: self.usage,
                    final_message: Some(content),
                });
            }

            self.push_message(message)?;
            for call in tool_calls {
                self.execute_call(&call)?;
            }
        }

        let turns = self.turns;
        self.record(&Record::End { stop_reason: "max-turns".to_string(), turns })?;
        Ok(RunReport {
            stop_reason: StopReason::MaxTurns,
            turns,
            usage: self.usage,
            final_message: None,
        })
    }

    /// Execute one call behind the approval gate and record what the model
    /// will see. Unknown tools and malformed arguments are model-visible
    /// error results, so the model can correct itself.
    fn execute_call(&mut self, call: &ToolCall) -> Result<(), AgentError> {
        let approved = self.approver.approve(call);
        let tool = Tool::from_name(&call.function.name);
        let (output, truncated) = if !approved {
            ("denied: the user did not approve this call.".to_string(), false)
        } else {
            match tool {
                None => (
                    format!("error: unknown tool `{}`", call.function.name),
                    false,
                ),
                Some(tool) => match serde_json::from_str(&call.function.arguments) {
                    Err(error) => (format!("error: arguments were not valid JSON: {error}"), false),
                    Ok(arguments) => match tool.execute(&arguments) {
                        Ok(outcome) => (outcome.output, outcome.truncated),
                        Err(message) => (format!("error: {message}"), false),
                    },
                },
            }
        };
        let _ = writeln!(
            self.progress,
            "\u{2190} {}",
            if approved { preview(&output, 80) } else { "denied".to_string() },
        );
        let _ = self.progress.flush();
        self.record(&Record::ToolResult {
            call_id: call.id.clone(),
            name: call.function.name.clone(),
            output: output.clone(),
            truncated,
            approved,
        })?;
        self.messages.push(Message::tool_result(call.id.clone(), output));
        Ok(())
    }

    fn push_message(&mut self, message: Message) -> Result<(), AgentError> {
        let record = match message.role {
            Role::User => Record::User {
                content: message.content.clone().unwrap_or_default(),
            },
            Role::Assistant => Record::Assistant {
                content: message.content.clone(),
                tool_calls: message.tool_calls.clone(),
            },
            Role::Tool => Record::ToolResult {
                call_id: message.tool_call_id.clone().unwrap_or_default(),
                name: String::new(),
                output: message.content.clone().unwrap_or_default(),
                truncated: false,
                approved: true,
            },
            Role::System => return Ok(()),
        };
        self.record(&record)?;
        self.messages.push(message);
        Ok(())
    }

    fn record(&mut self, record: &Record) -> Result<(), AgentError> {
        self.session.record(record).map_err(AgentError::Io)
    }
}

/// Parse a call's arguments for display only.
fn parse_arguments(arguments: &str) -> serde_json::Value {
    serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null)
}

/// `tool: summary` for a progress line.
fn describe_call(call: &ToolCall) -> String {
    let tool = Tool::from_name(&call.function.name);
    let arguments = parse_arguments(&call.function.arguments);
    match tool {
        Some(tool) => format!("{}: {}", tool.name(), tool.summarize(&arguments, 72)),
        None => format!("{}: (unknown tool)", call.function.name),
    }
}

/// First `limit` characters of `text`, first line only, for progress output.
fn preview(text: &str, limit: usize) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let mut preview: String = first_line.chars().take(limit).collect();
    if first_line.chars().count() > limit {
        preview.push('\u{2026}');
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FunctionCall;

    fn call(id: &str, name: &str, arguments: &str) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            kind: "function".to_string(),
            function: FunctionCall { name: name.to_string(), arguments: arguments.to_string() },
        }
    }

    #[test]
    fn preview_keeps_one_line_and_a_bound() {
        assert_eq!(preview("a\nb", 10), "a");
        assert_eq!(preview("x".repeat(30).as_str(), 10), format!("{}{}", "x".repeat(10), '\u{2026}'));
    }

    #[test]
    fn describe_call_handles_unknown_tools() {
        let text = describe_call(&call("1", "launch_missiles", "{}"));
        assert!(text.contains("unknown tool"));
    }
}
