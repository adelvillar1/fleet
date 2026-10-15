//! The built-in tool set: `bash`, `read_file`, `write_file`, `list_dir`.
//!
//! Every tool takes and returns plain values: a parsed JSON arguments object
//! in, a bounded UTF-8 `ToolOutcome` out. Tool failures are values the model
//! can read and react to, not control-flow errors.

use crate::api::ToolSpec;
use crate::mcp::ToolInfo;
use serde::{Deserialize, Serialize};
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

/// The namespace a lane's tools enter the model-visible tool set under:
/// `mcp__<svc>__<tool>`.
pub const MCP_NAMESPACE: &str = "mcp__";

/// The model-visible name of the progressive-disclosure tool a run
/// declares when it carries skills: `use_skill` fetches one skill's
/// body on demand. It lives beside the lane namespace because it is
/// tool-set vocabulary, but its semantics belong to `crate::skills` —
/// the declaration is here so every tool name has one home.
pub const USE_SKILL_TOOL: &str = "use_skill";

/// The model-visible name of the memory write tool a run declares when
/// it carries a corpus: `remember` appends one fact file in the
/// frontmatter shape the loader reads back. It lives beside
/// `use_skill` for the same reason — tool-set vocabulary whose
/// semantics belong to `crate::memory`, with the name kept here so
/// every tool has one home.
pub const REMEMBER_TOOL: &str = "remember";

/// The model-visible name of the memory modify/forget tool a run
/// declares when it carries a corpus. A supersession retires a fact —
/// folding it into a replacement, or expiring it — and records the
/// justification that made it allowed.
pub const SUPERSEDE_TOOL: &str = "supersede";

/// The model-visible name of the delegation tool a run declares beside
/// the others: `subagent` runs one self-contained task in a child
/// session with its own context and a scoped tool set. Its semantics
/// belong to `crate::subagents`; the name lives here so every tool has
/// one home.
pub const SUBAGENT_TOOL: &str = "subagent";

/// The model-visible name of the self-maintenance tool a run declares
/// when it carries a home: `improve` files one proposal — a plan with
/// acceptance criteria, a diff, and an expected effect — into
/// `<FLEET_HOME>/proposals/`. It never applies anything: applying is
/// the owner's `fleet improve apply`, through the same gate, the way
/// the fork's Creator installs through Plugin Manager rather than
/// through a model tool. The name lives here so every tool has one
/// home; the semantics belong to `crate::improve`.
pub const IMPROVE_TOOL: &str = "improve";

/// The arrow a forwarded subagent call's name carries. A subagent's own
/// tool calls reach the owner's Approver through the parent's gate, and
/// the gate must be able to say whose call it is: the forwarded call is
/// named `subagent \u{2192} <tool>` so the prompt names both.
pub const SUBAGENT_ARROW: &str = "\u{2192}";

/// `subagent \u{2192} `, the prefix every forwarded child call's name
/// carries. Built from [`SUBAGENT_TOOL`] so the two can never drift.
pub fn subagent_call_prefix() -> String {
    format!("{SUBAGENT_TOOL} {SUBAGENT_ARROW} ")
}

