//! Multi-provider routing: the roster of OpenAI-compatible gateways, the
//! owner's declared chain, sys1-judged per-task selection, failover across
//! failure classes, and the calibration decision row.
//!
//! The roster is a list of [`Candidate`]s in the owner's declared order
//! (directive 2026-10-13): the router kit at `127.0.0.1:8300` FIRST, then
//! Z.ai Coding Plan, Xiaomi MiMo, StepFun, the Qwen Token plan, and
//! DeepSeek direct last — DeepSeek is a working account held in reserve.
//! Liquid is declared after DeepSeek and enters no declared position: it
//! is available to the owner's config (a pin or a chain entry), and
//! moving it into the chain is the owner's one-line config change, never
//! this table's. Every fact in the compiled-in defaults is sourced from
//! the fork's `agnostic-router-kit` `roster.json` (endpoints, key
//! env-var *names*, model ids); no key value is ever stored here.
//!
//! The three moving parts:
//!
//! - **Selection** ([`select`]) asks `sys1_classify` (task `route`) over the
//!   slice-4 MCP client for a workload verdict and maps it to a candidate;
//!   an unusable/absent/failed verdict, or a mapped candidate with no key,
//!   falls back to the declared HEAD. Routing never blocks a run.
//! - **Failover** ([`Router::chat_stream`]) walks the declared chain from
//!   the selected candidate on the failure classes in [`failure_class`];
//!   a candidate that starts streaming is committed and a mid-stream
//!   failure fails the run typed — no silent restart.
//! - **The decision row** ([`calib::RouteRow`]) is appended to the
//!   calibration stream (`src/calib.rs`) once per session, best-effort, so
//!   a full disk never fails a run. Slice 6 also reads that stream back:
//!   a candidate with a trailing streak of failed attempts is demoted to
//!   the end of this session's failover walk.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::{ChatRequest, ChatResponse, Client, ClientError, StreamEvent};
use crate::calib;
use crate::lanes;
use crate::mcp::{McpServer, ServerSpec};
use crate::session;

/// The declared HEAD of the chain: the local router kit, the primary
/// route, not the escape hatch. It aggregates the prepaid plans and
/// routes `auto` by workload itself.
pub const HEAD_ID: &str = "router-kit";

/// The sys1 lane name whose `sys1_classify` tool judges a route.
const JUDGE_LANE: &str = "sys1";

/// The sys1 task that answers the routing judgment (workload + execution).
const JUDGE_TASK: &str = "route";

// --- the roster -------------------------------------------------------------

/// One roster entry: an OpenAI-compatible gateway, where it lives, the
/// *name* of the env var holding its key (never a value), and the model it
/// serves. `enabled`/`key_required` gate whether a run may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub base_url: String,
    /// The environment variable NAME holding this candidate's key. The
    /// value is resolved at runtime from the environment, `<home>/.env`,
    /// or the harness credential store — never stored here.
    pub key_env: String,
    pub model: String,
    pub enabled: bool,
    /// Whether a key is required to attempt this candidate. False only for
    /// the loopback router kit, which is attempted even before its local
    /// token is configured (a keyless attempt is one fast 401 that
    /// failover walks past).
    pub key_required: bool,
    /// A named reason when the entry is disabled, so the chain still works
    /// and the reason is visible rather than silent.
    pub disabled_reason: Option<String>,
}

/// The compiled-in defaults, in the owner's declared chain order. Every
/// endpoint, key env-var name, and model id is sourced from the fork's
/// `agnostic-router-kit` `roster.json` (the spec), except DeepSeek's model,
/// which stays Fleet's own `deepseek-chat` default for consistency with the
/// existing client and credential resolution.
///
/// Liquid is appended at the END and declared only: the chain order above
/// is the owner's standing directive, and this table does not reorder it.
/// The owner's config (a pin, or a chain entry) decides Liquid's real
/// position whenever they want it there.
fn default_roster() -> Vec<Candidate> {
    vec![
        Candidate {
            id: "router-kit".into(),
            base_url: "http://127.0.0.1:8300/v1".into(),
            key_env: "AGNOSTIC_ROUTER_TOKEN".into(),
            model: "auto".into(),
            enabled: true,
            key_required: false,
            disabled_reason: None,
        },
        Candidate {
            id: "zai-coding-plan".into(),
            base_url: "https://api.z.ai/api/coding/paas/v4".into(),
            key_env: "ZAI_CODING_API_KEY".into(),
            model: "GLM-5.3-Flash".into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "xiaomi-mimo".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
            key_env: "XIAOMI_MIMO_API_KEY".into(),
            model: "mimo-v2.6-flash".into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "stepfun".into(),
            base_url: "https://api.stepfun.ai/step_plan/v1".into(),
            key_env: "STEPFUN_API_KEY".into(),
            model: "step-5-preview".into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "token-plan".into(),
            base_url: "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1".into(),
            key_env: "TOKEN_PLAN_API_KEY".into(),
            model: "qwen3.8-flash".into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "deepseek".into(),
            base_url: crate::api::DEFAULT_BASE_URL.into(),
            key_env: "DEEPSEEK_API_KEY".into(),
            model: crate::api::DEFAULT_MODEL.into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        // Liquid's own API — the blend1 generative lane's endpoint
        // (lfm2.5-24b-a2b), declared here so the owner's config can pin
        // or chain it. Appended at the END of the default order on
        // purpose: the declared chain above is the owner's standing
        // directive (router kit → z.ai → mimo → stepfun → token plan →
        // deepseek) and this slice does not reorder it; the owner's
        // config decides liquid's real position. Note the base URL's
        // shape: Liquid's chat completions live under `/openai/v1`,
        // not `/v1` (a `/v1` path 404s), and one key — LIQUID_API_KEY —
        // serves both the d1 router and the 24B generative lane.
        Candidate {
            id: "liquid".into(),
            base_url: "https://api.liquid.ai/openai/v1".into(),
            key_env: "LIQUID_API_KEY".into(),
            model: "lfm2.5-24b-a2b".into(),
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
    ]
}

/// A roster plus its optional pin. The chain order is the compiled-in
/// declared order; the config file overrides entries per-field, never the
/// order (slice 6's calibration reorders candidates over time).
#[derive(Debug, Clone)]
pub struct Roster {
    pub candidates: Vec<Candidate>,
    /// When set, this provider id is pinned: selection skips sys1 judgment
    /// and routes straight to it (the `pinned` progress-line mode).
    pub pin: Option<String>,
}

/// Why a config file could not be used. A typed, named value: an invalid
/// config fails startup loud rather than being silently ignored (design 7).
#[derive(Debug)]
pub enum ConfigError {
    /// The file exists but could not be read.
    Read(io::Error),
    /// The file is not valid JSON (or not the expected shape).
    Parse(String),
    /// An override that cannot be applied (an unknown id that is not fully
    /// specified, so there is nothing to merge it onto).
    BadEntry(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Read(error) => write!(f, "could not read the routing config: {error}"),
            ConfigError::Parse(detail) => {
                write!(f, "the routing config is not valid JSON: {detail}")
            }
            ConfigError::BadEntry(detail) => write!(f, "the routing config has a bad entry: {detail}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// The `<FLEET_HOME>/config.json` shape. Every field is optional; the file
/// overrides the compiled-in defaults per entry (merge, not replace).
#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    pin: Option<String>,
    #[serde(default)]
    providers: Vec<ProviderEntry>,
    #[serde(default)]
    calib: Option<CalibEntry>,
    #[serde(default)]
    skills: Option<SkillsEntry>,
}

/// One provider override in the config file. Any subset of fields; a bare
/// `{"id": …}` toggles nothing but still addresses the entry. Unknown
/// fields are rejected, not ignored: a misspelled override (`keyenv`)
/// fails the load the way a misspelled id does — loud beats silent for a
/// hand-written config.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderEntry {
    pub id: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub key_env: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub key_required: Option<bool>,
    #[serde(default)]
    pub disabled_reason: Option<String>,
}

impl ProviderEntry {
    /// This entry's id, the key every merge and every surface addresses.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Validate one config proposal's provider entry against the roster's
/// own schema — the same `deny_unknown_fields` struct [`load_roster`]
/// reads, so a proposal cannot introduce a field the roster would
/// refuse and a misspelled key fails here, by name, before anything is
/// written. Slice 12's gate calls this before applying a `config`
/// proposal; the refusal names the offending field the way the roster's
/// own parse error does.
pub fn validate_provider_entry(value: &serde_json::Value) -> Result<ProviderEntry, String> {
    serde_json::from_value::<ProviderEntry>(value.clone())
        .map_err(|error| format!("the config entry is not a provider entry: {error}"))
}

/// The `calib` section: where the calibration stream lives when
/// dev-decisions' own default is not where it should be. Unknown fields
/// are rejected for the same reason a provider's are.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibEntry {
    #[serde(default)]
    log_root: Option<String>,
}

/// The `skills` section: the two curation knobs. Both are optional; a
/// missing file, a missing section, or a value this reader cannot use
/// falls back to the compiled-in defaults, exactly the way
/// [`calib_log_root`] does.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillsEntry {
    /// How many sessions a skill may go unused across before curation
    /// proposes retiring it.
    #[serde(default)]
    retire_after_sessions: Option<usize>,
    /// The description similarity at which two skills read as
    /// near-identical.
    #[serde(default)]
    merge_similarity: Option<f64>,
}

/// The resolved skills knobs, with the defaults applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillsConfig {
    pub retire_after_sessions: usize,
    pub merge_similarity: f64,
}

impl Default for SkillsConfig {
    fn default() -> SkillsConfig {
        SkillsConfig {
            retire_after_sessions: crate::skills::DEFAULT_RETIRE_AFTER_SESSIONS,
            merge_similarity: crate::skills::MERGE_SIMILARITY_THRESHOLD,
        }
    }
}

/// Load the roster: the compiled-in defaults, then `<home>/config.json`
/// merged over them per entry. A missing file is the defaults; a present
/// but invalid file is a typed [`ConfigError`].
pub fn load_roster(home: &Path) -> Result<Roster, ConfigError> {
    let path = home.join("config.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Roster { candidates: default_roster(), pin: None });
        }
        Err(error) => return Err(ConfigError::Read(error)),
    };
    let file: ConfigFile = serde_json::from_str(&text)
        .map_err(|error| ConfigError::Parse(format!("{}: {error}", path.display())))?;
    let (candidates, pin) = merge(default_roster(), &file)?;
    Ok(Roster { candidates, pin })
}

