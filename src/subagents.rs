//! Subagents: one delegated task, run in its own child session on the
//! parent's own route, with a scoped tool set and the parent's approver.
//!
//! The port of the fork's `packages/subagent` — its depth accounting,
//! its parent-options-for-delegation pattern, and its model-selection
//! preflight — as a native implementation. Nothing here imports, vendors,
//! or shells out to the fork (hard rule 6); the vocabulary and the guard
//! shapes are what crossed over.
//!
//! Five decisions hold the whole slice together:
//!
//! 1. **The loop is reused, never forked.** A child is an ordinary
//!    [`Agent`] over an ordinary [`Session`], with the same compiled-in
//!    system prompt, the goal as its first and only user message, a
//!    scoped declaration set, and its own turn cap.
//! 2. **Same model, structurally.** The child runs on the *same* routed
//!    client the parent is using and takes its model from the parent's
//!    own options, so "never downgrades" is a property of the
//!    construction rather than a rule the model is asked to follow.
//!    There is no per-call model parameter anywhere.
//! 3. **Same approver.** The child's calls reach the owner through the
//!    parent's gate, labelled with the subagent and its goal, so the
//!    owner knows whose call it is before deciding.
//! 4. **Depth one.** A subagent cannot spawn subagents: the tool is
//!    outside its admitted scope, and [`SUBAGENT_MAX_DEPTH`] names the
//!    cap for a later slice.
//! 5. **Never silence.** A failure returns the partial result, the typed
//!    reason, and the child's log path, in the parent's ordinary bounded
//!    `tool_result` — so the parent's log carries the call and its
//!    outcome either way (hard rule 5).

use crate::agent::{Agent, AgentOptions, Approver, SubagentStage, UiEvent};
use crate::api::{Client, FunctionCall, ToolCall};
use crate::calib::Stream;
use crate::route::RouteDecision;
use crate::session::{Record, Session};
use crate::skills::Skill;
use crate::tools::{self, ToolOutcome};
use std::io::Write;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

/// The turn cap a delegation gets when it does not ask for one. Lower
/// than a run's own [`crate::agent::DEFAULT_MAX_TURNS`] on purpose: a
/// delegated task is one bounded piece of work, and a child that needs
/// twenty turns to say what it found is a child whose goal was wrong.
pub const SUBAGENT_TURN_CAP: usize = 20;

/// The bounds on a requested turn cap. The schema carries them too, but
/// a bound only the schema carries is a bound a non-conforming caller
/// ignores, so the parse carries them as well.
pub const SUBAGENT_MIN_TURNS: usize = 1;
pub const SUBAGENT_MAX_TURNS: usize = 50;

/// The delegation-depth cap: a subagent's own run sits at depth 1 and
/// cannot produce a depth-2 child. Enforcement is structural — the
/// `subagent` tool is outside a child's admitted scope, so the loop
/// refuses the call by name — and this constant names the cap the
/// refusal cites and the assertion in [`run`] checks.
pub const SUBAGENT_MAX_DEPTH: usize = 1;

/// How long a goal reads in one progress line or an approval prompt.
const GOAL_PREVIEW: usize = 72;

/// Every tool name a subagent may be admitted to. This is the same list
/// the `tools` parameter's JSON Schema enum carries; the two are kept
/// in step by the declaration test in `crate::tools`, because a scope
/// the schema cannot express is a scope the model will ask for anyway.
pub const ADMISSIBLE_TOOLS: [&str; 5] =
    ["bash", "read_file", "write_file", "list_dir", tools::USE_SKILL_TOOL];

/// The tool set one subagent run is admitted to.
///
/// A scope does two things, and both matter: it decides which
/// declarations the child's requests carry, and it decides which calls
/// the loop refuses. A tool excluded from the declarations is not
/// callable by a conforming model; the refusal is what meets a
/// non-conforming one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Every admitted model-visible tool name, in the order the child
    /// declares them.
    admitted: Vec<String>,
    /// This run's delegation depth: 0 for an ordinary run, 1 for a
    /// subagent's own run. The child's depth is this plus one.
    pub depth: usize,
}

