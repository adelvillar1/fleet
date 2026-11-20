//! The served control endpoint: `fleet armada mcp` (D59, 2026-11-19).
//!
//! fleet has always been an MCP *client* — `src/mcp.rs` spawns registered
//! servers and calls their tools. This is the other direction: fleet serves
//! the armada control vocabulary over stdio so an agent can command the
//! fleet, which is the agent-first pattern the armada direction doc borrows
//! from muxin (§10.1: every capability is a structured JSON tool over MCP,
//! *and* a CLI command, *and* a GUI path).
//!
//! Two rules make it safe, and they are the whole design:
//!
//! 1. **Every call goes through the control plane's admit.** The endpoint
//!    does not re-check authority, and it does not have its own idea of what
//!    a verb may do — it resolves the verb and admits it exactly as the relay
//!    path does. One guard, one code path, no second permission system.
//! 2. **An unknown or ungranted verb is a NAMED refusal**, never a silent
//!    empty result. A tool that quietly does nothing is how an agent learns
//!    the wrong lesson about what it is allowed to do.
//!
//! The transport is stdio with line-delimited JSON-RPC 2.0 — the MCP
//! convention every client speaks.

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};

use crate::armada::control::{self, Verb};

/// The tool prefix a client sees. `fleet mcp` can register this endpoint as
/// a server and address its tools as `mcp__fleet-armada__<tool>`.
pub const SERVER_NAME: &str = "fleet-armada";

/// One served tool: the verb it drives and how it reads to an agent.
struct ToolVerb {
    name: &'static str,
    verb: Verb,
    description: &'static str,
}

/// The served vocabulary. D46's verb table IS this list — a new control verb
/// appears here and nowhere else in the endpoint.
const SERVED: &[ToolVerb] = &[
    ToolVerb {
        name: "roster_read",
        verb: Verb::State,
        description:
            "the flagship's roster: instances, liveness, capacity, and settled reliability",
    },
    ToolVerb {
        name: "instance_state",
        verb: Verb::State,
        description: "one instance's state, by sid",
    },
    ToolVerb {
        name: "instance_dispatch",
        verb: Verb::Dispatch,
        description: "dispatch a task to one instance — the instance admits for itself",
    },
    ToolVerb {
        name: "corpus_sync",
        verb: Verb::CorpusSync,
        description: "sync the shared corpus over its git remote",
    },
    ToolVerb {
        name: "health_read",
        verb: Verb::Health,
        description: "the health probe: is this install well?",
    },
    ToolVerb {
        name: "deploy",
        verb: Verb::Register,
        description: "provision a fresh instance (destructive class: demands fresh human proof)",
    },
    ToolVerb {
        name: "version_roll",
        verb: Verb::VersionRoll,
        description: "move an instance to a version (destructive class: demands fresh proof)",
    },
    ToolVerb {
        name: "retire",
        verb: Verb::Retire,
        description: "retire an instance (destructive class: demands fresh proof)",
    },
    ToolVerb {
        name: "register",
        verb: Verb::Register,
        description: "register an instance on the roster, the join's own act",
    },
    ToolVerb {
        name: "heartbeat",
        verb: Verb::Heartbeat,
        description: "record an instance's heartbeat, which is what earns liveness",
    },
];