/// The config file's calibration log root, when it names one. Read
/// separately from [`load_roster`] and best-effort: `fleet calib` and the
/// demotion reader both want the root without a roster, and a config file
/// too broken to parse has already failed the roster load loud. See
/// [`calib::resolve_root`] for the whole resolution order.
pub fn calib_log_root(home: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(home.join("config.json")).ok()?;
    let file: ConfigFile = serde_json::from_str(&text).ok()?;
    file.calib
        .and_then(|calib| calib.log_root)
        .map(|root| root.trim().to_string())
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
}

/// The skills knobs as `<home>/config.json` names them, with the
/// compiled-in defaults for everything the file does not. Best-effort by
/// design, like [`calib_log_root`]: a config too broken to parse has
/// already failed the roster load loud, and this reader is for the
/// surfaces that want the knobs without a roster.
///
/// A `merge_similarity` outside 0..1 is clamped into range rather than
/// rejected: a similarity is a ratio by definition, and clamping keeps
/// the curation honest instead of silently disabling it.
pub fn skills_config(home: &Path) -> SkillsConfig {
    let mut config = SkillsConfig::default();
    let Ok(text) = std::fs::read_to_string(home.join("config.json")) else { return config };
    let Ok(file) = serde_json::from_str::<ConfigFile>(&text) else { return config };
    let Some(skills) = file.skills else { return config };
    if let Some(sessions) = skills.retire_after_sessions {
        config.retire_after_sessions = sessions;
    }
    if let Some(similarity) = skills.merge_similarity {
        config.merge_similarity = similarity.clamp(0.0, 1.0);
    }
    config
}

/// Merge `file` over `candidates`, per entry by id. An override on a known
/// id patches the fields it names; an override on an unknown id is added
/// only when it is fully specified (else [`ConfigError::BadEntry`]).
fn merge(
    mut candidates: Vec<Candidate>,
    file: &ConfigFile,
) -> Result<(Vec<Candidate>, Option<String>), ConfigError> {
    for entry in &file.providers {
        if let Some(existing) = candidates.iter_mut().find(|candidate| candidate.id == entry.id) {
            if let Some(value) = &entry.base_url {
                existing.base_url = value.clone();
            }
            if let Some(value) = &entry.key_env {
                existing.key_env = value.clone();
            }
            if let Some(value) = &entry.model {
                existing.model = value.clone();
            }
            if let Some(value) = entry.enabled {
                existing.enabled = value;
            }
            if let Some(value) = entry.key_required {
                existing.key_required = value;
            }
            if let Some(value) = &entry.disabled_reason {
                existing.disabled_reason = Some(value.clone());
            }
        } else {
            match (&entry.base_url, &entry.key_env, &entry.model) {
                (Some(base_url), Some(key_env), Some(model)) => candidates.push(Candidate {
                    id: entry.id.clone(),
                    base_url: base_url.clone(),
                    key_env: key_env.clone(),
                    model: model.clone(),
                    enabled: entry.enabled.unwrap_or(true),
                    key_required: entry.key_required.unwrap_or(true),
                    disabled_reason: entry.disabled_reason.clone(),
                }),
                _ => {
                    return Err(ConfigError::BadEntry(format!(
                        "provider `{}` is not a built-in roster entry and does not name \
                         base_url, key_env, and model, so there is nothing to merge it onto",
                        entry.id
                    )))
                }
            }
        }
    }
    Ok((candidates, file.pin.clone()))
}

/// A roster candidate with its key resolved for this run.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub candidate: Candidate,
    /// The resolved key value, or `None` when its env var is unset. Kept in
    /// memory only; never logged or serialized into a decision row.
    pub key: Option<String>,
}

impl Resolved {
    /// This candidate's id.
    pub fn id(&self) -> &str {
        &self.candidate.id
    }

    /// Whether a run may attempt this candidate: enabled, and either it has
    /// a key or it does not require one.
    pub fn usable(&self) -> bool {
        self.candidate.enabled && (self.key.is_some() || !self.candidate.key_required)
    }
}

/// Resolve one key env-var name the same way the rest of Fleet resolves a
/// setting: the process environment, then `<home>/.env`, then the DeepSeek
/// Harness credential store's `refs` block (read-only). One stored key
/// serves every candidate whose `key_env` names it.
fn resolve_key(name: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    crate::home::resolve_setting(name, dotenv).or_else(|| {
        let user_home = std::env::var("HOME").ok();
        let path = crate::home::harness_credentials_path(
            std::env::var("DSH_HOME").ok().as_deref(),
            user_home.as_deref(),
        )?;
        let text = std::fs::read_to_string(path).ok()?;
        crate::home::harness_refs(&text, name)
    })
}