impl Scope {
    /// The default admitted scope: the four built-ins plus `use_skill`.
    /// Everything else — the two memory tools, the lane namespace, and
    /// `subagent` itself — is outside it by construction.
    pub fn default_admitted() -> Vec<String> {
        crate::tools::ALL_TOOLS
            .iter()
            .map(|tool| tool.name().to_string())
            .chain(std::iter::once(tools::USE_SKILL_TOOL.to_string()))
            .collect()
    }

    /// A child's scope: `requested` when the call narrowed it, the
    /// default otherwise, stamped with the child's delegation depth.
    pub fn child(requested: Option<Vec<String>>, depth: usize) -> Scope {
        Scope { admitted: requested.unwrap_or_else(Self::default_admitted), depth }
    }

    /// Whether `name` is inside this scope.
    pub fn admits(&self, name: &str) -> bool {
        self.admitted.iter().any(|tool| tool == name)
    }

    /// Every admitted name, in declaration order.
    pub fn names(&self) -> &[String] {
        &self.admitted
    }
}

/// One `subagent` call's arguments, parsed and validated.
///
/// Every failure is a typed refusal the model can read and react to, in
/// the same `error:` vocabulary a built-in's argument failure uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRequest {
    /// The delegated task, stated in full.
    pub goal: String,
    /// The admitted scope the call asked for; `None` is the default.
    pub tools: Option<Vec<String>>,
    /// The child's turn cap.
    pub max_turns: usize,
}

impl SubagentRequest {
    /// Parse one call's arguments object.
    pub fn parse(arguments: &serde_json::Value) -> Result<SubagentRequest, String> {
        let goal = arguments
            .get("goal")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|goal| !goal.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                "the `goal` argument is required and must be a non-empty string: the subagent sees nothing of this conversation, so the goal has to carry the whole task".to_string()
            })?;
        let tools = match arguments.get("tools") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::Array(items)) => {
                let mut admitted: Vec<String> = Vec::with_capacity(items.len());
                for item in items {
                    let name = item.as_str().ok_or_else(|| {
                        format!("every `tools` entry must be a tool name; {item} is not a string")
                    })?;
                    if !ADMISSIBLE_TOOLS.contains(&name) {
                        return Err(format!(
                            "`{name}` is not admissible in a subagent's scope; the admissible tools are {}",
                            ADMISSIBLE_TOOLS.join(", ")
                        ));
                    }
                    // A repeated name narrows nothing twice.
                    if !admitted.iter().any(|tool| tool == name) {
                        admitted.push(name.to_string());
                    }
                }
                Some(admitted)
            }
            Some(other) => {
                return Err(format!("`tools` must be an array of tool names; {other} is not"))
            }
        };
        let max_turns = match arguments.get("max-turns") {
            None | Some(serde_json::Value::Null) => SUBAGENT_TURN_CAP,
            Some(value) => {
                let asked = value.as_u64().filter(|asked| {
                    *asked >= SUBAGENT_MIN_TURNS as u64 && *asked <= SUBAGENT_MAX_TURNS as u64
                });
                match asked {
                    Some(asked) => asked as usize,
                    None => {
                        return Err(format!(
                            "`max-turns` must be an integer between {SUBAGENT_MIN_TURNS} and {SUBAGENT_MAX_TURNS}"
                        ))
                    }
                }
            }
        };
        Ok(SubagentRequest { goal, tools, max_turns })
    }
}

/// The route one child runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRoute {
    /// The child's model: the parent's own, always.
    pub model: String,
    /// The parent's active provider, when its client is routed. Carried
    /// so the progress line and the log can name the route the child
    /// actually took.
    pub provider: Option<String>,
}