/// The request declaration for `subagent`: one delegated task, the
/// child's admitted scope, and its turn cap.
///
/// Four things the description has to carry, because the model is the
/// only caller that can get them wrong: the goal must be
/// **self-contained** (the child sees nothing of this conversation), the
/// child runs on **this same model** (there is no cheaper path and no
/// model parameter), the admitted scope is a **declaration** the child's
/// subtree is held to (a declared tool is admitted and an undeclared one
/// is refused by name), and the brief is **composed rather than hoped
/// for** — `context`, `output`, and `approach` are the three parts that
/// make a delegation gradeable, and a brief that carries only a goal
/// gives the child nothing to diff against. `max-turns` is bounded in
/// the schema and again at parse, because a bound the schema alone
/// carries is a bound a non-conforming caller ignores.
pub fn subagent_spec() -> ToolSpec {
    ToolSpec::function(
        SUBAGENT_TOOL,
        "Spawn a subagent that runs one delegated task in its own session, with its own context and no knowledge of this conversation. The goal must be self-contained: state the whole task, every path, and every constraint, because the subagent sees nothing else. It runs on this same model, and its tool calls still ask for approval. Compose the brief rather than hoping for it: `context` is the background the subagent cannot see, `output` is the definition of done — the shape of the answer you will accept — and `approach` is your proposed solution, stated as a proposal rather than an instruction. The subagent's first message composes all four parts between explicit markers, so it cannot read the background as the task or the proposal as an order; it ends its answer with a delta block saying whether it followed your approach and, when it deviated, why (the approach was infeasible, a companion was missing and is named, or it found a better route) and whether that was forced or chosen. Omit all three and the subagent is handed the goal alone, exactly as before. The declaration IS the scope: pass `tools` to name the capabilities this subtree needs and nothing outside that list is callable — remember, supersede, lane tools, and improve are never admissible — and naming `subagent` opens a second delegation level, which the depth cap refuses at depth 2 by name. An undeclared tool is refused by name, so under-declare deliberately and let the refusal teach you what the subtree needed. It stops at `max-turns` (default 20, halved at depth 2, at most 50) and returns its final answer, its session id and log path, its turn count, and its outcome (completed, max-turns, or failed) — a failure returns the partial result and the reason, never silence. The summary is the subagent's own report, not evidence: verify any external side effect it claims — a file it wrote, a command it ran — before you report that claim as fact. Delegate a self-contained piece of work whose intermediate steps would not help you; do the work yourself when you need what you learn along the way.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "The delegated task, stated in full. The subagent sees nothing of this conversation, so every path, constraint, and definition of done belongs here.",
                },
                "tools": {
                    "type": "array",
                    "description": "The capabilities this subtree needs, which IS the subagent's admitted scope: a declared tool is admitted and an undeclared one is refused by name. Naming subagent opens a second delegation level; at depth 2 it is stripped and the call is refused citing the cap. An empty list admits no tool at all.",
                    "items": {
                        "type": "string",
                        "enum": ["bash", "read_file", "write_file", "list_dir", "use_skill", "subagent"],
                    },
                },
                "max-turns": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 50,
                    "description": "The subagent's turn cap. Defaults to 20 and is bounded at 50; a depth-2 child is held to half the default.",
                },
                "context": {
                    "type": "string",
                    "description": "The background the subagent needs and cannot see: paths already read, decisions already made, constraints discovered along the way. It rides the brief's CONTEXT section beside the capabilities the subtree was declared.",
                },
                "output": {
                    "type": "string",
                    "description": "The definition of done: the shape of the answer you will accept. It rides the brief's WHAT DONE LOOKS LIKE section.",
                },
                "approach": {
                    "type": "string",
                    "description": "Your proposed solution for this subtree, stated as a proposal rather than an instruction. The subagent reports whether it followed it, so a brief with no approach is a brief with nothing to diff against.",
                },
                "role": {
                    "type": "string",
                    "description": "The role this delegation runs under, by name — a name, not a path. The role's own overlay is loaded beside the compiled-in system prompt; the name rides the brief and the delegation row so the calibration's cells can key on it. A name that could not name a role is refused by name rather than silently defaulted.",
                },
            },
            "required": ["goal"],
        }),
    )
}

/// The request declaration for `use_skill`. The skills themselves ride
/// the system prompt as a name+description list; this declares the one
/// call that fetches a body. An unknown name is a typed error result the
/// model can read, which is why the description says so.
pub fn use_skill_spec() -> ToolSpec {
    ToolSpec::function(
        USE_SKILL_TOOL,
        "Load a skill's body into context by name. The available skills are listed in the system prompt with their descriptions; an unknown name is an error result, not a guess.",
        serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"],
        }),
    )
}