/// Resolve every candidate's key for this run. `dotenv` is the
/// `<home>/.env` map the caller already read.
pub fn resolve(candidates: &[Candidate], dotenv: &HashMap<String, String>) -> Vec<Resolved> {
    candidates
        .iter()
        .map(|candidate| Resolved {
            candidate: candidate.clone(),
            key: resolve_key(&candidate.key_env, dotenv),
        })
        .collect()
}

/// The declared HEAD's index: the first enabled candidate. Normally the
/// router kit at index 0; the next enabled one if the kit is disabled.
fn head_index(resolved: &[Resolved]) -> usize {
    resolved.iter().position(|entry| entry.candidate.enabled).unwrap_or(0)
}

// --- selection --------------------------------------------------------------

/// How the route was chosen, for the progress line, the TUI status bar,
/// and the decision row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// sys1 judged the workload and it mapped to a usable candidate.
    Judged,
    /// No usable judgment (sys1 absent/failed, an unmapped workload, or a
    /// mapped candidate with no key); the declared HEAD was used.
    Fallback,
    /// A configured pin bypassed judgment.
    Pinned,
}

impl How {
    /// The word each mode renders with.
    pub fn label(self) -> &'static str {
        match self {
            How::Judged => "judged",
            How::Fallback => "fallback",
            How::Pinned => "pinned",
        }
    }
}

/// One routing decision, made once per session before the run.
#[derive(Debug, Clone)]
pub struct RouteDecision {
    /// The selected candidate id.
    pub provider: String,
    pub how: How,
    /// sys1's judged workload, when there was one.
    pub workload: Option<String>,
    /// sys1's verdict (`pass`/`escalate`/`block`/`error`), when there was one.
    pub verdict: Option<String>,
    /// Why this route, in one line — the decision row and the status bar
    /// both read it.
    pub reason: String,
    /// The chain index failover begins walking from.
    pub start_index: usize,
}

/// The workload→candidate map, honoring the owner's "the kit is primary":
/// routine work (`quick`, `standard_code`) routes through the primary smart
/// router, and the specialized plans are the judged targets for the
/// workloads that benefit from a direct route. This is the one genuinely
/// tunable policy in the slice; it is ported from the kit's own tier table
/// with the owner's primary-route directive applied.
fn workload_candidate(workload: &str) -> Option<&'static str> {
    match workload {
        "quick" | "standard_code" => Some(HEAD_ID),
        "hard" => Some("zai-coding-plan"),
        "prose" => Some("xiaomi-mimo"),
        "deep_context" => Some("stepfun"),
        _ => None,
    }
}

/// Choose the route for `task`: a pin wins, else sys1 judges the workload,
/// else the declared HEAD. Never fails and never blocks a run — every
/// failure path is a named fallback. Returns the decision and how long the
/// judgment took (for the decision row's `latency_ms`).
pub async fn select(
    resolved: &[Resolved],
    pin: Option<&str>,
    task: &str,
    specs: &HashMap<String, ServerSpec>,
) -> (RouteDecision, u128) {
    let started = Instant::now();
    let head = head_index(resolved);
    let decision = if let Some(pin_id) = pin {
        match resolved.iter().position(|entry| entry.id() == pin_id && entry.usable()) {
            Some(index) => RouteDecision {
                provider: pin_id.to_string(),
                how: How::Pinned,
                workload: None,
                verdict: None,
                reason: format!("pinned to {pin_id}"),
                start_index: index,
            },
            None => fallback(resolved, head, &format!("pin `{pin_id}` is not usable; using the head")),
        }
    } else {
        match judge(task, specs).await {
            Some((workload, verdict)) => {
                match workload_candidate(&workload)
                    .and_then(|id| resolved.iter().position(|entry| entry.id() == id))
                {
                    Some(index) if resolved[index].usable() => RouteDecision {
                        provider: resolved[index].id().to_string(),
                        how: How::Judged,
                        workload: Some(workload.clone()),
                        verdict: Some(verdict),
                        reason: format!("sys1 judged `{workload}`"),
                        start_index: index,
                    },
                    Some(index) => fallback(
                        resolved,
                        head,
                        &format!("sys1 judged `{workload}` but `{}` has no key; using the head", resolved[index].id()),
                    ),
                    None => fallback(
                        resolved,
                        head,
                        &format!("sys1 judged an unmapped workload `{workload}`; using the head"),
                    ),
                }
            }
            None => fallback(resolved, head, "sys1 gave no usable verdict; using the head"),
        }
    };
    (decision, started.elapsed().as_millis())
}

/// A fallback decision to the declared HEAD.
fn fallback(resolved: &[Resolved], head: usize, reason: &str) -> RouteDecision {
    let provider = resolved.get(head).map(Resolved::id).unwrap_or(HEAD_ID).to_string();
    RouteDecision {
        provider,
        how: How::Fallback,
        workload: None,
        verdict: None,
        reason: reason.to_string(),
        start_index: head,
    }
}

/// Ask sys1 for a routing judgment: spawn the lane (like any other), call
/// `sys1_classify` with task `route`, and read the workload verdict. Any
/// failure — the lane cannot launch, the handshake or call fails, the
/// answer has no workload — is `None`, which the caller turns into a
/// fallback. The child is always torn down.
async fn judge(task: &str, specs: &HashMap<String, ServerSpec>) -> Option<(String, String)> {
    let spec = match specs.get(JUDGE_LANE) {
        Some(spec) => spec.clone(),
        None => lanes::spec_for(JUDGE_LANE).ok()?,
    };
    let server = McpServer::spawn(&spec).ok()?;
    let outcome = judge_live(&server, task).await;
    let _ = server.close().await;
    outcome
}

/// The live judgment over an already-spawned sys1 server. The call shape
/// is the tool's own schema (`tools/list`): `sys1_classify` takes `text`
/// (required) and `task` (the registered task name).
async fn judge_live(server: &McpServer, task: &str) -> Option<(String, String)> {
    server.initialize().await.ok()?;
    let call = server
        .call_tool("sys1_classify", json!({ "task": JUDGE_TASK, "text": task }))
        .await
        .ok()?;
    parse_workload(&call.text())
}

/// Read the workload verdict out of a `sys1_classify` answer. The answer's
/// text is a JSON object whose `answers` maps provider id → head id →
/// answer; the routing signal is the `workload` head's `label`. The
/// top-level `verdict` rides along for the decision row. A missing or
/// empty workload label is `None` (an unusable verdict → fallback).
fn parse_workload(text: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let verdict = value.get("verdict").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let answers = value.get("answers")?.as_object()?;
    for heads in answers.values() {
        if let Some(label) = heads
            .get("workload")
            .and_then(|workload| workload.get("label"))
            .and_then(|label| label.as_str())
            .filter(|label| !label.is_empty())
        {
            return Some((label.to_string(), verdict));
        }
    }
    None
}

// --- failover ---------------------------------------------------------------

/// One attempt recorded during a walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Attempt {
    pub provider: String,
    /// `served`, `failed`, or `skipped`.
    pub outcome: &'static str,
    /// The failure class, when the attempt failed; `no-key` when skipped.
    pub class: Option<&'static str>,
}