/// Resolve the route one child runs on.
///
/// This is the in-product form of the owner's never-downgrade directive
/// (plan directive 2, hard rule 9), and the port of the fork's
/// model-selection preflight: the fork checks a child's requested route
/// against an allowed list before creating it, and Fleet's equivalent is
/// structural rather than advisory. A delegation carries no model
/// parameter at all — there is no `model` in the schema and none in the
/// request — so the only way a child could run cheaper than its parent
/// is by resolving a different model right here. It does not: the
/// child's model is the parent's, read from the parent's own options,
/// and the parent's active decision rides along only to be named.
pub fn child_route(parent_model: &str, decision: Option<&RouteDecision>) -> ChildRoute {
    ChildRoute {
        model: parent_model.to_string(),
        provider: decision.map(|decision| decision.provider.clone()),
    }
}

/// Why a child stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The child answered without requesting tools.
    Completed,
    /// The child's turn cap was reached first.
    MaxTurns,
    /// The child failed: an API error, or a session it could not open.
    Failed,
}

impl Outcome {
    /// The outcome's name, in the session log's own ending vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::MaxTurns => "max-turns",
            Outcome::Failed => "failed",
        }
    }
}

/// What one delegated run produced.
///
/// The shape the fork's descriptor projects and the shape the parent's
/// `tool_result` renders: the child's final message, where its work
/// lives, how far it got, and how it ended. A failure fills `summary`
/// with whatever the child managed and `reason` with the typed cause —
/// never an empty `Ok`, never silence.
#[derive(Debug, Clone)]
pub struct SubagentResult {
    /// The child's final message, or its last words when it did not
    /// finish.
    pub summary: String,
    /// The child's session id.
    pub session: String,
    /// The child's log path.
    pub log: PathBuf,
    /// The turns the child ran.
    pub turns: u64,
    pub outcome: Outcome,
    /// The typed failure reason, on [`Outcome::Failed`].
    pub reason: Option<String>,
}

impl SubagentResult {
    /// A run that never got as far as a child session.
    fn refused(reason: String) -> SubagentResult {
        SubagentResult {
            summary: String::new(),
            session: String::new(),
            log: PathBuf::new(),
            turns: 0,
            outcome: Outcome::Failed,
            reason: Some(reason),
        }
    }

    /// The model-visible text the parent's `tool_result` carries: one
    /// header line naming the outcome, the turns, the session, and the
    /// log, then the child's own words. It rides the same bounded
    /// `tool_result` path every other tool's output rides.
    pub fn render(&self) -> String {
        let head = match &self.reason {
            Some(reason) => format!("subagent failed ({reason})"),
            None => format!("subagent {}", self.outcome.label()),
        };
        let mut text =
            format!("{head} \u{b7} {} turn{} \u{b7} session {}", self.turns, plural(self.turns), self.session);
        if !self.log.as_os_str().is_empty() {
            text.push_str(&format!(" \u{b7} log {}", self.log.display()));
        }
        text.push('\n');
        if self.summary.trim().is_empty() {
            // Never silence: a child that produced no words still says
            // so, and still names where its work is.
            text.push_str(match self.outcome {
                Outcome::Completed => {
                    "(the child finished with no final text \u{2014} its log holds the turns)"
                }
                _ => "(no partial answer \u{2014} the child's log holds what it did)",
            });
        } else {
            text.push_str(&self.summary);
        }
        text
    }
}