/// The request declaration for `remember`: the `brain_remember` shape,
/// mechanism over prose. `basis` is required on every write and
/// `valid-until` on every working-tier one, and the description says
/// so, because those two rules are what make the corpus's provenance
/// and lifecycle mean anything.
///
/// The description is also where the slice-19 instruction lives — the
/// slice-9 lesson: an instruction with no home where the model sees it
/// does not exist. It asks for the **phrases that resolved the
/// problem** (the trajectory's own vocabulary, which rides the fact's
/// indexed text and the trajectory channel), for the **sign** (a fact
/// tagged with a failure shape is an anti-vector, and its phrases are
/// the trajectory that diverged), and for the **third sign's outcome**
/// (a trajectory whose direction was right but whose companions were
/// absent is tagged `incomplete` and records them in `missing`, so it
/// is never mistaken for done).
pub fn remember_spec() -> ToolSpec {
    ToolSpec::function(
        REMEMBER_TOOL,
        "Write one fact into the memory corpus. A fact is one file the harness reads back on later turns, so write what a future session needs: the fact as one sentence (description), what kind it is (type), its provenance (basis), and its full content (body). basis is required on every write: confirmed-by when the user stated or verified the fact, inferred when this session merely produced it. tier is working (session-scoped; valid-until is required and the consolidation cycle compresses it) or durable (the default; no expiry). shape takes comma-separated tags from the structure shapes (rendering, state-sync, deploy, build, change, deps, docs), the failure shapes (fail-open, drops-the-row, bound-exceeded, claim-outran-evidence, fixed-width-clip, foreign-root), and the outcome tags (incomplete, resolved), at most three in total. phrases records the resolution: the exact vocabulary that drove toward the answer — the commands, file names, and words that worked — so a later recall for this shape matches them even when the question is worded differently. When what you are recording is a FAILURE, tag it with a failure shape and record in phrases the trajectory that was tried and diverged: recall will surface it as an anti-vector to avoid, marked, never as a candidate solution. When what you are recording was INCOMPLETE — the direction was right but the companions it needed were absent, so it drove partway and stalled — tag it `incomplete` and name those companions in missing: recall surfaces it as `incomplete:` with them named, so a later session finishes the trajectory instead of re-deriving it or reading it as settled. When a failure shape's fix landed, tag it `resolved` so the fact reads as a candidate rather than an anti-vector.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The fact's name: lowercase letters, digits, and hyphens. It is the file name and the key every surface uses, and it must not already exist.",
                },
                "description": {
                    "type": "string",
                    "description": "The fact as one sentence: what an assembled turn carries.",
                },
                "type": {
                    "type": "string",
                    "description": "Free-form kind; user and feedback are the protected tier the budget never evicts. Defaults to project.",
                },
                "shape": {
                    "type": "string",
                    "description": "Comma-separated shapes from the structure shapes (rendering, state-sync, deploy, build, change, deps, docs), the failure shapes (fail-open, drops-the-row, bound-exceeded, claim-outran-evidence, fixed-width-clip, foreign-root), and the outcome tags (incomplete, resolved), at most three. A failure-shape tag makes the fact an anti-vector — what broke, recalled so it is avoided — unless an outcome tag overrides it: `incomplete` says the trajectory stalled with companions missing, `resolved` says the fix landed.",
                },
                "phrases": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "The phrases that resolved the problem: the vocabulary that drove toward the answer, quoted as it was used. They ride the fact's indexed text and the trajectory channel, so a later recall for this shape matches them even when the question is worded differently. For a failure, the phrases of the trajectory that was tried and diverged.",
                },
                "missing": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "The companions an INCOMPLETE trajectory was missing: the absent pieces that kept a correct direction from reaching the minima. Name them and tag the fact `incomplete` — recall surfaces them on the `incomplete:` line so a later session finishes the trajectory instead of re-deriving it. They are not the fact's direction, so they do not ride the indexed text and cannot score.",
                },
                "basis": {
                    "type": "string",
                    "enum": ["confirmed-by", "inferred"],
                    "description": "Required. confirmed-by when the user stated or verified the fact; inferred when this session merely produced it.",
                },
                "tier": {
                    "type": "string",
                    "enum": ["working", "durable"],
                    "description": "Defaults to durable. working is session-scoped and requires valid-until.",
                },
                "valid-until": {
                    "type": "string",
                    "description": "YYYY-MM-DD, the day the fact stops being true. Required for the working tier; a durable fact carries none.",
                },
                "body": {
                    "type": "string",
                    "description": "The fact's full content.",
                },
            },
            "required": ["name", "description", "basis"],
        }),
    )
}

/// The request declaration for `supersede`: the modify/forget path,
/// with the justification that is its whole provenance. The
/// description names the permission model, because the model is the
/// caller most likely to meet it.
pub fn supersede_spec() -> ToolSpec {
    ToolSpec::function(
        SUPERSEDE_TOOL,
        "Retire a fact from memory. justification is required and is recorded in the calibration row and the replacement's body. With replacement, the modify path: a new fact is written (description is required) and the old one is folded into it, staying on disk and out of recall. Without replacement, the forget path: the fact is expired with the retirement noted in its body, and the cycle archives its file. A confirmed-by fact cannot be modified or forgotten without the user's approval, which the harness asks for; an inferred one is supersedable on your justification. The replacement inherits the superseded fact's phrases, its missing companions, its type, its shape, and its valid-until when you omit them: omit `missing` to keep the companions the trajectory was missing, pass an explicit empty list to clear them because they arrived, or pass a list to state the new ones. A replacement is always inferred — a supersession is a proposal until someone confirms it.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The fact being retired.",
                },
                "justification": {
                    "type": "string",
                    "description": "Required. Why the fact is being retired.",
                },
                "replacement": {
                    "type": "string",
                    "description": "The new fact's name, for the modify path. Omit to forget.",
                },
                "description": {
                    "type": "string",
                    "description": "The replacement's one-sentence fact. Required with replacement.",
                },
                "body": {
                    "type": "string",
                    "description": "The replacement's full content; defaults to the supersession note.",
                },
                "type": {
                    "type": "string",
                    "description": "The replacement's kind; defaults to the superseded fact's own.",
                },
                "shape": {
                    "type": "string",
                    "description": "The replacement's shapes — structure, failure, or outcome; defaults to the superseded fact's own.",
                },
                "missing": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "The replacement's missing companions. Omit it to inherit the superseded fact's own; pass an explicit empty list to clear them because they arrived; pass a list to state the new ones.",
                },
                "valid-until": {
                    "type": "string",
                    "description": "The replacement's YYYY-MM-DD expiry; defaults to the superseded fact's own.",
                },
            },
            "required": ["name", "justification"],
        }),
    )
}