/// Map a client error onto a failure class. 400/404 are client errors —
/// every candidate would fail them the same way — so they do not failover;
/// everything else does.
pub fn failure_class(error: &ClientError) -> &'static str {
    match error {
        ClientError::Api(api) => match api.status {
            401 => "auth",
            402 => "quota",
            403 => "forbidden",
            408 | 425 | 504 => "timeout",
            429 => "rate-limit",
            500..=599 => "server",
            400 | 404 => "client-error",
            _ => "api",
        },
        ClientError::Network(message) => {
            let lowered = message.to_lowercase();
            if lowered.contains("timeout") || lowered.contains("timed out") {
                "timeout"
            } else {
                "network"
            }
        }
        ClientError::Decode(_) => "decode",
    }
}

/// Whether a failure class advances the walk. Everything but a client error
/// (400/404) does.
fn failover_eligible_class(class: &str) -> bool {
    class != "client-error"
}

/// Whether `error` advances the failover walk to the next candidate.
fn failover_eligible(error: &ClientError) -> bool {
    failover_eligible_class(failure_class(error))
}

/// One candidate's scripted outcome for the pure walk test.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkOutcome {
    /// Skipped before any request: disabled or no key.
    Unusable,
    /// The candidate served a complete answer.
    Served,
    /// The candidate failed with `class`; `streamed` is whether any token
    /// crossed first (a mid-stream failure is fatal, not a failover).
    Failed { class: &'static str, streamed: bool },
}

/// The pure failover decision — the spec [`Router::chat_stream`] implements.
/// From `start`: skip unusable candidates (recorded as `no-key` skips), land
/// on the first served candidate, stop on a streamed failure or a
/// non-failover class, else advance. Returns the index that served (if any)
/// and the recorded attempts. Testing it here keeps the decision honest
/// without a socket; the integration test drives the real HTTP walk.
#[cfg(test)]
fn simulate_walk(start: usize, outcomes: &[WalkOutcome]) -> (Option<usize>, Vec<Attempt>) {
    let mut attempts = Vec::new();
    for (offset, outcome) in outcomes.iter().enumerate() {
        let index = start + offset;
        let provider = format!("cand-{index}");
        match outcome {
            WalkOutcome::Unusable => attempts.push(Attempt {
                provider,
                outcome: "skipped",
                class: Some("no-key"),
            }),
            WalkOutcome::Served => {
                attempts.push(Attempt { provider, outcome: "served", class: None });
                return (Some(index), attempts);
            }
            WalkOutcome::Failed { class, streamed } => {
                attempts.push(Attempt { provider, outcome: "failed", class: Some(class) });
                if *streamed || !failover_eligible_class(class) {
                    return (None, attempts);
                }
            }
        }
    }
    (None, attempts)
}

/// The router's mutable state, behind a mutex so the shared `Client` can
/// advance it with `&self`.
struct RouterState {
    /// The candidate the next walk starts from (the committed one once a
    /// candidate has served).
    index: usize,
    /// True once a candidate has served this run: locked in, no restart.
    committed: bool,
    /// The index that served, when one did.
    served: Option<usize>,
    /// Every attempt across every turn, in order.
    attempts: Vec<Attempt>,
}

/// Walks the roster with failover. Built once per session from the resolved
/// roster and the selection decision; the run's [`Client`] holds it behind
/// an `Arc` and every streamed request crosses [`Router::chat_stream`].
pub struct Router {
    /// One plain client per candidate (no router of its own), reused across
    /// turns so each gateway's connection pool survives.
    clients: Vec<Client>,
    resolved: Vec<Resolved>,
    decision: RouteDecision,
    state: Mutex<RouterState>,
}

impl Router {
    /// A router over `resolved`, starting its walk at the decision's index.
    pub fn new(resolved: Vec<Resolved>, decision: RouteDecision) -> Router {
        let clients = resolved
            .iter()
            .map(|entry| {
                Client::new(entry.candidate.base_url.as_str(), entry.key.clone().unwrap_or_default())
            })
            .collect();
        let index = decision.start_index.min(resolved.len().saturating_sub(1));
        Router {
            clients,
            resolved,
            decision,
            state: Mutex::new(RouterState { index, committed: false, served: None, attempts: Vec::new() }),
        }
    }

    /// The initial selection decision (for the headless line and the TUI
    /// status bar).
    pub fn decision(&self) -> RouteDecision {
        self.decision.clone()
    }

    /// The assembled decision row after the run, with every attempt and the
    /// final choice. `latency_ms` is the selection judgment's cost.
    pub fn row(&self, task: &str, latency_ms: u128) -> calib::RouteRow {
        let state = lock(&self.state);
        let served_index = state.served;
        let provider = served_index
            .and_then(|index| self.resolved.get(index))
            .or_else(|| self.resolved.get(self.decision.start_index))
            .map(Resolved::id)
            .unwrap_or(HEAD_ID)
            .to_string();
        let model = served_index
            .and_then(|index| self.resolved.get(index))
            .map(|entry| entry.candidate.model.clone())
            .unwrap_or_default();
        let providers_used: Vec<String> = state
            .attempts
            .iter()
            .filter(|attempt| attempt.outcome != "skipped")
            .map(|attempt| attempt.provider.clone())
            .collect();
        let ts = session::iso_utc_stamp(SystemTime::now()).unwrap_or_default();
        calib::RouteRow {
            ts,
            provider,
            how: self.decision.how.label().to_string(),
            workload: self.decision.workload.clone(),
            verdict: self.decision.verdict.clone(),
            reason: self.decision.reason.clone(),
            model,
            providers_used,
            attempts: state.attempts.clone(),
            input_chars: task.chars().count(),
            input_hash: calib::hash16(task),
            latency_ms,
        }
    }