/// Everything one delegated run needs, gathered by the caller so the
/// borrow of the parent's approver and progress sink is explicit rather
/// than implicit.
pub struct Delegation<'a> {
    /// The parent's own client, routed or not. The child shares it, so
    /// the child's requests cross the same roster with the same
    /// failover — that is what makes "same model" structural.
    pub client: &'a Client,
    /// The parent's active route decision, when its client is routed.
    pub route: Option<RouteDecision>,
    /// The parent's model, from its own options.
    pub model: String,
    /// The sessions directory the child's log belongs in: the parent
    /// log's own directory, so a child never lands somewhere else.
    pub sessions_dir: PathBuf,
    /// The delegating session's id, recorded on the child's start record.
    pub parent_session: String,
    /// The delegated task, verbatim.
    pub goal: String,
    /// The admitted scope, already validated.
    pub tools: Option<Vec<String>>,
    /// The child's turn cap.
    pub max_turns: usize,
    /// The parent's approver. The child's calls cross it, labelled.
    pub approver: &'a mut (dyn Approver + 'a),
    /// The parent's progress sink. A child's own progress lines land
    /// here, so a headless run shows the child working and a TUI's sink
    /// stays quiet.
    pub progress: &'a mut dyn Write,
    /// Where the parent's live surface listens, for the child's
    /// spawned/working/done progress. The child's own turn, token, and
    /// finish events deliberately do not ride here: they belong to the
    /// child's conversation, and landing them in the parent's would
    /// move its turn counter and clear its working state mid-run.
    pub ui_tx: Option<UnboundedSender<UiEvent>>,
    /// The calibration stream the parent's rows go to. The child's turn
    /// and screening rows land in the same stream, naturally: they are
    /// real turns on the real roster, and no new op is needed.
    pub calib: Option<Stream>,
    /// The skills the parent declared. The child declares the same list,
    /// because `use_skill` is in its admitted scope and a tool whose
    /// every call fails is not a tool.
    pub skills: Vec<Skill>,
    /// The child's delegation depth.
    pub depth: usize,
}

/// Run one delegated task to a stop and return what it produced.
///
/// Synchronous from the parent's side: the parent's turn waits. Every
/// exit path returns a [`SubagentResult`] — a stop, a cap, an API
/// failure, a session that could not be opened, a depth that exceeds the
/// cap — and every one of them carries the child's log path once a log
/// exists.
pub async fn run(delegation: Delegation<'_>) -> SubagentResult {
    // The depth cap, asserted before anything is created. Unreachable
    // through the tool today — a subagent's scope excludes `subagent`, so
    // the loop refuses the call first — and named here so a later slice
    // that opens a second level meets the guard rather than inventing
    // one.
    if delegation.depth > SUBAGENT_MAX_DEPTH {
        return SubagentResult::refused(format!(
            "subagent depth {} exceeds the cap {SUBAGENT_MAX_DEPTH}",
            delegation.depth
        ));
    }
    let Delegation {
        client,
        route,
        model,
        sessions_dir,
        parent_session,
        goal,
        tools,
        max_turns,
        approver,
        progress,
        ui_tx,
        calib,
        skills,
        depth,
    } = delegation;

    let child_route = child_route(&model, route.as_ref());
    let mut child = match Session::create_subagent(&sessions_dir, &child_route.model, &parent_session)
    {
        Ok(child) => child,
        Err(error) => {
            return SubagentResult::refused(format!(
                "the child session could not be created: {error}"
            ))
        }
    };
    let session = child.id().to_string();
    let log = child.path().to_path_buf();

    let route_note = match &child_route.provider {
        Some(provider) => format!(" on {provider}"),
        None => String::new(),
    };
    let _ = writeln!(
        progress,
        "\u{2014} subagent: {} \u{b7} child {session} \u{b7} {max_turns}-turn cap{route_note}",
        bounded(&goal, GOAL_PREVIEW)
    );
    let _ = progress.flush();
    emit(
        &ui_tx,
        UiEvent::Subagent {
            goal: bounded(&goal, GOAL_PREVIEW),
            session: session.clone(),
            stage: SubagentStage::Spawned,
        },
    );

    let options = AgentOptions {
        model: child_route.model,
        max_turns,
        ui_tx: None,
        lane_specs: None,
        calib,
        skills,
        memory: None,
        scope: Some(Scope::child(tools, depth)),
        // The child sees nothing of the parent: no memory, no lanes, no
        // home to file proposals into. The scope excludes `improve`
        // either way, so the declaration and the call agree.
        improve_home: None,
    };
    let gate = InheritedApprover {
        inner: approver,
        goal: goal.clone(),
        ui_tx: ui_tx.clone(),
        session: session.clone(),
        calls: 0,
    };
    // The child is scoped to its own block so the borrow of `child`
    // ends before the log is read back for the partial result.
    let outcome = {
        let mut agent = match Agent::new(client, options, &mut child, Box::new(gate), progress) {
            Ok(agent) => agent,
            Err(error) => {
                let result = SubagentResult {
                    summary: String::new(),
                    session,
                    log,
                    turns: 0,
                    outcome: Outcome::Failed,
                    reason: Some(error.to_string()),
                };
                emit(
                    &ui_tx,
                    UiEvent::Subagent {
                        goal: bounded(&goal, GOAL_PREVIEW),
                        session: result.session.clone(),
                        stage: SubagentStage::Done {
                            outcome: Outcome::Failed.label().to_string(),
                            turns: 0,
                        },
                    },
                );
                return result;
            }
        };
        agent.run(&goal).await
    };

    let records = crate::session::replay(&log).unwrap_or_default();
    let mut result = match outcome {
        Ok(report) => SubagentResult {
            summary: report.final_message.unwrap_or_default(),
            session,
            log,
            turns: report.turns,
            outcome: match report.stop_reason {
                crate::agent::StopReason::Completed => Outcome::Completed,
                crate::agent::StopReason::MaxTurns => Outcome::MaxTurns,
            },
            reason: None,
        },
        Err(error) => SubagentResult {
            summary: String::new(),
            session,
            log,
            // The turns the child actually completed, read back from its
            // own log: the loop's counter would name the turn it died on.
            turns: turns_completed(&records),
            outcome: Outcome::Failed,
            reason: Some(error.to_string()),
        },
    };
    // A child that stopped without a final message still returns its
    // partial result: the last words in its own log. Silence is the one
    // thing this path never returns. A child that *completed* keeps its
    // own (possibly empty) final message — an earlier turn's text is not
    // its answer.
    if result.outcome != Outcome::Completed && result.summary.trim().is_empty() {
        if let Some(words) = last_words(&records) {
            result.summary = words;
        }
    }
    let _ = writeln!(
        progress,
        "\u{2190} subagent {} \u{b7} {} turn{}",
        result.outcome.label(),
        result.turns,
        plural(result.turns)
    );
    let _ = progress.flush();
    emit(
        &ui_tx,
        UiEvent::Subagent {
            goal: bounded(&goal, GOAL_PREVIEW),
            session: result.session.clone(),
            stage: SubagentStage::Done {
                outcome: result.outcome.label().to_string(),
                turns: result.turns,
            },
        },
    );
    result
}