/// The request declaration for `improve`: file one self-maintenance
/// proposal. The tool is the fork's proposal shape — a plan with
/// acceptance criteria, a diff, and an expected effect — and it never
/// applies anything: the owner's `fleet improve apply` runs the same
/// gate the plan names, the way Creator mode installs through Plugin
/// Manager rather than through a model tool. The description says so,
/// because the model is the caller most likely to assume otherwise.
///
/// `target` is one of `skill`, `memory`, `source`, or `config`;
/// `target-name` is the skill's name, the fact's name, the repo-
/// relative source path, or the provider id. `basis` is required on
/// every proposal, exactly as it is on every memory write: `inferred`
/// when this session produced the change, `confirmed-by` when the user
/// stated or verified it. `change` is the JSON change object the gate
/// applies — its `op` is one of `write-skill`, `retire-skill`,
/// `supersede`, `forget`, `config`, `config-remove`, or `patch`.
pub fn improve_spec() -> ToolSpec {
    ToolSpec::function(
        IMPROVE_TOOL,
        "File one self-maintenance proposal: a change to the harness's own skills, memory, or config, with the justification, the acceptance criteria, and the expected effect a reviewer needs. This tool only FILES the proposal into <FLEET_HOME>/proposals/ — it never applies anything. Applying is the owner's `fleet improve apply <name>`, which runs the local gate (fmt, clippy, build, test), asks for approval when the change is destructive, and records the verdict in the calibration ledger. Use it when a session's outcome justifies a change the harness should make to itself; state the evidence in the justification.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The proposal's name: lowercase letters, digits, and hyphens. It is the file name and the key every surface uses.",
                },
                "target": {
                    "type": "string",
                    "description": "What the change addresses: skill, memory, source, or config.",
                    "enum": ["skill", "memory", "source", "config"],
                },
                "target-name": {
                    "type": "string",
                    "description": "The target's name or path: the skill's name, the fact's name, the repo-relative source path, or the provider id.",
                },
                "basis": {
                    "type": "string",
                    "description": "The proposal's provenance. Required on every proposal: inferred when this session produced the change, confirmed-by when the user stated or verified it.",
                    "enum": ["inferred", "confirmed-by"],
                },
                "justification": {
                    "type": "string",
                    "description": "Why this change is justified: the session's evidence, the failing test, or the graded outcome behind it.",
                },
                "acceptance": {
                    "type": "string",
                    "description": "The acceptance criteria: what must be true for the change to count as done.",
                },
                "effect": {
                    "type": "string",
                    "description": "The expected effect: what changes for the harness when this lands.",
                },
                "change": {
                    "type": "string",
                    "description": "The change itself as a JSON object with an `op` field: write-skill ({op, content}), retire-skill ({op}), supersede ({op, name, justification, replacement?, description?, body?, type?, shape?, valid-until?}), forget ({op, name, justification}), config ({op, entry}), config-remove ({op, id}), or patch ({op, path, content, diff?}).",
                },
            },
            "required": ["name", "target", "target-name", "basis", "justification", "acceptance", "effect", "change"],
        }),
    )
}

/// The model-visible name of one lane tool. The lane's own name stays
/// inside the namespaced one, so a replayed log and the request the model
/// saw name the same tool the lane does.
pub fn lane_tool_name(lane: &str, tool: &str) -> String {
    format!("{MCP_NAMESPACE}{lane}__{tool}")
}

/// The lane and the lane's own tool name behind a model-visible name, or
/// `None` for anything that is not a lane tool — the four built-ins
/// included.
pub fn lane_of(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix(MCP_NAMESPACE)?;
    let (lane, tool) = rest.split_once("__")?;
    (!lane.is_empty() && !tool.is_empty()).then_some((lane, tool))
}

/// One lane tool as the model sees it: the namespaced name it was
/// declared under, with the lane's own description and JSON Schema
/// carried verbatim. The same value builds the request's declarations and
/// the session log's `lane_tools` record, so what the model saw is
/// exactly what the log holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneTool {
    /// The model-visible name: `mcp__<svc>__<short>`.
    pub name: String,
    /// The lane's own name for this tool, which is what `tools/call`
    /// carries. Recorded beside the namespaced name so the log names both
    /// ends of the mapping rather than leaving it to be re-derived.
    pub lane_tool: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// One lane's contribution to a run's tool set, in the order the lane
/// joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneTools {
    pub lane: String,
    pub tools: Vec<LaneTool>,
}

/// Every declaration one request carries: the four built-ins first, in
/// their fixed order, then each joined lane's tools under the lane
/// namespace, in the order the lanes joined.
///
/// Pure over its input, which is what makes the assembly testable without
/// a lane, a child process, or a run.
pub fn declarations(lanes: &[LaneTools]) -> Vec<ToolSpec> {
    let mut specs: Vec<ToolSpec> = ALL_TOOLS.iter().map(|tool| tool.spec()).collect();
    for lane in lanes {
        for tool in &lane.tools {
            specs.push(ToolSpec::function(
                tool.name.clone(),
                tool.description.clone(),
                tool.input_schema.clone(),
            ));
        }
    }
    specs
}

/// One lane's roster as the model sees it: every `tools/list` entry moved
/// into the lane namespace, in the order the server listed them.
///
/// A lane's own tool names already carry the lane's name — `sem1_probe`,
/// `sys1_classify` — and repeating it inside the namespace would read as
/// `mcp__sem1__sem1_probe`. The namespace drops that redundant prefix, so
/// the model calls `mcp__sem1__probe`; a tool whose name does not start
/// with the lane's keeps its own name verbatim. Two tools whose short
/// names meet resolve to distinct declared names — the lane's own full
/// name first, then a numbered variant — so no two tools ever share one
/// declaration and neither is shadowed when a call looks its name up.
pub fn namespaced(lane: &str, tools: &[ToolInfo]) -> Vec<LaneTool> {
    let prefix = format!("{lane}_");
    let mut declared: Vec<String> = Vec::with_capacity(tools.len());
    tools
        .iter()
        .map(|tool| {
            let short = tool
                .name
                .strip_prefix(&prefix)
                .filter(|short| !short.is_empty())
                .unwrap_or(&tool.name);
            let name = unique_name(lane, short, &tool.name, &declared);
            declared.push(name.clone());
            LaneTool {
                name,
                lane_tool: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            }
        })
        .collect()
}