    /// One streamed request, routed with failover. The behavior of a single
    /// candidate's stream is exactly `Client::chat_stream`'s; this only
    /// chooses which candidate a request crosses and walks the chain on a
    /// failover class before any token has streamed.
    pub async fn chat_stream(
        &self,
        request: &ChatRequest<'_>,
        on_event: &mut (dyn FnMut(StreamEvent) + '_),
    ) -> Result<ChatResponse, ClientError> {
        let (start, committed) = {
            let state = lock(&self.state);
            (state.index, state.committed)
        };
        // Locked in: one attempt, and any failure is fatal — no silent
        // restart once a candidate has served this run.
        if committed {
            return match self.one_attempt(start, request, on_event).await {
                Ok(response) => Ok(response),
                Err((error, _streamed)) => {
                    let class = failure_class(&error);
                    self.record(Attempt {
                        provider: self.resolved[start].id().to_string(),
                        outcome: "failed",
                        class: Some(class),
                    });
                    Err(error)
                }
            };
        }

        let mut last_error: Option<ClientError> = None;
        for index in start..self.resolved.len() {
            if !self.resolved[index].candidate.enabled {
                self.record(Attempt {
                    provider: self.resolved[index].id().to_string(),
                    outcome: "skipped",
                    class: Some("disabled"),
                });
                continue;
            }
            if !self.resolved[index].usable() {
                self.record(Attempt {
                    provider: self.resolved[index].id().to_string(),
                    outcome: "skipped",
                    class: Some("no-key"),
                });
                continue;
            }
            match self.one_attempt(index, request, on_event).await {
                Ok(response) => {
                    self.record(Attempt {
                        provider: self.resolved[index].id().to_string(),
                        outcome: "served",
                        class: None,
                    });
                    let mut state = lock(&self.state);
                    state.index = index;
                    state.committed = true;
                    state.served = Some(index);
                    return Ok(response);
                }
                Err((error, streamed)) => {
                    let class = failure_class(&error);
                    self.record(Attempt {
                        provider: self.resolved[index].id().to_string(),
                        outcome: "failed",
                        class: Some(class),
                    });
                    // A candidate that started streaming is committed: a
                    // mid-stream failure fails the run typed.
                    if streamed {
                        return Err(error);
                    }
                    if !failover_eligible(&error) {
                        return Err(error);
                    }
                    last_error = Some(error);
                }
            }
        }
        Err(last_error
            .unwrap_or_else(|| ClientError::Network("no usable roster candidate remained".to_string())))
    }

    /// One candidate's attempt: build the request with the candidate's
    /// model, wrap `on_event` so the first token marks the stream
    /// committed, and return the response or the error plus whether it
    /// streamed.
    async fn one_attempt(
        &self,
        index: usize,
        request: &ChatRequest<'_>,
        on_event: &mut (dyn FnMut(StreamEvent) + '_),
    ) -> Result<ChatResponse, (ClientError, bool)> {
        let routed = ChatRequest {
            model: self.resolved[index].candidate.model.as_str(),
            messages: request.messages,
            tools: request.tools.clone(),
        };
        let streamed = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&streamed);
        let result = self.clients[index]
            .chat_stream_plain(&routed, |event| {
                flag.store(true, Ordering::Relaxed);
                on_event(event);
            })
            .await;
        match result {
            Ok(response) => Ok(response),
            Err(error) => Err((error, streamed.load(Ordering::Relaxed))),
        }
    }

    /// Record one attempt under the state lock.
    fn record(&self, attempt: Attempt) {
        lock(&self.state).attempts.push(attempt);
    }
}


/// A route prepared for a session: the router, its decision, the
/// judgment's latency, and the calibration stream this session writes to.
pub struct Prepared {
    pub router: Arc<Router>,
    pub decision: RouteDecision,
    pub latency_ms: u128,
    /// The calibration stream: this session's rows go here, and its first
    /// read is what demoted the chain.
    pub calib: calib::Stream,
}

impl Prepared {
    /// Record this session's routing decision to the calibration stream,
    /// best-effort — a full queue drops the row, never the run.
    pub fn record_route(&self, task: &str) {
        self.calib.record(calib::Row::Route(self.router.row(task, self.latency_ms)));
    }
}

/// Resolve the roster, judge the route for `task`, and build the router.
/// A bad config file is a typed [`ConfigError`] the caller turns into a
/// startup failure; every other outcome (sys1 absent, an unmapped verdict,
/// a keyless candidate) is a named fallback, never a blocked run.
pub async fn prepare(
    home: &Path,
    dotenv: &HashMap<String, String>,
    task: &str,
    specs: &HashMap<String, ServerSpec>,
) -> Result<Prepared, ConfigError> {
    let roster = load_roster(home)?;
    let resolved = resolve(&roster.candidates, dotenv);
    let (mut decision, latency_ms) = select(&resolved, roster.pin.as_deref(), task, specs).await;
    // The calibration stream, and its first read: today's graded rows
    // demote a candidate that keeps failing to the end of this session's
    // walk (slice 5's last acceptance criterion).
    let calib = calib::Stream::open(calib::resolve_root(
        home,
        std::env::var("HOME").ok().as_deref(),
    ));
    let demoted = calib::demotions(
        calib.root(),
        &calib::today(),
        &resolved.iter().map(Resolved::id).collect::<Vec<_>>(),
    );
    let resolved = reorder_for_demotion(resolved, &decision.provider, &demoted);
    if !demoted.is_empty() {
        // The demotion is auditable: it rides in the decision row's
        // reason, so a later reader sees why the walk was ordered as it
        // was rather than having to infer it.
        decision.reason.push_str(&format!(
            "; demoted {} ({} consecutive failures today)",
            demoted.join(", "),
            calib::DEMOTION_THRESHOLD
        ));
    }
    decision.start_index =
        resolved.iter().position(|entry| entry.id() == decision.provider).unwrap_or(0);
    let router = Arc::new(Router::new(resolved, decision.clone()));
    Ok(Prepared { router, decision, latency_ms, calib })
}

/// Reorder the chain for this session's walk: the selected candidate keeps
/// the head of the walk — sys1's judgment is untouched — every other
/// candidate keeps its declared order, and the demoted ones move to the
/// end. A candidate that is both selected and demoted stays at the head:
/// demotion reorders the walk, it does not override the selection.
pub fn reorder_for_demotion(
    resolved: Vec<Resolved>,
    selected: &str,
    demoted: &[String],
) -> Vec<Resolved> {
    let (mut demoted_entries, mut kept): (Vec<Resolved>, Vec<Resolved>) = resolved
        .into_iter()
        .partition(|entry| demoted.iter().any(|id| id == entry.id()) && entry.id() != selected);
    let mut ordered = Vec::with_capacity(kept.len() + demoted_entries.len());
    if let Some(index) = kept.iter().position(|entry| entry.id() == selected) {
        ordered.push(kept.remove(index));
    }
    ordered.append(&mut kept);
    ordered.append(&mut demoted_entries);
    ordered
}

/// Lock a mutex, recovering from poisoning the way the MCP client does: a
/// panic while the lock was held must not turn every later call into
/// another panic.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| {
        eprintln!("fleet: a routing mutex was poisoned by a panic; recovering the value it guards");
        PoisonError::into_inner(poison)
    })
}

// --- the `fleet providers` surface ------------------------------------------

/// Where one provider stands for the health command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderState {
    /// Enabled and servable now (key present, or key not required).
    Ready,
    /// Enabled but its key is absent and required.
    NoKey,
    /// Disabled in the roster or config.
    Disabled,
}

impl ProviderState {
    fn dot(self) -> &'static str {
        match self {
            ProviderState::Ready => "\u{2713}",
            ProviderState::NoKey => "\u{25d0}",
            ProviderState::Disabled => "\u{25cb}",
        }
    }

    fn label(self) -> &'static str {
        match self {
            ProviderState::Ready => "configured",
            ProviderState::NoKey => "no key",
            ProviderState::Disabled => "disabled",
        }
    }
}

/// The state of one resolved candidate.
pub fn provider_state(entry: &Resolved) -> ProviderState {
    if !entry.candidate.enabled {
        ProviderState::Disabled
    } else if entry.key.is_some() || !entry.candidate.key_required {
        ProviderState::Ready
    } else {
        ProviderState::NoKey
    }
}

/// The ANSI palette, mirroring `lanes`' so the two health surfaces read as
/// one.
mod palette {
    pub const ACCENT: &str = "\u{1b}[36m";
    pub const OK: &str = "\u{1b}[32m";
    pub const WARN: &str = "\u{1b}[33m";
    pub const DIM: &str = "\u{1b}[90m";
    pub const TEXT: &str = "\u{1b}[37m";
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

/// The card width, matching `lanes`.
const CARD_WIDTH: usize = 72;

/// Render the roster as cards, one per candidate, in declared chain order:
/// a status dot, the endpoint, the key env-var NAME with its presence
/// (never the value), the model, and the head marked. Never a plain-text
/// table.
pub fn render(resolved: &[Resolved]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet providers{RESET} {DIM}\u{2014} the routing roster, in declared chain order{RESET}\n\n",
        BOLD = palette::BOLD,
        RESET = palette::RESET,
        DIM = palette::DIM,
    ));
    for (index, entry) in resolved.iter().enumerate() {
        out.push_str(&render_card(entry, index == 0));
    }
    let ready = resolved.iter().filter(|entry| provider_state(entry) == ProviderState::Ready).count();
    out.push_str(&format!(
        "\n{DIM}{} providers \u{b7} {ready} configured \u{b7} {} not ready \u{b7} head {HEAD_ID}{RESET}\n",
        resolved.len(),
        resolved.len() - ready,
        DIM = palette::DIM,
        RESET = palette::RESET,
    ));
    out
}