/// The parent's [`Approver`], seen from the child's side.
///
/// One gate for the whole tree: the owner sees a subagent's `bash` call
/// before it runs, exactly as they see the parent's. The forwarded call
/// is renamed `subagent \u{2192} <tool>` and carries the goal, so the
/// prompt answers "whose call is this?" without the owner having to
/// know which session is in flight.
struct InheritedApprover<'a> {
    inner: &'a mut (dyn Approver + 'a),
    goal: String,
    ui_tx: Option<UnboundedSender<UiEvent>>,
    session: String,
    /// How many of the child's calls this gate has forwarded. The
    /// child's liveness signal, reported to the parent's surface.
    calls: u64,
}

impl Approver for InheritedApprover<'_> {
    fn approve(&mut self, call: &ToolCall) -> bool {
        self.calls += 1;
        emit(
            &self.ui_tx,
            UiEvent::Subagent {
                goal: bounded(&self.goal, GOAL_PREVIEW),
                session: self.session.clone(),
                stage: SubagentStage::Working { calls: self.calls, tool: call.function.name.clone() },
            },
        );
        self.inner.approve(&labelled(call, &self.goal))
    }

    fn latched(&self) -> bool {
        self.inner.latched()
    }
}

/// The child's call as the parent's gate sees it: named for the
/// subagent it came from, carrying the goal beside the call's own
/// arguments so the prompt can name both.
fn labelled(call: &ToolCall, goal: &str) -> ToolCall {
    let arguments: serde_json::Value =
        serde_json::from_str(&call.function.arguments).unwrap_or(serde_json::Value::Null);
    ToolCall {
        id: call.id.clone(),
        kind: call.kind.clone(),
        function: FunctionCall {
            name: format!("{}{}", tools::subagent_call_prefix(), call.function.name),
            arguments: serde_json::json!({
                "goal": bounded(goal, GOAL_PREVIEW),
                "tool": call.function.name,
                "arguments": arguments,
            })
            .to_string(),
        },
    }
}