/// The `tools/list` payload: every served verb with its input schema.
pub fn tool_list() -> Value {
    let tools: Vec<Value> = SERVED
        .iter()
        .map(|served| {
            json!({
                "name": served.name,
                "description": format!(
                    "{} (control verb `{}`, {} class)",
                    served.description,
                    served.verb.as_str(),
                    served.verb.class().as_str()
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sid": {"type": "string", "description": "the instance's sid"},
                        "task": {"type": "string", "description": "the task, for a dispatch"},
                        "target": {"type": "string", "description": "the target, for a destructive verb"},
                        "principal": {
                            "type": "string",
                            "description": "the principal this call acts for; defaults to the owner"
                        }
                    }
                }
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Find a served tool by the name an agent called.
fn verb_for(name: &str) -> Result<&'static ToolVerb, String> {
    SERVED
        .iter()
        .find(|served| served.name == name)
        .ok_or_else(|| {
            format!(
                "no tool `{name}` here — this endpoint serves the control vocabulary ({})",
                SERVED
                    .iter()
                    .map(|served| served.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Run one tool call through the guard. This is the endpoint's only path to
/// an effect, and it does NOT short-circuit reads: a `state` call still
/// resolves authority and the guard, because an endpoint that skipped the
/// guard for "harmless" verbs would be the confused deputy D32 exists to
/// prevent.
pub fn call(home: &Path, name: &str, arguments: &Value, now_secs: u64) -> Result<Value, String> {
    let served = verb_for(name)?;
    let principal = arguments
        .get("principal")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(crate::identity::OWNER);
    let sid = arguments
        .get("sid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let target = arguments
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // The guard's own reading of this call, before anything runs.
    let admitted = control::admit_local(home, served.verb, principal, target, sid, now_secs)?;
    let outcome = control::execute(home, &admitted)?;
    Ok(json!({
        "tool": name,
        "verb": served.verb.as_str(),
        "class": served.verb.class().as_str(),
        "principal": principal,
        "outcome": outcome.as_str(),
    }))
}

/// Serve the endpoint over stdio until the client closes it.
///
/// The loop is deliberately plain: one JSON object per line in, one out. A
/// parse failure is a JSON-RPC error rather than a dropped line, because a
/// client that gets silence cannot tell a broken request from a broken
/// server.
pub fn serve_stdio(home: &Path) -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": Value::Null,
                           "error": {"code": -32700,
                                     "message": format!("the request is not JSON: {error}")}})
                )
                .map_err(|error| error.to_string())?;
                continue;
            }
        };
        if let Some(reply) = respond(home, &request) {
            writeln!(out, "{reply}").map_err(|error| error.to_string())?;
            out.flush().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Answer one JSON-RPC request. A notification (no `id`) answers `None`.
pub fn respond(home: &Path, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let answer = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION")},
        })),
        "tools/list" => Ok(tool_list()),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            call(home, &name, &arguments, now)
                .map(|result| json!({"content": [{"type": "text", "text": result.to_string()}]}))
        }
        "notifications/initialized" | "notifications/cancelled" => return None,
        other => Err(format!(
            "no method `{other}` here — this endpoint serves initialize, tools/list and tools/call"
        )),
    };
    let id = id?;
    Some(match answer {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(refusal) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": refusal}})
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fleet-amcp-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_tool_list_is_the_control_vocabulary() {
        let listed = tool_list();
        let names: Vec<&str> = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        for expected in [
            "roster_read",
            "instance_state",
            "instance_dispatch",
            "corpus_sync",
            "health_read",
            "deploy",
            "version_roll",
            "retire",
            "register",
            "heartbeat",
        ] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
        assert_eq!(
            names.len(),
            10,
            "the endpoint serves the ten verbs: {names:?}"
        );
    }

    #[test]
    fn an_unknown_tool_is_refused_by_name() {
        let home = scratch("unknown");
        let refused = call(&home, "sudo_everything", &json!({}), 1).expect_err("refused");
        assert!(refused.contains("no tool `sudo_everything`"), "{refused}");
        assert!(
            refused.contains("roster_read"),
            "the refusal lists what exists: {refused}"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn initialize_and_tools_list_answer_over_the_wire() {
        let home = scratch("wire");
        let hello = respond(
            &home,
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        )
        .expect("initialize answers");
        assert_eq!(hello["result"]["serverInfo"]["name"], SERVER_NAME);
        let listed = respond(
            &home,
            &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        )
        .expect("tools/list answers");
        assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 10);
        assert!(respond(
            &home,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        )
        .is_none());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_read_verb_runs_through_the_guard_and_answers() {
        let home = scratch("state");
        let called = respond(
            &home,
            &json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "roster_read", "arguments": {}}}),
        )
        .expect("answers");
        let text = called["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"verb\":\"state\""), "{text}");
        assert!(text.contains("\"principal\":\"owner\""), "{text}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_unknown_method_is_named_not_silent() {
        let home = scratch("method");
        let refused = respond(
            &home,
            &json!({"jsonrpc": "2.0", "id": 4, "method": "resources/list", "params": {}}),
        )
        .expect("answers");
        let message = refused["error"]["message"].as_str().unwrap();
        assert!(message.contains("no method `resources/list`"), "{message}");
        std::fs::remove_dir_all(&home).ok();
    }
}