/// One provider's card.
fn render_card(entry: &Resolved, is_head: bool) -> String {
    let state = provider_state(entry);
    let dot_color = match state {
        ProviderState::Ready => palette::OK,
        ProviderState::NoKey => palette::WARN,
        ProviderState::Disabled => palette::DIM,
    };
    let mut header_right = format!("{} {}", state.dot(), state.label());
    if is_head {
        header_right.push_str(&format!(" {}\u{2605} head{}", palette::ACCENT, palette::RESET));
    }
    let header = format!(
        "{DIM}\u{250c}\u{2500}{RESET} {BOLD}{ACCENT}{id}{RESET} {DIM}{dashes} {dot_color}{right}{RESET} {DIM}\u{2510}{RESET}",
        DIM = palette::DIM,
        RESET = palette::RESET,
        BOLD = palette::BOLD,
        ACCENT = palette::ACCENT,
        id = entry.candidate.id,
        dashes = dashes_for(&format!(" {} ", entry.candidate.id), &plain(&header_right)),
        dot_color = dot_color,
        right = header_right,
    );
    let key_state = match (&entry.key, entry.candidate.key_required) {
        (Some(_), _) => "set".to_string(),
        (None, false) => "absent (optional)".to_string(),
        (None, true) => "absent".to_string(),
    };
    let mut lines = vec![
        format!("{DIM}\u{2502}{RESET} {TEXT}{url}{RESET}", DIM = palette::DIM, RESET = palette::RESET, TEXT = palette::TEXT, url = entry.candidate.base_url),
        format!(
            "{DIM}\u{2502}{RESET} {TEXT}key {name} \u{b7} {state} \u{b7} model {model}{RESET}",
            DIM = palette::DIM,
            RESET = palette::RESET,
            TEXT = palette::TEXT,
            name = entry.candidate.key_env,
            state = key_state,
            model = entry.candidate.model,
        ),
    ];
    if let Some(reason) = &entry.candidate.disabled_reason {
        lines.push(format!("{DIM}\u{2502}{RESET} {DIM}{reason}{RESET}", DIM = palette::DIM, RESET = palette::RESET, reason = reason));
    }
    let bottom = format!("{DIM}\u{2514}{bottom}\u{2518}{RESET}", DIM = palette::DIM, RESET = palette::RESET, bottom = "\u{2500}".repeat(CARD_WIDTH - 2));
    let mut card = format!("{header}\n");
    for line in lines {
        card.push_str(&line);
        card.push('\n');
    }
    card.push_str(&bottom);
    card.push('\n');
    card
}