/// The child's last words, for a run that did not finish: the last
/// assistant message carrying text in its own log. `None` when the child
/// never got a word out.
fn last_words(records: &[Record]) -> Option<String> {
    records.iter().rev().find_map(|record| match record {
        Record::Assistant { content: Some(text), .. } if !text.trim().is_empty() => {
            Some(text.clone())
        }
        _ => None,
    })
}

/// How many turns a run completed, read back from its log: one assistant
/// record per turn that produced a message. A run that failed mid-request
/// has no record for the turn it died on, so the log's count is the honest
/// "how far it got" — the loop's own counter would name a turn the model
/// never answered.
fn turns_completed(records: &[Record]) -> u64 {
    records.iter().filter(|record| matches!(record, Record::Assistant { .. })).count() as u64
}

/// Send one event when a surface is listening. A closed channel is not
/// a failure: the session log and the run report are the record.
fn emit(ui_tx: &Option<UnboundedSender<UiEvent>>, event: UiEvent) {
    if let Some(tx) = ui_tx {
        let _ = tx.send(event);
    }
}

/// `s` for anything but one.
fn plural(count: u64) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// First `limit` characters of `text` with an elision marker when
/// anything was cut, for a progress line or a prompt.
fn bounded(text: &str, limit: usize) -> String {
    let mut cut: String = text.chars().take(limit).collect();
    if text.chars().count() > limit {
        cut.push('\u{2026}');
    }
    cut
}