/// The model-visible name for one lane tool: the short name when it is
/// free, the lane's own full name when it is not, and a numbered variant
/// of the full name when both are taken.
///
/// The fallback used to be the full name alone. That covers the obvious
/// collision — a lane shipping `probe` and `sem1_probe`, where the second
/// falls back to `mcp__sem1__sem1_probe` — and misses the corner: a lane
/// shipping `sem1_x` and a bare `x` makes the bare tool's own full name
/// *be* the short name it collided on, so the fallback handed it the
/// first tool's declaration verbatim and `find` shadowed it for the rest
/// of the run. Unreachable with the four vendored lanes, every one of
/// which lists only prefixed tools, and one re-vendor away from
/// mattering.
fn unique_name(lane: &str, short: &str, full: &str, declared: &[String]) -> String {
    for candidate in [lane_tool_name(lane, short), lane_tool_name(lane, full)] {
        if !declared.contains(&candidate) {
            return candidate;
        }
    }
    // Both natural names are taken: number the full name until a free one
    // turns up. `declared` is finite and every candidate is distinct, so
    // this ends well before the last iteration.
    for suffix in 2..=declared.len() + 2 {
        let candidate = lane_tool_name(lane, &format!("{full}_{suffix}"));
        if !declared.contains(&candidate) {
            return candidate;
        }
    }
    lane_tool_name(lane, full)
}

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
///
/// The one cap every model-visible tool result goes through: the four
/// built-ins inside [`Tool::execute`], and a lane's answer at the agent's
/// lane seam (KI-041). It is public for exactly that reason — a second cap
/// elsewhere would drift from this one's marker and its bound, and a lane
/// answering with a batch of embedding vectors is precisely the payload the
/// cap exists for.
pub fn bound(outcome: ToolOutcome) -> ToolOutcome {
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

    #[test]
    fn use_skill_declares_one_string_name_and_is_not_one_of_the_closed_four() {
        let spec = serde_json::to_value(use_skill_spec()).unwrap();
        assert_eq!(spec["function"]["name"], USE_SKILL_TOOL);
        assert_eq!(spec["function"]["parameters"]["required"][0], "name");
        assert_eq!(spec["function"]["parameters"]["properties"]["name"]["type"], "string");
        // The closed four stay closed: `use_skill` is declared beside
        // them by the agent when a run carries skills, never inside them.
        assert_eq!(Tool::from_name(USE_SKILL_TOOL), None);
    }

    #[test]
    fn remember_declares_the_structured_shape_with_basis_required() {
        let spec = serde_json::to_value(remember_spec()).unwrap();
        assert_eq!(spec["function"]["name"], REMEMBER_TOOL);
        let parameters = &spec["function"]["parameters"];
        // The brain_remember shape: structured fields, mechanism over
        // prose.
        for field in [
            "name", "description", "type", "shape", "phrases", "missing", "basis", "tier",
            "valid-until", "body",
        ] {
            assert!(parameters["properties"].get(field).is_some(), "remember lacks {field}");
        }
        // `basis` is required on every write; `valid-until` is required
        // by the working tier, which the description names.
        let required: Vec<&str> =
            parameters["required"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(required, vec!["name", "description", "basis"]);
        assert_eq!(parameters["properties"]["basis"]["enum"][0], "confirmed-by");
        assert_eq!(parameters["properties"]["basis"]["enum"][1], "inferred");
        assert_eq!(parameters["properties"]["tier"]["enum"][0], "working");
        // The two memory tools are not part of the closed four either.
        assert_eq!(Tool::from_name(REMEMBER_TOOL), None);
        assert_eq!(Tool::from_name(SUPERSEDE_TOOL), None);
        // Slice 19: the phrases ride an array of strings, and the
        // description is their home — the slice-9 lesson, an instruction
        // with no home where the model sees it does not exist. It asks
        // for the resolution's phrases, for the sign, and for the third
        // sign's outcome with the companions named in `missing`.
        let phrases = &parameters["properties"]["phrases"];
        assert_eq!(phrases["type"], "array");
        assert_eq!(phrases["items"]["type"], "string");
        let missing = &parameters["properties"]["missing"];
        assert_eq!(missing["type"], "array");
        assert_eq!(missing["items"]["type"], "string");
        let description = spec["function"]["description"].as_str().unwrap();
        assert!(description.contains("phrases records the resolution"), "{description}");
        assert!(description.contains("anti-vector"), "{description}");
        assert!(description.contains("fail-open"), "{description}");
        assert!(description.contains("basis is required on every write"), "{description}");
        // The third vector's instruction, in the tool's own words: the
        // owner's most-mistaken-for-done case is recordable at all.
        assert!(description.contains("INCOMPLETE"), "{description}");
        assert!(description.contains("name those companions in missing"), "{description}");
        assert!(description.contains("fixed-width-clip"), "{description}");
        assert!(description.contains("foreign-root"), "{description}");
    }

    #[test]
    fn supersede_declares_the_justification_as_required() {
        let spec = serde_json::to_value(supersede_spec()).unwrap();
        assert_eq!(spec["function"]["name"], SUPERSEDE_TOOL);
        let parameters = &spec["function"]["parameters"];
        let required: Vec<&str> =
            parameters["required"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(required, vec!["name", "justification"]);
        // The modify path's replacement and its description are named,
        // and the permission model rides the description — the model
        // is the caller most likely to meet it.
        assert!(parameters["properties"].get("replacement").is_some());
        assert!(parameters["properties"].get("description").is_some());
        assert!(spec["function"]["description"].as_str().unwrap().contains("confirmed-by"));
        // The third vector's inherit-or-clear rule is stated where the
        // model reads it: omit `missing` to keep the companions, pass an
        // empty list to clear them.
        assert!(parameters["properties"].get("missing").is_some());
        let description = spec["function"]["description"].as_str().unwrap();
        assert!(description.contains("omit `missing` to keep the companions"), "{description}");
        assert!(description.contains("explicit empty list to clear them"), "{description}");
    }

    #[test]
    fn subagent_declares_the_goal_required_and_the_scope_narrowable() {
        let spec = serde_json::to_value(subagent_spec()).unwrap();
        assert_eq!(spec["function"]["name"], SUBAGENT_TOOL);
        let parameters = &spec["function"]["parameters"];
        // `goal` is the one required parameter: the child sees nothing of
        // the parent's conversation, so a goal that is not self-contained
        // is a task the child cannot do.
        let required: Vec<&str> =
            parameters["required"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(required, vec!["goal"]);
        // `tools` narrows the admitted scope, and its enum is the whole
        // admissible set: the durable-write, lane, and delegation tools
        // are not in it, so a conforming caller cannot name them.
        let admitted: Vec<&str> = parameters["properties"]["tools"]["items"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        assert_eq!(
            admitted,
            crate::subagents::ADMISSIBLE_TOOLS.to_vec(),
            "the schema enum and ADMISSIBLE_TOOLS are one set — the comment's coupling is this assertion"
        );
        // `subagent` is admissible because the depth cap is 2: naming it
        // in a declaration opens a second level, and the scope strips it
        // again at the cap. The durable-write and lane tools are not,
        // and never are.
        assert!(admitted.contains(&SUBAGENT_TOOL), "{admitted:?}");
        for excluded in [REMEMBER_TOOL, SUPERSEDE_TOOL, IMPROVE_TOOL, "mcp__sem1__probe"] {
            assert!(!admitted.contains(&excluded), "{excluded} must not be admissible");
        }
        // The turn cap is bounded in the schema as well as at parse.
        let turns = &parameters["properties"]["max-turns"];
        assert_eq!(turns["minimum"], 1);
        assert_eq!(turns["maximum"], 50);
        // The delegation tool is not one of the closed four either.
        assert_eq!(Tool::from_name(SUBAGENT_TOOL), None);
        // The description names the two facts the model most needs: the
        // goal is self-contained, and the child runs on the same model.
        let description = spec["function"]["description"].as_str().unwrap();
        assert!(description.contains("self-contained"), "{description}");
        assert!(description.contains("this same model"), "{description}");
        // The three parts that make a brief gradeable are declared, and
        // none of them is required: a call that omits all three is
        // handed the goal alone.
        for part in ["context", "output", "approach", "role"] {
            let declared = &parameters["properties"][part];
            assert_eq!(declared["type"], "string", "{part} must be declared a string");
        }
        assert!(description.contains("`context`"), "{description}");
        assert!(description.contains("`approach`"), "{description}");
        assert!(description.contains("`output`"), "{description}");
        assert!(description.contains("delta block"), "{description}");
        assert!(
            description.contains("The declaration IS the scope"),
            "{description}"
        );
    }

    #[test]
    fn a_forwarded_subagent_call_is_named_for_whose_call_it_is() {
        assert_eq!(subagent_call_prefix(), "subagent \u{2192} ");
        // The prefix is derived from the tool's own name, so renaming one
        // without the other is not possible.
        assert!(subagent_call_prefix().starts_with(SUBAGENT_TOOL));
    }

    /// One lane's roster as `tools/list` answers it, for the assembly
    /// tests below.
    fn lane_roster(lane: &str, tools: &[(&str, &str)]) -> LaneTools {
        LaneTools {
            lane: lane.to_string(),
            tools: tools
                .iter()
                .map(|(name, description)| LaneTool {
                    name: lane_tool_name(lane, name),
                    lane_tool: format!("{lane}_{name}"),
                    description: description.to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": { "items": { "type": "string" } },
                    }),
                })
                .collect(),
        }
    }

    #[test]
    fn lane_tool_names_round_trip_through_the_namespace() {
        assert_eq!(lane_tool_name("sem1", "probe"), "mcp__sem1__probe");
        assert_eq!(lane_of("mcp__sem1__probe"), Some(("sem1", "probe")));
        // A tool name with an underscore in it still splits on the lane's
        // separator only.
        assert_eq!(lane_of("mcp__sys1__sys1_classify"), Some(("sys1", "sys1_classify")));
        // Everything that is not a lane tool reads as not one.
        for name in ["bash", "read_file", "mcp__sem1", "mcp__sem1__", "mcp____x", "sem1_probe"] {
            assert_eq!(lane_of(name), None, "{name} must not read as a lane tool");
        }
    }

    #[test]
    fn declarations_put_the_builtins_first_and_namespace_the_lanes() {
        let lanes = vec![lane_roster("sem1", &[("probe", "probe the embeddings worker")])];
        let specs = declarations(&lanes);
        // The four built-ins keep their names and their order, always.
        assert_eq!(specs.len(), 5);
        let names: Vec<String> = specs
            .iter()
            .map(|spec| serde_json::to_value(spec).unwrap()["function"]["name"]
                .as_str()
                .unwrap()
                .to_string())
            .collect();
        assert_eq!(
            names,
            vec!["bash", "read_file", "write_file", "list_dir", "mcp__sem1__probe"]
        );
        // The lane's own description and schema rode along verbatim.
        let lane = serde_json::to_value(&specs[4]).unwrap();
        assert_eq!(lane["function"]["description"], "probe the embeddings worker");
        assert_eq!(
            lane["function"]["parameters"]["properties"]["items"]["type"],
            "string"
        );
    }

    #[test]
    fn declarations_keep_the_join_order_across_lanes() {
        let lanes = vec![
            lane_roster("sem1", &[("probe", "probe")]),
            lane_roster("sys1", &[("sys1_classify", "classify")]),
        ];
        let specs = declarations(&lanes);
        assert_eq!(specs.len(), 6);
        let last = serde_json::to_value(&specs[5]).unwrap();
        assert_eq!(last["function"]["name"], "mcp__sys1__sys1_classify");
        // No lanes joined: exactly the closed four, unchanged.
        assert_eq!(declarations(&[]).len(), 4);
    }

    #[test]
    fn a_lanes_roster_is_renamed_into_the_namespace_in_order() {
        let listed = vec![
            ToolInfo {
                name: "sem1_embed".to_string(),
                description: "embed".to_string(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
            ToolInfo {
                name: "sem1_probe".to_string(),
                description: "probe".to_string(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
        ];
        let tools = namespaced("sem1", &listed);
        // The redundant lane prefix is dropped inside the namespace: the
        // model calls `mcp__sem1__probe`, not `mcp__sem1__sem1_probe`.
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["mcp__sem1__embed", "mcp__sem1__probe"]);
        // Both ends of the mapping are recorded: the lane's own name is
        // what `tools/call` carries.
        let own: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
        assert_eq!(own, vec!["sem1_embed", "sem1_probe"]);
        assert_eq!(tools[0].description, "embed");
        assert_eq!(tools[1].input_schema, serde_json::json!({ "type": "object" }));
    }

    #[test]
    fn a_tool_without_the_lanes_prefix_keeps_its_own_name() {
        // Not every lane names its tools after itself; those keep their
        // name verbatim inside the namespace.
        let listed = vec![ToolInfo {
            name: "classify".to_string(),
            description: "classify".to_string(),
            input_schema: serde_json::json!({ "type": "object" }),
        }];
        let tools = namespaced("sys1", &listed);
        assert_eq!(tools[0].name, "mcp__sys1__classify");
        assert_eq!(tools[0].lane_tool, "classify");
    }

    #[test]
    fn two_tools_whose_short_names_meet_do_not_share_one_declaration() {
        // A lane shipping both `probe` and `sem1_probe`: the second one
        // falls back to its own full name, so each declaration names
        // exactly one tool.
        let listed = vec![
            ToolInfo {
                name: "probe".to_string(),
                description: "short".to_string(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
            ToolInfo {
                name: "sem1_probe".to_string(),
                description: "long".to_string(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
        ];
        let tools = namespaced("sem1", &listed);
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["mcp__sem1__probe", "mcp__sem1__sem1_probe"]);
        let own: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
        assert_eq!(own, vec!["probe", "sem1_probe"]);
    }

    /// One `tools/list` entry carrying nothing but its name, for the
    /// collision tests below.
    fn listed(name: &str) -> ToolInfo {
        ToolInfo {
            name: name.to_string(),
            description: String::new(),
            input_schema: serde_json::json!({ "type": "object" }),
        }
    }

    /// Every declared name resolves to exactly one tool, which is what
    /// makes it callable: the agent's lookup is a `find` over these.
    fn each_name_resolves_to_one_tool(tools: &[LaneTool]) {
        for tool in tools {
            let found: Vec<&LaneTool> =
                tools.iter().filter(|candidate| candidate.name == tool.name).collect();
            assert_eq!(
                found.len(),
                1,
                "{} is declared {} times: a call by that name runs at most one of them",
                tool.name,
                found.len()
            );
            assert_eq!(found[0].lane_tool, tool.lane_tool);
            assert!(
                lane_of(&tool.name).is_some_and(|(lane, _)| lane == "sem1"),
                "{} must still read as a sem1 tool",
                tool.name
            );
        }
    }

    #[test]
    fn a_lane_shipping_a_prefixed_and_a_bare_tool_declares_both() {
        // The corner the old fallback missed: `sem1_x` takes the short
        // name, and the bare `x`'s own full name *is* that same short
        // name, so falling back to the full name handed the second tool
        // the first one's declaration and shadowed it for the whole run.
        let tools = namespaced("sem1", &[listed("sem1_x"), listed("x")]);
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert_ne!(names[0], names[1], "two tools must not share one declaration: {names:?}");
        each_name_resolves_to_one_tool(&tools);
        // Both ends of each mapping survive: the lane's own name is what
        // `tools/call` carries.
        let own: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
        assert_eq!(own, vec!["sem1_x", "x"]);
    }

    #[test]
    fn a_third_colliding_tool_gets_a_numbered_name() {
        // Both natural names are gone after `sem1_x` and `x`, so a lane
        // that also lists `x_2` has to be numbered past it. The iteration
        // has to keep going rather than return the duplicate.
        let tools = namespaced("sem1", &[listed("sem1_x"), listed("x"), listed("x_2")]);
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names, vec!["mcp__sem1__x", "mcp__sem1__x_2", "mcp__sem1__x_2_2"]);
        each_name_resolves_to_one_tool(&tools);
        let own: Vec<&str> = tools.iter().map(|tool| tool.lane_tool.as_str()).collect();
        assert_eq!(own, vec!["sem1_x", "x", "x_2"]);
    }
}