/// The dash run that pads a card header. Counts characters, exact for the
/// ASCII ids and labels used here.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "\u{2500}".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// Strip ANSI escapes for a width measurement (the header's right edge is
/// measured without its color codes).
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            out.push(character);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(status: u16) -> ClientError {
        ClientError::Api(crate::api::ApiError { status, message: "x".to_string() })
    }

    // --- the failure-class table ---

    #[test]
    fn the_failure_class_table_names_each_status() {
        assert_eq!(failure_class(&api(401)), "auth");
        assert_eq!(failure_class(&api(402)), "quota");
        assert_eq!(failure_class(&api(403)), "forbidden");
        assert_eq!(failure_class(&api(429)), "rate-limit");
        assert_eq!(failure_class(&api(500)), "server");
        assert_eq!(failure_class(&api(503)), "server");
        assert_eq!(failure_class(&api(504)), "timeout");
        assert_eq!(failure_class(&ClientError::Network("connect refused".to_string())), "network");
        assert_eq!(failure_class(&ClientError::Network("the timeout expired".to_string())), "timeout");
        assert_eq!(failure_class(&ClientError::Decode("bad sse".to_string())), "decode");
    }

    #[test]
    fn every_failure_class_but_a_client_error_fails_over() {
        for status in [401u16, 402, 403, 408, 429, 500, 502, 503, 504] {
            assert!(failover_eligible(&api(status)), "{status} must fail over");
        }
        for status in [400u16, 404] {
            assert!(!failover_eligible(&api(status)), "{status} must not fail over");
        }
        assert!(failover_eligible(&ClientError::Network("refused".to_string())));
        assert!(failover_eligible(&ClientError::Decode("garbage".to_string())));
    }

    // --- the chain walk ---

    #[test]
    fn the_walk_lands_on_the_first_healthy_candidate_and_records_attempts() {
        // head 402, then 429, then 500, then the fourth serves.
        let (served, attempts) = simulate_walk(
            0,
            &[
                WalkOutcome::Failed { class: "quota", streamed: false },
                WalkOutcome::Failed { class: "rate-limit", streamed: false },
                WalkOutcome::Failed { class: "server", streamed: false },
                WalkOutcome::Served,
            ],
        );
        assert_eq!(served, Some(3));
        assert_eq!(attempts.len(), 4);
        assert_eq!(attempts[0].outcome, "failed");
        assert_eq!(attempts[0].class, Some("quota"));
        assert_eq!(attempts[3].outcome, "served");
        assert_eq!(attempts[3].provider, "cand-3");
    }

    #[test]
    fn the_walk_skips_unusable_candidates_and_keeps_walking() {
        let (served, attempts) = simulate_walk(
            1,
            &[
                WalkOutcome::Unusable,
                WalkOutcome::Failed { class: "auth", streamed: false },
                WalkOutcome::Served,
            ],
        );
        assert_eq!(served, Some(3));
        assert_eq!(attempts[0].outcome, "skipped");
        assert_eq!(attempts[0].class, Some("no-key"));
        assert_eq!(attempts[2].outcome, "served");
    }

    #[test]
    fn a_mid_stream_failure_is_fatal_and_stops_the_walk() {
        let (served, attempts) = simulate_walk(
            0,
            &[
                WalkOutcome::Failed { class: "network", streamed: true },
                WalkOutcome::Served,
            ],
        );
        // The first candidate streamed, then died: no silent restart.
        assert_eq!(served, None);
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].class, Some("network"));
    }

    #[test]
    fn a_client_error_stops_the_walk_without_failing_over() {
        let (served, attempts) = simulate_walk(
            0,
            &[WalkOutcome::Failed { class: "client-error", streamed: false }, WalkOutcome::Served],
        );
        assert_eq!(served, None);
        assert_eq!(attempts.len(), 1);
    }

    #[test]
    fn an_exhausted_walk_serves_no_one() {
        let (served, attempts) =
            simulate_walk(0, &[WalkOutcome::Failed { class: "quota", streamed: false }; 3].as_slice());
        assert_eq!(served, None);
        assert_eq!(attempts.len(), 3);
    }

    // --- the sys1 verdict → candidate mapping ---

    #[test]
    fn the_workload_map_routes_the_kit_primary_and_the_plans_direct() {
        assert_eq!(workload_candidate("quick"), Some(HEAD_ID));
        assert_eq!(workload_candidate("standard_code"), Some(HEAD_ID));
        assert_eq!(workload_candidate("hard"), Some("zai-coding-plan"));
        assert_eq!(workload_candidate("prose"), Some("xiaomi-mimo"));
        assert_eq!(workload_candidate("deep_context"), Some("stepfun"));
        assert_eq!(workload_candidate("something_else"), None);
    }

    #[test]
    fn a_usable_verdict_maps_to_its_candidate() {
        let workload = parse_workload(
            r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.9}}}}"#,
        );
        assert_eq!(workload, Some(("hard".to_string(), "pass".to_string())));
    }

    #[test]
    fn an_error_or_absent_verdict_is_unusable() {
        // No provider succeeded: verdict error, no workload answer.
        assert_eq!(parse_workload(r#"{"verdict":"error","answers":{}}"#), None);
        // A declined workload head carries no label.
        assert_eq!(
            parse_workload(r#"{"verdict":"pass","answers":{"glide":{"workload":{"_declined":"no-answer"}}}}"#),
            None
        );
        // Not JSON at all.
        assert_eq!(parse_workload("not json"), None);
    }

    #[tokio::test]
    async fn an_unusable_verdict_falls_back_to_the_head() {
        // A roster whose only keyed candidate is the deepseek reserve; the
        // judged `hard` maps to zai, which has no key, so it falls back.
        let resolved = vec![
            Resolved {
                candidate: Candidate {
                    id: "router-kit".into(),
                    base_url: "http://127.0.0.1:8300/v1".into(),
                    key_env: "AGNOSTIC_ROUTER_TOKEN".into(),
                    model: "auto".into(),
                    enabled: true,
                    key_required: false,
                    disabled_reason: None,
                },
                key: None,
            },
            Resolved {
                candidate: Candidate {
                    id: "zai-coding-plan".into(),
                    base_url: "https://api.z.ai".into(),
                    key_env: "ZAI_CODING_API_KEY".into(),
                    model: "GLM-5.3-Flash".into(),
                    enabled: true,
                    key_required: true,
                    disabled_reason: None,
                },
                key: None,
            },
        ];
        // An empty specs map with no vendored sys1 → the judge cannot run,
        // which is the fallback path exercised here.
        let (decision, _latency) = select(&resolved, None, "do a thing", &HashMap::new()).await;
        assert_eq!(decision.how, How::Fallback);
        assert_eq!(decision.provider, HEAD_ID);
        assert_eq!(decision.start_index, 0);
    }

    #[tokio::test]
    async fn a_pin_routes_straight_to_its_candidate() {
        let resolved = vec![
            Resolved {
                candidate: Candidate {
                    id: "router-kit".into(),
                    base_url: "http://127.0.0.1:8300/v1".into(),
                    key_env: "AGNOSTIC_ROUTER_TOKEN".into(),
                    model: "auto".into(),
                    enabled: true,
                    key_required: false,
                    disabled_reason: None,
                },
                key: None,
            },
            Resolved {
                candidate: Candidate {
                    id: "deepseek".into(),
                    base_url: "https://api.deepseek.com".into(),
                    key_env: "DEEPSEEK_API_KEY".into(),
                    model: "deepseek-chat".into(),
                    enabled: true,
                    key_required: true,
                    disabled_reason: None,
                },
                key: Some("sk-fake".to_string()),
            },
        ];
        let (decision, _latency) =
            select(&resolved, Some("deepseek"), "anything", &HashMap::new()).await;
        assert_eq!(decision.how, How::Pinned);
        assert_eq!(decision.provider, "deepseek");
        assert_eq!(decision.start_index, 1);
    }

    // --- the config merge ---

    #[test]
    fn the_config_file_overrides_entries_field_by_field() {
        let file: ConfigFile = serde_json::from_str(
            r#"{
                "pin": "deepseek",
                "providers": [
                    { "id": "router-kit", "enabled": false },
                    { "id": "zai-coding-plan", "model": "GLM-5.3-Pro" },
                    { "id": "deepseek", "base_url": "https://example.invalid/v1" }
                ]
            }"#,
        )
        .unwrap();
        let (candidates, pin) = merge(default_roster(), &file).unwrap();
        assert_eq!(pin.as_deref(), Some("deepseek"));
        // The router kit was disabled; everything else on it survived.
        let kit = candidates.iter().find(|c| c.id == "router-kit").unwrap();
        assert!(!kit.enabled);
        assert_eq!(kit.base_url, "http://127.0.0.1:8300/v1");
        // The zai model changed; its endpoint and key env did not.
        let zai = candidates.iter().find(|c| c.id == "zai-coding-plan").unwrap();
        assert_eq!(zai.model, "GLM-5.3-Pro");
        assert_eq!(zai.key_env, "ZAI_CODING_API_KEY");
        // The deepseek endpoint changed; its key env did not.
        let deepseek = candidates.iter().find(|c| c.id == "deepseek").unwrap();
        assert_eq!(deepseek.base_url, "https://example.invalid/v1");
        assert_eq!(deepseek.key_env, "DEEPSEEK_API_KEY");
    }

    #[test]
    fn a_fully_specified_new_entry_is_added() {
        let file: ConfigFile = serde_json::from_str(
            r#"{ "providers": [ { "id": "my-gateway", "base_url": "https://g.example/v1", "key_env": "MY_KEY", "model": "m1" } ] }"#,
        )
        .unwrap();
        let (candidates, _) = merge(default_roster(), &file).unwrap();
        let added = candidates.iter().find(|c| c.id == "my-gateway").expect("the new entry");
        assert_eq!(added.base_url, "https://g.example/v1");
        assert!(added.enabled);
        assert!(added.key_required);
    }

    #[test]
    fn a_partial_new_entry_is_a_named_error() {
        let file: ConfigFile = serde_json::from_str(
            r#"{ "providers": [ { "id": "my-gateway", "model": "m1" } ] }"#,
        )
        .unwrap();
        let error = merge(default_roster(), &file).unwrap_err();
        match error {
            ConfigError::BadEntry(detail) => {
                assert!(detail.contains("my-gateway"), "{detail}");
                assert!(detail.contains("base_url"), "{detail}");
            }
            other => panic!("expected a BadEntry error, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_config_file_is_the_defaults() {
        let home = std::env::temp_dir().join(format!("fleet-route-nocfg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&home);
        let roster = load_roster(&home).unwrap();
        assert_eq!(roster.candidates.len(), 7);
        assert_eq!(roster.candidates[0].id, HEAD_ID);
        assert_eq!(roster.candidates[5].id, "deepseek");
        // Liquid is declared after DeepSeek and enters no declared
        // position: it is available to the owner's config, and moving it
        // into the chain is that config's one-line change.
        assert_eq!(roster.candidates[6].id, "liquid");
        assert_eq!(roster.pin, None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_invalid_config_file_is_a_typed_named_error() {
        let home = std::env::temp_dir().join(format!("fleet-route-badcfg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&home);
        std::fs::write(home.join("config.json"), b"{ not json ").unwrap();
        match load_roster(&home) {
            Err(ConfigError::Parse(detail)) => assert!(detail.contains("config.json"), "{detail}"),
            other => panic!("expected a Parse error, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&home);
    }

    // --- the decision row ---

    #[test]
    fn the_decision_row_carries_the_devdec_vocabulary() {
        let resolved = resolve(&default_roster(), &HashMap::new());
        let decision = RouteDecision {
            provider: "zai-coding-plan".to_string(),
            how: How::Judged,
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            reason: "sys1 judged `hard`".to_string(),
            start_index: 1,
        };
        let router = Router::new(resolved, decision);
        lock(&router.state).attempts.extend([
            Attempt { provider: "router-kit".to_string(), outcome: "failed", class: Some("auth") },
            Attempt { provider: "zai-coding-plan".to_string(), outcome: "served", class: None },
        ]);
        lock(&router.state).served = Some(1);
        let row = router.row("refactor the parser", 812);
        // The row is serialized through the stream's tag, which is what
        // puts `op` in the JSON.
        let line = serde_json::to_string(&calib::Row::Route(row)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["op"], "route");
        assert_eq!(value["provider"], "zai-coding-plan");
        assert_eq!(value["how"], "judged");
        assert_eq!(value["workload"], "hard");
        assert_eq!(value["verdict"], "pass");
        assert_eq!(value["model"], "GLM-5.3-Flash");
        assert_eq!(value["latency_ms"], 812);
        // Both attempts ride along, in order, and the served one is named.
        assert_eq!(value["attempts"].as_array().unwrap().len(), 2);
        assert_eq!(value["attempts"][0]["provider"], "router-kit");
        assert_eq!(value["attempts"][0]["class"], "auth");
        assert_eq!(value["attempts"][1]["outcome"], "served");
        assert_eq!(value["providers_used"][0], "router-kit");
        assert_eq!(value["providers_used"][1], "zai-coding-plan");
        // The task text is never logged — only its length and a hash.
        assert!(!line.contains("refactor the parser"));
        assert_eq!(value["input_chars"], 19);
        assert!(value["input_hash"].is_string());
        assert!(value["ts"].is_string());
    }

    #[test]
    fn the_demotion_reorders_the_walk_and_leaves_the_selection_alone() {
        let chain = vec![
            resolved_for_test("router-kit"),
            resolved_for_test("zai-coding-plan"),
            resolved_for_test("xiaomi-mimo"),
        ];
        // A demoted candidate moves to the end; the selected one keeps the
        // head of the walk.
        let ordered = reorder_for_demotion(chain, "zai-coding-plan", &["router-kit".to_string()]);
        let ids: Vec<&str> = ordered.iter().map(Resolved::id).collect();
        assert_eq!(ids, vec!["zai-coding-plan", "xiaomi-mimo", "router-kit"]);

        // A candidate that is both selected and demoted stays the head:
        // demotion reorders the walk, it never overrides the judgment.
        let chain = vec![
            resolved_for_test("router-kit"),
            resolved_for_test("zai-coding-plan"),
            resolved_for_test("xiaomi-mimo"),
        ];
        let ordered = reorder_for_demotion(chain, "router-kit", &["router-kit".to_string()]);
        let ids: Vec<&str> = ordered.iter().map(Resolved::id).collect();
        assert_eq!(ids, vec!["router-kit", "zai-coding-plan", "xiaomi-mimo"]);
    }

    /// One resolved candidate for the reorder tests.
    fn resolved_for_test(id: &str) -> Resolved {
        Resolved {
            candidate: Candidate {
                id: id.to_string(),
                base_url: "http://127.0.0.1:1/v1".to_string(),
                key_env: format!("{}_KEY", id.to_uppercase().replace('-', "_")),
                model: format!("{id}-model"),
                enabled: true,
                key_required: false,
                disabled_reason: None,
            },
            key: None,
        }
    }

    #[test]
    fn the_config_calib_section_names_the_log_root() {
        let home = std::env::temp_dir().join(format!("fleet-calib-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        // Absent by default.
        assert_eq!(calib_log_root(&home), None);
        std::fs::write(
            home.join("config.json"),
            r#"{ "calib": { "log_root": "/tmp/somewhere-else/logs" } }"#,
        )
        .unwrap();
        assert_eq!(
            calib_log_root(&home),
            Some(PathBuf::from("/tmp/somewhere-else/logs"))
        );
        // A misspelled section is rejected, the way a provider's is.
        std::fs::write(home.join("config.json"), r#"{ "calib": { "logroot": "/x" } }"#).unwrap();
        assert_eq!(calib_log_root(&home), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_config_skills_section_names_the_curation_knobs() {
        let home = std::env::temp_dir().join(format!("fleet-skills-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        // Absent is the compiled-in defaults.
        assert_eq!(skills_config(&home), SkillsConfig::default());
        assert_eq!(
            skills_config(&home).retire_after_sessions,
            crate::skills::DEFAULT_RETIRE_AFTER_SESSIONS
        );
        std::fs::write(
            home.join("config.json"),
            r#"{ "skills": { "retire_after_sessions": 3, "merge_similarity": 0.65 } }"#,
        )
        .unwrap();
        let config = skills_config(&home);
        assert_eq!(config.retire_after_sessions, 3);
        assert!((config.merge_similarity - 0.65).abs() < 1e-9);
        // A misspelled key is rejected, the way a provider's is.
        std::fs::write(
            home.join("config.json"),
            r#"{ "skills": { "retire_after": 3 } }"#,
        )
        .unwrap();
        assert_eq!(skills_config(&home), SkillsConfig::default());
        // A similarity outside 0..1 is clamped, not silently disabling.
        std::fs::write(
            home.join("config.json"),
            r#"{ "skills": { "merge_similarity": 4.2 } }"#,
        )
        .unwrap();
        assert_eq!(skills_config(&home).merge_similarity, 1.0);
        let _ = std::fs::remove_dir_all(&home);
    }

    // --- the providers surface ---

    #[test]
    fn the_providers_render_is_cards_with_the_head_marked_and_never_a_table() {
        let resolved = resolve(&default_roster(), &HashMap::new());
        // No keys are set in this process, so the remote plans read `no key`
        // and the loopback kit reads `absent (optional)`.
        let rendered = render(&resolved);
        assert_eq!(rendered.matches('\u{250c}').count(), 7, "{rendered}");
        // The head is marked and named.
        assert!(rendered.contains("head router-kit"), "{rendered}");
        // Key env-var NAMES appear; no value ever does.
        assert!(rendered.contains("ZAI_CODING_API_KEY"), "{rendered}");
        assert!(rendered.contains("DEEPSEEK_API_KEY"), "{rendered}");
        // Liquid rides as the seventh card: declared, available to the
        // owner's config, its endpoint and model named.
        assert!(rendered.contains("LIQUID_API_KEY"), "{rendered}");
        assert!(rendered.contains("https://api.liquid.ai/openai/v1"), "{rendered}");
        assert!(rendered.contains("model lfm2.5-24b-a2b"), "{rendered}");
        // The endpoints and the models ride along.
        assert!(rendered.contains("https://api.z.ai/api/coding/paas/v4"), "{rendered}");
        assert!(rendered.contains("model deepseek-chat"), "{rendered}");
        // No plain-text table.
        assert!(!rendered.contains("| id |"), "{rendered}");
    }

    #[test]
    fn the_declared_chain_is_the_owners_order() {
        let roster = default_roster();
        let ids: Vec<&str> = roster.iter().map(|candidate| candidate.id.as_str()).collect();
        // Liquid is appended at the END and asserted here because it
        // changes the compiled-in list — but it enters no declared
        // position: the chain's order is the owner's standing directive
        // and this table does not reorder it. The owner's config (a pin,
        // or a chain entry) decides liquid's real position.
        assert_eq!(
            ids,
            vec![
                "router-kit",
                "zai-coding-plan",
                "xiaomi-mimo",
                "stepfun",
                "token-plan",
                "deepseek",
                "liquid"
            ]
        );
        // The kit is the head and needs no key; DeepSeek is the
        // declared order's last and does; Liquid is declared beside it
        // with its own key required.
        assert!(!roster[0].key_required);
        assert!(roster[5].key_required);
        assert_eq!(roster[6].id, "liquid");
        assert!(roster[6].key_required);
    }
}