/// Bound one child's rendered result the way every other tool's output
/// is bounded, so a child that wrote a novel still rides the parent's
/// ordinary `tool_result` path.
pub fn bound_result(result: &SubagentResult) -> ToolOutcome {
    tools::bound(ToolOutcome { output: result.render(), truncated: false })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentError;

    #[test]
    fn the_default_scope_is_the_four_builtins_plus_use_skill() {
        let scope = Scope::child(None, 1);
        assert_eq!(scope.depth, 1);
        assert_eq!(
            scope.names(),
            &vec![
                "bash".to_string(),
                "read_file".to_string(),
                "write_file".to_string(),
                "list_dir".to_string(),
                "use_skill".to_string(),
            ]
        );
        // Everything the design leaves out is out by construction.
        for excluded in [tools::REMEMBER_TOOL, tools::SUPERSEDE_TOOL, tools::SUBAGENT_TOOL] {
            assert!(!scope.admits(excluded), "{excluded} must not be admitted by default");
        }
        assert!(!scope.admits("mcp__sem1__probe"), "a lane tool must not be admitted");
    }

    #[test]
    fn a_requested_scope_narrows_and_keeps_its_order() {
        let scope = Scope::child(
            Some(vec!["read_file".to_string(), "bash".to_string()]),
            1,
        );
        assert_eq!(scope.names(), &vec!["read_file".to_string(), "bash".to_string()]);
        assert!(!scope.admits("write_file"));
    }

    #[test]
    fn an_empty_requested_scope_admits_nothing() {
        // A child with no tools is a legitimate narrowing: it can only
        // answer.
        let scope = Scope::child(Some(Vec::new()), 1);
        assert!(scope.names().is_empty());
        assert!(!scope.admits("bash"));
    }

    #[test]
    fn a_goal_is_required_and_must_say_something() {
        for missing in [
            serde_json::json!({}),
            serde_json::json!({ "goal": "" }),
            serde_json::json!({ "goal": "   " }),
            serde_json::json!({ "goal": 7 }),
        ] {
            let error = SubagentRequest::parse(&missing).unwrap_err();
            assert!(error.contains("goal"), "{error}");
        }
    }

    #[test]
    fn the_turn_cap_defaults_and_is_bounded_both_ways() {
        let asked = SubagentRequest::parse(&serde_json::json!({ "goal": "go" })).unwrap();
        assert_eq!(asked.max_turns, SUBAGENT_TURN_CAP);
        let capped =
            SubagentRequest::parse(&serde_json::json!({ "goal": "go", "max-turns": 7 })).unwrap();
        assert_eq!(capped.max_turns, 7);
        for out_of_range in [0, 51, 1_000] {
            let error = SubagentRequest::parse(&serde_json::json!({
                "goal": "go",
                "max-turns": out_of_range,
            }))
            .unwrap_err();
            assert!(error.contains("max-turns"), "{error}");
        }
    }

    #[test]
    fn an_inadmissible_tool_is_refused_by_name() {
        for name in [tools::REMEMBER_TOOL, tools::SUPERSEDE_TOOL, tools::SUBAGENT_TOOL] {
            let error = SubagentRequest::parse(&serde_json::json!({
                "goal": "go",
                "tools": ["bash", name],
            }))
            .unwrap_err();
            assert!(error.contains(name), "{error}");
            assert!(error.contains("not admissible"), "{error}");
        }
        // A lane tool is refused the same way, by its namespaced name.
        let error = SubagentRequest::parse(&serde_json::json!({
            "goal": "go",
            "tools": ["mcp__sem1__probe"],
        }))
        .unwrap_err();
        assert!(error.contains("mcp__sem1__probe"), "{error}");
    }

    #[test]
    fn a_repeated_tool_narrows_nothing_twice() {
        let request = SubagentRequest::parse(&serde_json::json!({
            "goal": "go",
            "tools": ["bash", "bash", "read_file"],
        }))
        .unwrap();
        let scope = Scope::child(request.tools.clone(), 1);
        assert_eq!(scope.names(), &vec!["bash".to_string(), "read_file".to_string()]);
    }

    #[test]
    fn a_child_runs_on_the_parents_model_whatever_the_decision_says() {
        // The never-downgrade guard, in its testable form: the child's
        // model is the parent's, and no decision can move it.
        let decision = RouteDecision {
            provider: "zai-coding-plan".to_string(),
            model: "GLM-5.3-Flash".to_string(),
            how: crate::route::How::Judged,
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            reason: "sys1 judged `hard`".to_string(),
            start_index: 0,
        };
        let routed = child_route("test-model", Some(&decision));
        assert_eq!(routed.model, "test-model");
        assert_eq!(routed.provider.as_deref(), Some("zai-coding-plan"));
        // An unrouted parent's child is unrouted too, and still on the
        // parent's model.
        let plain = child_route("test-model", None);
        assert_eq!(plain.model, "test-model");
        assert_eq!(plain.provider, None);
    }

    #[test]
    fn a_result_renders_its_outcome_its_turns_and_its_log() {
        let result = SubagentResult {
            summary: "found three call sites".to_string(),
            session: "20261014T000000Z-abcdef".to_string(),
            log: PathBuf::from("/tmp/sessions/20261014T000000Z-abcdef.jsonl"),
            turns: 3,
            outcome: Outcome::Completed,
            reason: None,
        };
        let text = result.render();
        assert!(text.starts_with("subagent completed \u{b7} 3 turns \u{b7} session "), "{text}");
        assert!(text.contains("/tmp/sessions/20261014T000000Z-abcdef.jsonl"), "{text}");
        assert!(text.ends_with("found three call sites"), "{text}");
        // One turn is not "1 turns".
        let one = SubagentResult { turns: 1, ..result.clone() };
        assert!(one.render().contains("1 turn \u{b7}"), "{}", one.render());
    }

    #[test]
    fn a_failure_renders_its_reason_and_never_returns_silence() {
        let failed = SubagentResult {
            summary: "checked two of five files".to_string(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/sessions/child-1.jsonl"),
            turns: 2,
            outcome: Outcome::Failed,
            reason: Some("API error 402: the account needs a balance top-up".to_string()),
        };
        let text = failed.render();
        assert!(text.contains("subagent failed (API error 402"), "{text}");
        assert!(text.contains("checked two of five files"), "{text}");
        // No words at all still says so, and still names the log.
        let silent = SubagentResult { summary: String::new(), ..failed.clone() };
        let text = silent.render();
        assert!(text.contains("no partial answer"), "{text}");
        assert!(text.contains("child-1.jsonl"), "{text}");
    }

    #[test]
    fn a_refused_run_names_its_reason_and_has_no_log() {
        let refused = SubagentResult::refused("the child session could not be created: nope".to_string());
        assert_eq!(refused.outcome, Outcome::Failed);
        assert!(refused.session.is_empty());
        assert!(refused.log.as_os_str().is_empty());
        assert!(refused.render().contains("the child session could not be created"));
    }

    #[test]
    fn a_forwarded_call_is_named_for_the_subagent_and_carries_the_goal() {
        let call = ToolCall {
            id: "call-1".to_string(),
            kind: "function".to_string(),
            function: FunctionCall {
                name: "bash".to_string(),
                arguments: r#"{"command":"ls"}"#.to_string(),
            },
        };
        let forwarded = labelled(&call, "summarize the repo");
        assert_eq!(forwarded.function.name, "subagent \u{2192} bash");
        let arguments: serde_json::Value =
            serde_json::from_str(&forwarded.function.arguments).unwrap();
        assert_eq!(arguments["goal"], "summarize the repo");
        assert_eq!(arguments["tool"], "bash");
        assert_eq!(arguments["arguments"]["command"], "ls");
        // The child's own call is untouched: the labelling happens on the
        // way to the gate, never on the way back.
        assert_eq!(call.function.name, "bash");
    }

    #[test]
    fn a_long_goal_is_bounded_for_a_prompt_and_a_progress_line() {
        let long = "x".repeat(200);
        assert_eq!(bounded(&long, GOAL_PREVIEW).chars().count(), GOAL_PREVIEW + 1);
        assert!(bounded(&long, GOAL_PREVIEW).ends_with('\u{2026}'));
        assert_eq!(bounded("short", GOAL_PREVIEW), "short");
    }

    #[test]
    fn the_last_words_of_a_log_are_the_last_assistant_text() {
        let records = vec![
            Record::Start {
                v: 1,
                model: "m".to_string(),
                created: "c".to_string(),
                kind: Some(crate::session::SUBAGENT_KIND.to_string()),
                parent: Some("p".to_string()),
            },
            Record::User { content: "the goal".to_string() },
            Record::Assistant { content: Some("first try".to_string()), tool_calls: vec![] },
            Record::Assistant { content: None, tool_calls: vec![] },
            Record::Assistant { content: Some("last words".to_string()), tool_calls: vec![] },
            Record::End { stop_reason: "max-turns".to_string(), turns: 3 },
        ];
        assert_eq!(last_words(&records).as_deref(), Some("last words"));
        // A log with no assistant text has no last words.
        let empty = vec![Record::User { content: "the goal".to_string() }];
        assert_eq!(last_words(&empty), None);
    }

    #[test]
    fn an_agent_error_names_the_class_the_calibration_row_would() {
        // The typed reason a failed child returns is the agent error's
        // own display, so the parent's log and the child's turn row name
        // the same cause.
        let error = AgentError::Api(crate::api::ClientError::Api(crate::api::ApiError {
            status: 402,
            message: "Insufficient Balance".to_string(),
        }));
        let result = SubagentResult {
            summary: String::new(),
            session: "child-1".to_string(),
            log: PathBuf::from("/tmp/child-1.jsonl"),
            turns: 1,
            outcome: Outcome::Failed,
            reason: Some(error.to_string()),
        };
        assert!(result.reason.as_deref().unwrap().contains("402"), "{:?}", result.reason);
        assert!(result.render().contains("Insufficient Balance"));
    }
}
