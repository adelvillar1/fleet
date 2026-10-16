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
use crate::lane_client;
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

/// One roster entry: a provider — where it lives, the *name* of the env
/// var holding its key (never a value), the models it serves, how it is
/// billed, and how a request reaches it.
/// `enabled`/`key_required` gate whether a run may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub base_url: String,
    /// The environment variable NAME holding this candidate's key. The
    /// value is resolved at runtime from the environment, `<home>/.env`,
    /// or the harness credential store — never stored here.
    pub key_env: String,
    /// The provider's models, in the provider's own preference order;
    /// the first is its default. The preference of execution chooses
    /// among these per workload — and a provider pin routes every shape
    /// through this provider's own models.
    pub models: Vec<String>,
    /// How this provider is billed. The plan providers are the
    /// preference; a pay-as-you-go provider is declared and targeted by
    /// no default workload — the kit's `allowPayg: false` ported as
    /// policy, which is what makes the reserve unable to serve
    /// silently.
    pub billing: Billing,
    /// How a request reaches this provider: an OpenAI-compatible HTTP
    /// endpoint, or a vendored lane driven over fleet's own MCP client
    /// (blend1's composed ask — the grounding provider, smaller models
    /// composed with the specialized legs).
    pub kind: CandidateKind,
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

impl Candidate {
    /// The provider's default model — the first of its list.
    pub fn model(&self) -> &str {
        self.models.first().map(String::as_str).unwrap_or_default()
    }
}

/// How a provider is billed. The preference of execution targets plan
/// providers; a payg one is the declared reserve a pin can still reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Billing {
    Plan,
    Payg,
}

/// How a request reaches a provider. `Http` is an OpenAI-compatible
/// chat-completions endpoint; `Lane` is a vendored lane driven over the
/// MCP client — the self-contained shape, no separate component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateKind {
    Http,
    Lane,
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
/// The compiled-in roster, before any config file merges over it.
/// Public because the settings writer needs to tell "revert this
/// provider to its compiled-in default" (an override whose id is in
/// here) from "remove this provider from the roster entirely" (an id
/// that is not) — the two are different edits with different blast
/// radius, and the second is the destructive one.
pub fn default_roster() -> Vec<Candidate> {
    vec![
        Candidate {
            id: "router-kit".into(),
            base_url: "http://127.0.0.1:8300/v1".into(),
            key_env: "AGNOSTIC_ROUTER_TOKEN".into(),
            models: vec!["auto".into()],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: false,
            disabled_reason: None,
        },
        Candidate {
            id: "zai-coding-plan".into(),
            base_url: "https://api.z.ai/api/coding/paas/v4".into(),
            key_env: "ZAI_CODING_API_KEY".into(),
            models: vec!["GLM-5.3-Flash".into()],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "xiaomi-mimo".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
            key_env: "XIAOMI_MIMO_API_KEY".into(),
            models: vec![
                "mimo-v2.6-flash".into(),
                "mimo-v2.6-pro".into(),
                "mimo-v2.5-pro".into(),
                "mimo-v2.5".into(),
            ],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "stepfun".into(),
            base_url: "https://api.stepfun.ai/step_plan/v1".into(),
            key_env: "STEPFUN_API_KEY".into(),
            models: vec![
                "step-3.7-flash".into(),
                "step-3.5-flash".into(),
                "step-3.5-flash-2603".into(),
                "step-5-preview".into(),
            ],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "token-plan".into(),
            base_url: "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1"
                .into(),
            key_env: "TOKEN_PLAN_API_KEY".into(),
            models: vec!["qwen3.8-flash".into()],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        Candidate {
            id: "deepseek".into(),
            base_url: crate::api::DEFAULT_BASE_URL.into(),
            key_env: "DEEPSEEK_API_KEY".into(),
            models: vec![crate::api::DEFAULT_MODEL.into()],
            billing: Billing::Payg,
            kind: CandidateKind::Http,
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
            models: vec!["lfm2.5-24b-a2b".into()],
            billing: Billing::Plan,
            kind: CandidateKind::Http,
            enabled: true,
            key_required: true,
            disabled_reason: None,
        },
        // blend1 — the grounding provider: the composed ask (Liquid's
        // hosted lfm 24B synthesizer over the specialized legs sys1 /
        // sdm1 / sem1 / gen1), which exists to give better grounded
        // responses from smaller models. A lane-kind candidate: the
        // vendored lane's own surface (`/v1/ask`, port 8420), driven
        // through fleet's MCP client rather than an external component.
        // The `composed` workload targets it; no other workload does.
        Candidate {
            id: "blend1".into(),
            base_url: "http://127.0.0.1:8420".into(),
            key_env: "BLEND1_BEARER_TOKEN".into(),
            models: vec!["blend1".into()],
            billing: Billing::Plan,
            kind: CandidateKind::Lane,
            enabled: true,
            key_required: false,
            disabled_reason: None,
        },
    ]
}

// --- the preference of execution --------------------------------------------

/// One workload's preference of execution: the ordered (provider, model)
/// candidates — the target first, then the fallbacks in preference order.
/// The walk is THIS list, not a global chain: a failure advances to the
/// workload's own next candidate, and an exhausted list fails the run by
/// name rather than falling through to a candidate no workload asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preference {
    pub workload: &'static str,
    pub candidates: Vec<(&'static str, &'static str)>,
}

/// The compiled-in preference table, ported from the live
/// zcode-router-kit's `routing.workloads` (verified 2026-10-15) — the
/// owner's tuned state, at (provider, model) granularity. Two
/// divergences from the source, both deliberate: `quick`'s duplicated
/// stepfun entry is deduped, and no workload targets deepseek (the payg
/// reserve — the kit's `allowPayg: false` ported as policy, so the
/// reserve cannot serve unless the owner pins it).
pub fn default_preferences() -> Vec<Preference> {
    vec![
        Preference {
            workload: "quick",
            candidates: vec![
                ("stepfun", "step-3.7-flash"),
                ("xiaomi-mimo", "mimo-v2.6-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
            ],
        },
        Preference {
            workload: "standard_code",
            candidates: vec![
                ("stepfun", "step-3.7-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
                ("xiaomi-mimo", "mimo-v2.6-flash"),
            ],
        },
        Preference {
            workload: "hard",
            candidates: vec![
                ("stepfun", "step-5-preview"),
                ("xiaomi-mimo", "mimo-v2.6-pro"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
            ],
        },
        Preference {
            workload: "prose",
            candidates: vec![
                ("stepfun", "step-3.7-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
                ("token-plan", "qwen3.8-flash"),
            ],
        },
        Preference {
            workload: "deep_context",
            candidates: vec![
                ("stepfun", "step-5-preview"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
                ("xiaomi-mimo", "mimo-v2.6-pro"),
            ],
        },
        Preference {
            workload: "composed",
            candidates: vec![
                ("blend1", "blend1"),
                ("stepfun", "step-3.7-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
                ("xiaomi-mimo", "mimo-v2.6-flash"),
            ],
        },
    ]
}

/// The judgment gates, ported from the kit's `routing` block: a verdict
/// below `min_confidence` routes to `default_workload` rather than to
/// the guessed shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gates {
    pub min_confidence: f64,
    pub default_workload: &'static str,
}

impl Default for Gates {
    fn default() -> Self {
        Gates {
            min_confidence: 0.6,
            default_workload: "standard_code",
        }
    }
}

/// A workload's preference, or `None` for a workload no table names.
pub fn preference_for<'a>(table: &'a [Preference], workload: &str) -> Option<&'a Preference> {
    table.iter().find(|entry| entry.workload == workload)
}

/// A roster plus its optional pin. The chain order is the compiled-in
/// declared order; the config file overrides entries per-field, never the
/// order (slice 6's calibration reorders candidates over time).
#[derive(Debug, Clone)]
pub struct Roster {
    pub candidates: Vec<Candidate>,
    /// When set, this provider (or `provider/model` pair) is pinned: a
    /// provider pin still judges the workload and routes every shape
    /// through that provider's own models; a pair pins both halves and
    /// skips the judgment (the `pinned` progress-line mode).
    pub pin: Option<String>,
    /// The preference of execution: each workload's ordered
    /// (provider, model) candidates. Compiled-in defaults, overridden
    /// per workload by `<FLEET_HOME>/routing.json`.
    pub preferences: Vec<Preference>,
    /// The judgment gates. Compiled-in defaults, overridden by
    /// `<FLEET_HOME>/routing.json`.
    pub gates: Gates,
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
            ConfigError::BadEntry(detail) => {
                write!(f, "the routing config has a bad entry: {detail}")
            }
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
    pub models: Option<Vec<String>>,
    #[serde(default)]
    pub billing: Option<Billing>,
    #[serde(default)]
    pub kind: Option<CandidateKind>,
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
/// dev-decisions' own default is not where it should be, and whether the
/// calibration may move the failover's preference. Unknown fields are
/// rejected for the same reason a provider's are.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CalibEntry {
    #[serde(default)]
    log_root: Option<String>,
    /// Whether a provider whose judged sample is fit and whose
    /// confidence-weighted wrong-rate is at or over the floor is skipped
    /// in the failover's preference. Absent is on: the ride-back is the
    /// slice's point, and turning it off is the owner's explicit choice.
    #[serde(default)]
    informs_failover: Option<bool>,
}

/// The resolved calibration knobs, with the defaults applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationConfig {
    /// Whether the calibration may move the failover's preference. Off
    /// restores the pre-slice-18 behavior exactly: no calibration skip,
    /// no calibration clause in the decision row's reason.
    pub informs_failover: bool,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        CalibrationConfig {
            informs_failover: true,
        }
    }
}

/// The calibration knobs as `<home>/config.json` names them, with the
/// compiled-in default for everything the file does not. Best-effort by
/// design, like [`calib_log_root`]: a config too broken to parse has
/// already failed the roster load loud, and this reader is for the
/// surfaces that want the knobs without a roster.
pub fn calibration_config(home: &Path) -> CalibrationConfig {
    let Ok(text) = std::fs::read_to_string(home.join("config.json")) else {
        return CalibrationConfig::default();
    };
    let Ok(file) = serde_json::from_str::<ConfigFile>(&text) else {
        return CalibrationConfig::default();
    };
    CalibrationConfig {
        informs_failover: file
            .calib
            .and_then(|calib| calib.informs_failover)
            .unwrap_or(true),
    }
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
            // No config.json is the defaults — but routing.json is its
            // own plugin file and loads either way.
            let (preferences, gates) = load_routing(home)?;
            return Ok(Roster {
                candidates: default_roster(),
                pin: None,
                preferences,
                gates,
            });
        }
        Err(error) => return Err(ConfigError::Read(error)),
    };
    let file: ConfigFile = serde_json::from_str(&text)
        .map_err(|error| ConfigError::Parse(format!("{}: {error}", path.display())))?;
    let (candidates, pin) = merge(default_roster(), &file)?;
    let (preferences, gates) = load_routing(home)?;
    Ok(Roster {
        candidates,
        pin,
        preferences,
        gates,
    })
}

/// The `<FLEET_HOME>/routing.json` plugin file: the preference of
/// execution as data. Every field optional; a workload entry replaces
/// that workload's list, and the gates merge field by field. Unknown
/// fields are rejected the way a provider's are — loud beats silent.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingFile {
    #[serde(default)]
    workloads: Vec<PreferenceEntry>,
    #[serde(default)]
    gates: Option<GatesEntry>,
}

/// One workload's preference in the plugin file: the workload's name
/// and its ordered candidates, each a `"provider/model"` string.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreferenceEntry {
    workload: String,
    candidates: Vec<String>,
}

/// The gates as the plugin file names them.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatesEntry {
    #[serde(default)]
    min_confidence: Option<f64>,
    #[serde(default)]
    default_workload: Option<String>,
}

/// The preference table and gates: the compiled-in defaults, with
/// `<FLEET_HOME>/routing.json` merged over them. An absent file is the
/// defaults; a malformed one is a typed named error that fails startup
/// loud, the way a bad provider entry does.
fn load_routing(home: &Path) -> Result<(Vec<Preference>, Gates), ConfigError> {
    let path = home.join("routing.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((default_preferences(), Gates::default()));
        }
        Err(error) => return Err(ConfigError::Read(error)),
    };
    let file: RoutingFile = serde_json::from_str(&text)
        .map_err(|error| ConfigError::Parse(format!("{}: {error}", path.display())))?;
    // The plugin file's strings are leaked once per load: a roster is
    // loaded a handful of times per process, and the table's
    // `&'static str` shape is what the rest of the crate reads.
    let mut preferences = default_preferences();
    for entry in &file.workloads {
        let mut candidates = Vec::with_capacity(entry.candidates.len());
        for candidate in &entry.candidates {
            let (provider, model) = candidate.split_once('/').ok_or_else(|| {
                ConfigError::BadEntry(format!(
                    "routing.json's `{}` candidate `{candidate}` is not a `provider/model` pair",
                    entry.workload
                ))
            })?;
            candidates.push((leak_str(provider), leak_str(model)));
        }
        if let Some(slot) = preferences
            .iter_mut()
            .find(|slot| slot.workload == entry.workload)
        {
            slot.candidates = candidates;
        } else {
            preferences.push(Preference {
                workload: leak_str(&entry.workload),
                candidates,
            });
        }
    }
    let mut gates = Gates::default();
    if let Some(entry) = &file.gates {
        if let Some(value) = entry.min_confidence {
            gates.min_confidence = value;
        }
        if let Some(value) = &entry.default_workload {
            gates.default_workload = leak_str(value);
        }
    }
    Ok((preferences, gates))
}

/// Leak one string for the table's `'static` lifetime. The routing
/// plugin file is read a handful of times per process at most; the
/// alternative — an owned table threaded through every signature —
/// buys nothing a roster load does not already cost.
fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_string().into_boxed_str())
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
    let Ok(text) = std::fs::read_to_string(home.join("config.json")) else {
        return config;
    };
    let Ok(file) = serde_json::from_str::<ConfigFile>(&text) else {
        return config;
    };
    let Some(skills) = file.skills else {
        return config;
    };
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
        if let Some(existing) = candidates
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
        {
            if let Some(value) = &entry.base_url {
                existing.base_url = value.clone();
            }
            if let Some(value) = &entry.key_env {
                existing.key_env = value.clone();
            }
            if let Some(value) = &entry.models {
                existing.models = value.clone();
            }
            if let Some(value) = &entry.model {
                // The singular form sets a one-model list — the
                // backward-compatible spelling of `models: [value]`.
                existing.models = vec![value.clone()];
            }
            if let Some(value) = entry.billing {
                existing.billing = value;
            }
            if let Some(value) = entry.kind {
                existing.kind = value;
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
            let models = entry
                .models
                .clone()
                .or_else(|| entry.model.clone().map(|model| vec![model]));
            match (&entry.base_url, &entry.key_env, &models) {
                (Some(base_url), Some(key_env), Some(models)) => candidates.push(Candidate {
                    id: entry.id.clone(),
                    base_url: base_url.clone(),
                    key_env: key_env.clone(),
                    models: models.clone(),
                    billing: entry.billing.unwrap_or(Billing::Plan),
                    kind: entry.kind.unwrap_or(CandidateKind::Http),
                    enabled: entry.enabled.unwrap_or(true),
                    key_required: entry.key_required.unwrap_or(true),
                    disabled_reason: entry.disabled_reason.clone(),
                }),
                _ => {
                    return Err(ConfigError::BadEntry(format!(
                        "provider `{}` is not a built-in roster entry and does not name \
                         base_url, key_env, and a model (or models), so there is nothing to \
                         merge it onto",
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
    resolved
        .iter()
        .position(|entry| entry.candidate.enabled)
        .unwrap_or(0)
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

/// Where this route's judgment came from. A cached judgment is a
/// recorded one, and the row says so: a cache that suppressed its own
/// provenance would make the calibration stream's counts lie about
/// how many judgments were actually made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JudgedFrom {
    /// The judging lane answered this run.
    #[default]
    Fresh,
    /// The route-decision cache answered from a recorded verdict.
    Cached,
    /// No judgment was obtained at all — the fallback and pin paths.
    None,
}

impl JudgedFrom {
    /// The word the decision row and the surfaces render with.
    pub fn label(self) -> &'static str {
        match self {
            JudgedFrom::Fresh => "fresh",
            JudgedFrom::Cached => "cached",
            JudgedFrom::None => "none",
        }
    }
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
    /// The model the selected candidate serves — what the run will
    /// actually call, carried so the surface and the session log name
    /// the truth instead of the CLI's requested default.
    pub model: String,
    pub how: How,
    /// sys1's judged workload, when there was one.
    pub workload: Option<String>,
    /// sys1's verdict (`pass`/`escalate`/`block`/`error`), when there was one.
    pub verdict: Option<String>,
    /// Why this route, in one line — the decision row and the status bar
    /// both read it.
    pub reason: String,
    /// Whether the judgment was made this run, served from the
    /// route-decision cache, or never obtained. Rides the decision so
    /// the row can count the split without re-deriving it.
    pub judged_from: JudgedFrom,
    /// The chain index failover begins walking from.
    pub start_index: usize,
    /// The walk order: the (provider, model) pairs this route walks, the
    /// target first — the workload's own preference list, a provider pin
    /// filtering it, or an exact pair. The walk is THIS list, never a
    /// global chain: an exhausted list fails the run by name.
    pub order: Vec<(String, String)>,
}

/// The candidate a workload's preference targets — the first of its
/// list. Public because the judgment beat grades a recorded decision
/// against the lane's independent workload label through this same
/// table, so a judgment and a selection cannot disagree about what a
/// workload maps to.
pub fn workload_candidate(workload: &str) -> Option<&'static str> {
    default_preferences()
        .into_iter()
        .find(|preference| preference.workload == workload)
        .and_then(|preference| preference.candidates.first().map(|(provider, _)| *provider))
}

/// Choose the route for `task`: a pin wins (a provider pin still lets
/// the router decide the model), else sys1 judges the workload and the
/// workload's own preference list is the walk. Never fails and never
/// blocks a run — every failure path is a named fallback. Returns the
/// decision and how long the judgment took (for the decision row's
/// `latency_ms`).
pub async fn select(
    resolved: &[Resolved],
    pin: Option<&str>,
    task: &str,
    specs: &HashMap<String, ServerSpec>,
    table: &[Preference],
    gates: Gates,
    cache: Option<&mut crate::route_cache::Cache>,
) -> (RouteDecision, u128) {
    let started = Instant::now();
    let head = head_index(resolved);
    // A pin naming an exact `provider/model` pair is the one pin that
    // skips the judgment: both halves are already decided.
    if let Some(pin_id) = pin {
        if let Some((provider, model)) = pin_id.split_once('/') {
            return (
                decision_from(
                    vec![(provider.to_string(), model.to_string())],
                    How::Pinned,
                    None,
                    None,
                    format!("pinned to {provider}/{model}"),
                    JudgedFrom::None,
                ),
                started.elapsed().as_millis(),
            );
        }
    }
    let provider_pin = pin.filter(|pin_id| {
        resolved.iter().any(|entry| entry.id() == *pin_id) || pin_id.contains('/')
    });
    let unknown_pin = pin
        .filter(|pin_id| {
            !pin_id.contains('/') && !resolved.iter().any(|entry| entry.id() == *pin_id)
        })
        .map(|pin_id| format!("pin `{pin_id}` names no roster provider; "));

    let (judged, judged_from) = judge(task, specs, cache).await;
    // The workload this route serves: sys1's, or the gates' default when
    // the judgment is absent or below the confidence floor. The
    // confidence itself rides the gate note in the reason — the lane's
    // own number, named where it changed the route. A gated route is a
    // FALLBACK, not a judgment: the row must not claim sys1 chose it.
    let (workload, verdict, _confidence, gate_note, gated) = match &judged {
        Some((label, confidence, verdict)) if *confidence >= gates.min_confidence => (
            label.clone(),
            Some(verdict.clone()),
            *confidence,
            String::new(),
            false,
        ),
        Some((label, confidence, _)) => (
            gates.default_workload.to_string(),
            judged.as_ref().map(|(_, _, verdict)| verdict.clone()),
            *confidence,
            format!(
                "sys1 judged `{label}` at {confidence:.2}, below the {:.2} gate; ",
                gates.min_confidence
            ),
            true,
        ),
        None => (
            gates.default_workload.to_string(),
            None,
            0.0,
            "sys1 gave no usable verdict; ".to_string(),
            true,
        ),
    };

    let decision = match preference_for(table, &workload) {
        Some(preference) => {
            // The walk is the workload's own list — a provider pin
            // filters it to that provider, and the router still decides
            // the model from what the workload asks of it.
            let order: Vec<(String, String)> = preference
                .candidates
                .iter()
                .filter(|(provider, _)| provider_pin.is_none_or(|pin| pin == *provider))
                .map(|(provider, model)| (provider.to_string(), model.to_string()))
                .collect();
            let (order, reason) = if order.is_empty() {
                // The workload names no candidate from the pinned
                // provider: the provider's own default model serves,
                // and the reason says so rather than silently
                // substituting some other provider.
                match provider_pin {
                    Some(pin) => (
                        vec![(
                            pin.to_string(),
                            resolved
                                .iter()
                                .find(|entry| entry.id() == pin)
                                .map(|entry| entry.candidate.model().to_string())
                                .unwrap_or_default(),
                        )],
                        format!(
                            "`{workload}` names no {pin} candidate; using {pin}'s default model"
                        ),
                    ),
                    None => (Vec::new(), format!("`{workload}`'s preference is empty")),
                }
            } else {
                let reason = if let Some(pin) = provider_pin {
                    format!("pinned to {pin}; sys1 judged `{workload}`")
                } else {
                    format!("sys1 judged `{workload}`")
                };
                (order, reason)
            };
            let note = format!(
                "{}{}",
                unknown_pin.as_deref().unwrap_or_default(),
                gate_note,
            );
            if order.is_empty() {
                fallback(
                    resolved,
                    head,
                    table,
                    gates,
                    &format!("{note}{reason}"),
                    judged_from,
                )
            } else {
                let how = if provider_pin.is_some() {
                    How::Pinned
                } else if gated {
                    How::Fallback
                } else {
                    How::Judged
                };
                decision_from(
                    order,
                    how,
                    Some(workload.clone()),
                    verdict.clone(),
                    format!("{note}{reason}"),
                    judged_from,
                )
            }
        }
        None => fallback(
            resolved,
            head,
            table,
            gates,
            &format!(
                "{}sys1 judged an unmapped workload `{workload}`; ",
                unknown_pin.as_deref().unwrap_or_default()
            ),
            judged_from,
        ),
    };
    (decision, started.elapsed().as_millis())
}

/// A decision over an explicit walk order — the one construction, so
/// the order, the target, and the model can never disagree.
fn decision_from(
    order: Vec<(String, String)>,
    how: How,
    workload: Option<String>,
    verdict: Option<String>,
    reason: String,
    judged_from: JudgedFrom,
) -> RouteDecision {
    let (provider, model) = order
        .first()
        .cloned()
        .unwrap_or_else(|| (HEAD_ID.to_string(), String::new()));
    // The walk starts at the selection, and the selection IS the
    // order's head — so the start index is an ORDER position, always
    // zero here. It is not the candidate's roster position: that was
    // the pre-port walk's semantics (the walk was the declared chain),
    // and using it here would start a multi-entry preference's walk
    // mid-list, silently skipping the very candidate sys1 selected.
    // `prepare` recomputes the same value after the demotion reorder,
    // which also keeps the selected candidate at the head.
    let start_index = 0;
    RouteDecision {
        provider,
        model,
        how,
        workload,
        verdict,
        reason,
        judged_from,
        start_index,
        order,
    }
}

/// A fallback decision: the gates' default workload's preference, named
/// with `reason`. There is no silent fall-through to a candidate no
/// workload asked for — the default workload's list IS the fallback.
fn fallback(
    resolved: &[Resolved],
    head: usize,
    table: &[Preference],
    gates: Gates,
    reason: &str,
    judged_from: JudgedFrom,
) -> RouteDecision {
    let order = preference_for(table, gates.default_workload)
        .map(|preference| {
            preference
                .candidates
                .iter()
                .map(|(provider, model)| (provider.to_string(), model.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let mut decision = decision_from(
        order,
        How::Fallback,
        None,
        None,
        reason.to_string(),
        judged_from,
    );
    // An empty table (a config that named no workloads) still has to
    // walk something: the declared chain's head, as the pre-slice
    // behavior did.
    if decision.order.is_empty() {
        let provider = resolved
            .get(head)
            .map(Resolved::id)
            .unwrap_or(HEAD_ID)
            .to_string();
        decision.provider = provider.clone();
        decision.start_index = head;
        decision.order = resolved
            .iter()
            .map(|entry| (entry.id().to_string(), entry.candidate.model().to_string()))
            .collect();
    }
    decision
}

/// Ask sys1 for a routing judgment — or read the recorded one. A hit
/// skips the spawn entirely; a miss spawns the lane (like any other),
/// calls `sys1_classify`, and writes the verdict through so the next
/// run for this task finds it. Any failure — the lane cannot launch,
/// the handshake or call fails, the answer has no workload — is
/// `None`, which the caller turns into a fallback. The child is always
/// torn down.
///
/// The second element says which happened, because a cached judgment
/// is still a judgment the row must attribute honestly.
async fn judge(
    task: &str,
    specs: &HashMap<String, ServerSpec>,
    cache: Option<&mut crate::route_cache::Cache>,
) -> (Option<(String, f64, String)>, JudgedFrom) {
    let spec = match specs.get(JUDGE_LANE) {
        Some(spec) => spec.clone(),
        None => match lanes::spec_for(JUDGE_LANE) {
            Ok(spec) => spec,
            Err(_) => return (None, JudgedFrom::None),
        },
    };
    let hash = calib::hash16(task);
    // The cache is consulted only when it belongs to the lane that
    // would judge now: a key mismatch discards the whole file rather
    // than serving another lane's verdicts.
    if let Some(cache) = cache.as_ref() {
        if cache.keyed_for(&crate::route_cache::Cache::key_for(&spec)) {
            if let Some(recorded) = cache.get(&hash) {
                return (
                    Some((
                        recorded.workload.clone(),
                        recorded.confidence,
                        recorded.verdict.clone(),
                    )),
                    JudgedFrom::Cached,
                );
            }
        }
    }
    let server = match McpServer::spawn(&spec) {
        Ok(server) => server,
        Err(_) => return (None, JudgedFrom::None),
    };
    let outcome = judge_live(&server, task).await;
    let _ = server.close().await;
    let from = if outcome.is_some() {
        JudgedFrom::Fresh
    } else {
        JudgedFrom::None
    };
    // The write-through: a verdict the lane actually made is recorded
    // for the next run. A failed write costs a recompute and nothing
    // else — the cache never blocks a route.
    if let (Some(cache), Some((workload, confidence, verdict))) = (cache, outcome.as_ref()) {
        cache.put(
            hash,
            crate::route_cache::Verdict {
                workload: workload.clone(),
                confidence: *confidence,
                verdict: verdict.clone(),
            },
        );
        cache.flush(&crate::route_cache::Cache::key_for(&spec));
    }
    (outcome, from)
}

/// The live judgment over an already-spawned sys1 server. The call shape
/// is the tool's own schema (`tools/list`): `sys1_classify` takes `text`
/// (required) and `task` (the registered task name).
async fn judge_live(server: &McpServer, task: &str) -> Option<(String, f64, String)> {
    server.initialize().await.ok()?;
    let call = server
        .call_tool("sys1_classify", json!({ "task": JUDGE_TASK, "text": task }))
        .await
        .ok()?;
    parse_workload(&call.text())
}

/// Read the workload verdict out of a `sys1_classify` answer. The answer's
/// text is a JSON object whose `answers` maps provider id → head id →
/// answer; the routing signal is the `workload` head's `label` and its
/// `confidence`. The top-level `verdict` rides along for the decision
/// row. A missing or empty workload label is `None` (an unusable verdict
/// → fallback); a missing confidence reads as 0.0, which the gate then
/// routes to the default workload.
fn parse_workload(text: &str) -> Option<(String, f64, String)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let verdict = value
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let answers = value.get("answers")?.as_object()?;
    for heads in answers.values() {
        if let Some(workload) = heads
            .get("workload")
            .and_then(|workload| workload.as_object())
        {
            if let Some(label) = workload
                .get("label")
                .and_then(|label| label.as_str())
                .filter(|label| !label.is_empty())
            {
                let confidence = workload
                    .get("confidence")
                    .and_then(|c| c.as_f64())
                    .unwrap_or(0.0);
                return Some((label.to_string(), confidence, verdict));
            }
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

/// What one session's walk actually did. The surface prints it after
/// every run, so "why did this candidate serve" is answerable without
/// opening the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkReport {
    /// The candidate that served and its model, when one did.
    pub served: Option<(String, String)>,
    /// Every attempt, in the order the walk made them.
    pub attempts: Vec<Attempt>,
    /// The candidates skipped for want of keys, each with the env var
    /// it wanted — the named absence, never a bare skip.
    pub unkeyed: Vec<(String, String)>,
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
            // A lane candidate's failure names itself: the boundary's
            // messages start "lane `<name>`: ", and the class says the
            // failure was a lane's rather than guessing from the prose.
            if lowered.starts_with("lane ") {
                "lane"
            } else if lowered.contains("timeout") || lowered.contains("timed out") {
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
                attempts.push(Attempt {
                    provider,
                    outcome: "served",
                    class: None,
                });
                return (Some(index), attempts);
            }
            WalkOutcome::Failed { class, streamed } => {
                attempts.push(Attempt {
                    provider,
                    outcome: "failed",
                    class: Some(class),
                });
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
    /// The calibration clauses the walk collected, in the order the
    /// skips happened. They ride the decision row's reason, so a later
    /// reader sees why the walk was ordered as it was.
    calibration_clauses: Vec<String>,
    /// The clause a lane candidate's answer earned: its path, its
    /// escalation, its score — the lane's own facts, surfaced on the
    /// row rather than flattened away (the owner's ruling on the
    /// blend1 adapter).
    lane_note: Option<String>,
    /// The named refusal a lane candidate's failure left on the row:
    /// the lane's own words, so a reader sees WHY the composed leg
    /// advanced rather than inferring it from a bare class.
    lane_failure: Option<String>,
}

/// What the walk invokes per candidate. A gateway candidate is a plain
/// chat-completions client; a lane candidate (blend1) is a launch spec
/// whose ask is mapped at [`crate::lane_client`]'s boundary — or the
/// named reason no spec could be resolved, which fails every attempt
/// by name and lets the walk advance.
enum Endpoint {
    Http(Client),
    Lane(Result<ServerSpec, String>),
}

/// Walks the roster with failover. Built once per session from the resolved
/// roster and the selection decision; the run's [`Client`] holds it behind
/// an `Arc` and every streamed request crosses [`Router::chat_stream`].
pub struct Router {
    /// One endpoint per candidate (no router of its own), reused across
    /// turns so each gateway's connection pool survives.
    endpoints: Vec<Endpoint>,
    resolved: Vec<Resolved>,
    /// The walk order: positions into `resolved`, in the order this
    /// session's preference walks them — the workload's own candidate
    /// list, demotion applied.
    order: Vec<usize>,
    /// The model each order position serves: the preference names
    /// (provider, model) pairs, and a provider's default is not what
    /// every workload asks for.
    models: Vec<String>,
    decision: RouteDecision,
    state: Mutex<RouterState>,
    /// The candidates the calibration skips in this walk's preference, as
    /// the cells themselves. Slice 18's ride-back: a provider's cell for
    /// the current workload's shape — and only that cell — is consulted,
    /// so a provider judged wrong for one shape is never skipped for
    /// another. Empty when the gate is off, which restores the pre-slice
    /// behavior exactly.
    calibration_skips: Vec<calib::CalibrationCell>,
}

impl Router {
    /// A router over `resolved`, starting its walk at the decision's index.
    pub fn new(resolved: Vec<Resolved>, decision: RouteDecision) -> Router {
        Router::build(resolved, decision, Vec::new(), &HashMap::new())
    }

    /// A router over `resolved` whose walk passes over the calibration's
    /// skipped cells. The private half of [`Router::new`]: the
    /// constructor's shape is unchanged for every caller that has no
    /// calibration to consult.
    ///
    /// `lane_specs` carries the launch spec for each lane-kind candidate
    /// by lane name — the agent's own specs, so a lane the operator
    /// configured is launched the way they configured it. A lane the map
    /// does not name falls back to the vendored copy (`lanes::spec_for`),
    /// which is the self-contained harness's shape; a lane with neither
    /// is a named reason, and every attempt on it fails by that name.
    fn build(
        resolved: Vec<Resolved>,
        decision: RouteDecision,
        calibration_skips: Vec<calib::CalibrationCell>,
        lane_specs: &HashMap<String, ServerSpec>,
    ) -> Router {
        let endpoints = resolved
            .iter()
            .map(|entry| match entry.candidate.kind {
                CandidateKind::Http => Endpoint::Http(Client::new(
                    entry.candidate.base_url.as_str(),
                    entry.key.clone().unwrap_or_default(),
                )),
                CandidateKind::Lane => Endpoint::Lane(
                    lane_specs
                        .get(entry.id())
                        .cloned()
                        .or_else(|| lanes::spec_for(entry.id()).ok())
                        .ok_or_else(|| format!("the lane `{}` has no launch spec", entry.id())),
                ),
            })
            .collect();
        // The walk order: the decision's own (provider, model) list as
        // positions into `resolved`, each position's model beside it. A
        // decision that carries no order (a hand-built one, or an empty
        // preference table) walks the declared chain with each
        // candidate's default model — the pre-slice shape.
        let (order, models) = if decision.order.is_empty() {
            let order: Vec<usize> = (0..resolved.len()).collect();
            let models: Vec<String> = resolved
                .iter()
                .map(|entry| entry.candidate.model().to_string())
                .collect();
            (order, models)
        } else {
            let mut order = Vec::with_capacity(decision.order.len());
            let mut models = Vec::with_capacity(decision.order.len());
            for (provider, model) in &decision.order {
                let position = resolved.iter().position(|entry| entry.id() == *provider);
                // A preference naming a provider the roster does not
                // carry is skipped, not fatal: the walk is a record of
                // what it could try.
                if let Some(position) = position {
                    order.push(position);
                    models.push(model.clone());
                }
            }
            (order, models)
        };
        let index = decision.start_index.min(order.len().saturating_sub(1));
        Router {
            endpoints,
            resolved,
            order,
            models,
            decision,
            state: Mutex::new(RouterState {
                index,
                committed: false,
                served: None,
                attempts: Vec::new(),
                calibration_clauses: Vec::new(),
                lane_note: None,
                lane_failure: None,
            }),
            calibration_skips,
        }
    }

    /// The initial selection decision (for the headless line and the TUI
    /// status bar).
    pub fn decision(&self) -> RouteDecision {
        self.decision.clone()
    }

    /// Whether this walk's preference passes `provider` over, and the
    /// clause that says why — the cell's own numbers, so a later reader
    /// sees which (provider × shape) cell caused the skip rather than a
    /// per-provider average.
    ///
    /// The conservative influence, and its bounds: the declared chain's
    /// order and its head are the owner's standing directive, so the walk's
    /// *starting* candidate — sys1's selection — is never skipped, and
    /// a candidate is skipped only when a later candidate in the declared
    /// chain is healthy. Everything between is the preference the
    /// calibration moves.
    fn calibration_skip(&self, position: usize, start: usize) -> Option<String> {
        if position <= start || self.calibration_skips.is_empty() {
            return None;
        }
        let provider = self.resolved.get(self.order[position])?.id().to_string();
        let cell = self
            .calibration_skips
            .iter()
            .find(|cell| cell.provider == provider)?;
        // A skip that would leave the walk with nothing to try is not a
        // skip: the last healthy candidate in THIS walk's order is
        // always attempted.
        let healthy_later = self
            .order
            .iter()
            .enumerate()
            .skip(position + 1)
            .filter_map(|(_, &index)| self.resolved.get(index))
            .any(|entry| entry.usable());
        healthy_later.then(|| {
            format!(
                "; calibration skipped {provider} for shape `{}` in the failover preference \
                 (judged wrong at {:.2} on a fit cell of {} rows, span {:.2})",
                cell.shape(),
                cell.weighted_wrong_rate,
                cell.judged,
                cell.confidence_span,
            )
        })
    }

    /// The assembled decision row after the run, with every attempt and the
    /// final choice. `latency_ms` is the selection judgment's cost, and
    /// the reason carries every clause the walk collected — the demotion
    /// that reordered it (slice 5) and the calibration that skipped a
    /// candidate in its preference (slice 18).
    pub fn row(&self, task: &str, latency_ms: u128) -> calib::RouteRow {
        let state = lock(&self.state);
        // What served: the position that served, read through the walk
        // order into the roster — or the decision's own target when
        // nothing did.
        let served = state.served.and_then(|position| {
            let index = *self.order.get(position)?;
            let entry = self.resolved.get(index)?;
            Some((
                entry.id().to_string(),
                self.models.get(position).cloned().unwrap_or_default(),
            ))
        });
        let (provider, model) = served
            .clone()
            .unwrap_or_else(|| (self.decision.provider.clone(), self.decision.model.clone()));
        let providers_used: Vec<String> = state
            .attempts
            .iter()
            .filter(|attempt| attempt.outcome != "skipped")
            .map(|attempt| attempt.provider.clone())
            .collect();
        let mut reason = self.decision.reason.clone();
        for clause in &state.calibration_clauses {
            reason.push_str(clause);
        }
        // The lane's own facts, when a lane candidate served: the path
        // it answered through, whether it escalated, its score.
        if let Some(note) = &state.lane_note {
            reason.push_str(note);
        }
        // The named refusal, when a lane candidate failed: its own
        // words, so the row says why the walk advanced.
        if let Some(failure) = &state.lane_failure {
            reason.push_str(&format!("; {failure}"));
        }
        let ts = session::iso_utc_stamp(SystemTime::now()).unwrap_or_default();
        calib::RouteRow {
            ts,
            provider,
            how: self.decision.how.label().to_string(),
            workload: self.decision.workload.clone(),
            verdict: self.decision.verdict.clone(),
            reason,
            model,
            providers_used,
            attempts: state.attempts.clone(),
            input_chars: task.chars().count(),
            input_hash: calib::hash16(task),
            latency_ms,
            judged_from: self.decision.judged_from.label().to_string(),
        }
    }

    /// What this session's walk actually did, for the surface: the
    /// candidate that served and its model, every attempt in order, and
    /// the candidates skipped for want of keys with the env var each
    /// wanted. The route row already carries the same facts for the
    /// stream; this is the owner-facing half.
    pub fn walk_report(&self) -> WalkReport {
        let state = lock(&self.state);
        let served = state.served.and_then(|position| {
            let index = *self.order.get(position)?;
            let entry = self.resolved.get(index)?;
            Some((
                entry.id().to_string(),
                self.models.get(position).cloned().unwrap_or_default(),
            ))
        });
        let unkeyed = state
            .attempts
            .iter()
            .filter(|attempt| attempt.class == Some("no-key"))
            .filter_map(|attempt| {
                self.resolved
                    .iter()
                    .find(|entry| entry.id() == attempt.provider)
                    .map(|entry| (entry.id().to_string(), entry.candidate.key_env.clone()))
            })
            .collect();
        WalkReport {
            served,
            attempts: state.attempts.clone(),
            unkeyed,
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
                    // `start` is a walk position: the provider it names
                    // is read through the order, the same mapping every
                    // other record here applies (KI-208).
                    self.record(Attempt {
                        provider: self.resolved[self.order[start]].id().to_string(),
                        outcome: "failed",
                        class: Some(class),
                    });
                    Err(error)
                }
            };
        }

        let mut last_error: Option<ClientError> = None;
        for position in start..self.order.len() {
            let index = self.order[position];
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
            // The calibration's preference, consulted before the attempt:
            // a fit-and-wrong candidate is passed over in favour of a
            // healthy later one, and the clause that says why rides the
            // decision row's reason (see `prepare`).
            if let Some(clause) = self.calibration_skip(position, start) {
                lock(&self.state).calibration_clauses.push(clause);
                self.record(Attempt {
                    provider: self.resolved[index].id().to_string(),
                    outcome: "skipped",
                    class: Some("calibration"),
                });
                continue;
            }
            match self.one_attempt(position, request, on_event).await {
                Ok(response) => {
                    self.record(Attempt {
                        provider: self.resolved[index].id().to_string(),
                        outcome: "served",
                        class: None,
                    });
                    let mut state = lock(&self.state);
                    state.index = position;
                    state.committed = true;
                    state.served = Some(position);
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
        Err(last_error.unwrap_or_else(|| {
            ClientError::Network("no usable roster candidate remained".to_string())
        }))
    }

    /// One candidate's attempt: build the request with the position's
    /// model, wrap `on_event` so the first token marks the stream
    /// committed, and return the response or the error plus whether it
    /// streamed.
    async fn one_attempt(
        &self,
        position: usize,
        request: &ChatRequest<'_>,
        on_event: &mut (dyn FnMut(StreamEvent) + '_),
    ) -> Result<ChatResponse, (ClientError, bool)> {
        let index = self.order[position];
        let model = self.models[position].clone();
        match &self.endpoints[index] {
            Endpoint::Http(client) => {
                let routed = ChatRequest {
                    model: &model,
                    messages: request.messages,
                    tools: request.tools.clone(),
                };
                let streamed = Arc::new(AtomicBool::new(false));
                let flag = Arc::clone(&streamed);
                let result = client
                    .chat_stream_plain(&routed, |event| {
                        flag.store(true, Ordering::Relaxed);
                        on_event(event);
                    })
                    .await;
                result.map_err(|error| (error, streamed.load(Ordering::Relaxed)))
            }
            Endpoint::Lane(spec) => {
                // A lane candidate answers a one-shot ask, not a chat
                // completion: the request's conversation is mapped to
                // the lane's prompt at the boundary, and the lane's
                // answer back into the chat shape. A lane that cannot
                // be asked — no spec, a child that will not spawn, a
                // refusal — fails by name, and the walk advances.
                let prompt = lane_client::ask_prompt(request.messages);
                let asked = match spec {
                    Ok(spec) => lane_client::ask(spec, &prompt).await,
                    Err(reason) => Err(reason.clone()),
                };
                match asked {
                    Ok(answer) => {
                        let response = lane_client::chat_response(&answer);
                        on_event(StreamEvent::Content(answer.text));
                        lock(&self.state).lane_note = Some(answer.note);
                        Ok(response)
                    }
                    Err(reason) => {
                        // The named refusal rides the row: the lane's
                        // own words, beside the attempt's `lane` class.
                        let named = format!("lane `{}`: {reason}", self.resolved[index].id());
                        lock(&self.state).lane_failure = Some(named.clone());
                        Err((ClientError::Network(named), false))
                    }
                }
            }
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
        self.calib
            .record(calib::Row::Route(self.router.row(task, self.latency_ms)));
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
    // The route-decision cache, read beside the roster it describes. A
    // hit skips the judge lane's spawn entirely; a miss writes
    // through. The cache never blocks a route — every failure path
    // inside it costs a recompute and nothing else.
    let mut decision_cache = crate::route_cache::Cache::load(home);
    let (mut decision, latency_ms) = select(
        &resolved,
        roster.pin.as_deref(),
        task,
        specs,
        &roster.preferences,
        roster.gates,
        Some(&mut decision_cache),
    )
    .await;
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
    // Demotion reorders THIS session's walk — the workload's own list —
    // never the selection, and never the declared roster the surfaces
    // render.
    decision.order = reorder_for_demotion(&decision.order, &decision.provider, &demoted);
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
    // The calibration's ride-back: the day's judged rows name the
    // (provider × shape) cells whose fit sample is wrong at or over the
    // floor, and the walk passes them over in its preference — never at
    // the head, never ahead of sys1's selection, never for a shape other
    // than the one the row was judged for, and never when nothing
    // healthy lies after them. Gate-off restores the pre-slice behavior
    // exactly.
    let calibration = calibration_config(home);
    let skips = if calibration.informs_failover {
        calibration_skips(
            calib.root(),
            &calib::today(),
            &resolved,
            &decision.provider,
            decision.workload.as_deref().unwrap_or_default(),
        )
    } else {
        Vec::new()
    };
    // The third vector's read: a fit cell whose selections keep stalling
    // short of the minima for want of a companion names that in the
    // decision row's reason and moves nothing — the direction was right,
    // so the provider is not skipped. Same gate, same scope.
    let notes = if calibration.informs_failover {
        calibration_notes(
            calib.root(),
            &calib::today(),
            &resolved,
            &decision.provider,
            decision.workload.as_deref().unwrap_or_default(),
        )
    } else {
        Vec::new()
    };
    for cell in &notes {
        decision.reason.push_str(&calibration_note_clause(cell));
    }
    decision.start_index = decision
        .order
        .iter()
        .position(|(provider, _)| provider == &decision.provider)
        .unwrap_or(0);
    let router = Arc::new(Router::build(resolved, decision.clone(), skips, specs));
    Ok(Prepared {
        router,
        decision,
        latency_ms,
        calib,
    })
}

/// The cells the calibration skips in this walk's preference: every
/// roster candidate's cell **for the current workload's shape** (other
/// than the selected one, whose place at the head of the walk is sys1's
/// judgment's) whose sample is fit and whose confidence-weighted
/// wrong-rate is at or over [`calib::WRONG_RATE_FLOOR`].
///
/// Shape-scoped by construction: the day's cells are filtered to the one
/// workload shape the route was judged for, so a provider judged wrong
/// for shape X is never skipped for a shape-Y task, and its shape-Y
/// record never moves a shape-X walk. Pure and gated by its caller: the
/// gate-off path never calls it, which is what makes "the config turns
/// it off" restore the pre-slice behavior exactly rather than
/// approximately.
pub fn calibration_skips(
    root: &Path,
    day: &str,
    resolved: &[Resolved],
    selected: &str,
    workload: &str,
) -> Vec<calib::CalibrationCell> {
    let rows = calib::read_day(root, day, calib::READ_CAP);
    calib::calibration(&rows)
        .into_iter()
        .filter(|cell| cell.informs_failover())
        .filter(|cell| cell.workload == workload)
        .filter(|cell| cell.provider != selected)
        .filter(|cell| resolved.iter().any(|entry| entry.id() == cell.provider))
        .collect()
}

/// The cells whose route rows should NAME that the minima needs
/// companions: every roster candidate's cell for the current workload's
/// shape (other than the selected one) whose sample is fit and whose
/// confidence-weighted **insufficient**-rate is at or over
/// [`calib::INSUFFICIENT_RATE_FLOOR`].
///
/// The third vector's read in the walk, and it is deliberately NOT a
/// skip: an insufficient cell's direction was right — the trajectory
/// drove partway and stalled for want of a companion — so the provider
/// is passed over by nothing. What the cell earns is a clause in the
/// decision row's reason naming the companions the minima needs, so a
/// later reader sees why the walk behaved as it did rather than having
/// to infer it. Pure and gated by its caller on the same config gate as
/// [`calibration_skips`].
pub fn calibration_notes(
    root: &Path,
    day: &str,
    resolved: &[Resolved],
    selected: &str,
    workload: &str,
) -> Vec<calib::CalibrationCell> {
    let rows = calib::read_day(root, day, calib::READ_CAP);
    calib::calibration(&rows)
        .into_iter()
        .filter(|cell| cell.notes_companions())
        // A cell that is also fit-and-wrong is already a skip; the skip's
        // own clause names it, and a second clause would say less.
        .filter(|cell| !cell.informs_failover())
        .filter(|cell| cell.workload == workload)
        .filter(|cell| cell.provider != selected)
        .filter(|cell| resolved.iter().any(|entry| entry.id() == cell.provider))
        .collect()
}

/// The clause a companion-noting cell rides in the decision row's
/// reason: the cell's own numbers, so the row says which
/// (provider × shape) cell named the missing companions and at what
/// rate. It is informational — it moves no candidate.
pub fn calibration_note_clause(cell: &calib::CalibrationCell) -> String {
    format!(
        "; calibration notes {} for shape `{}`: {:.2} insufficient on a fit cell of {} rows \
         (span {:.2}) — the direction is right, the minima needs companions, and the provider \
         is not skipped",
        cell.provider,
        cell.shape(),
        cell.weighted_insufficient_rate,
        cell.judged,
        cell.confidence_span,
    )
}

/// One walk's order: (provider id, model) pairs, in the order the walk
/// tries them.
type ProviderModels = Vec<(String, String)>;

/// Reorder one walk's order for this session: the selected candidate
/// keeps the head — the judgment is untouched — every other candidate
/// keeps its preference order, and the demoted ones move to the end. A
/// candidate that is both selected and demoted stays the head:
/// demotion reorders the walk, it does not override the selection.
pub fn reorder_for_demotion(
    order: &[(String, String)],
    selected: &str,
    demoted: &[String],
) -> Vec<(String, String)> {
    let (demoted_entries, mut kept): (ProviderModels, ProviderModels) = order
        .iter()
        .cloned()
        .partition(|(provider, _)| demoted.iter().any(|id| id == provider) && provider != selected);
    let mut ordered = Vec::with_capacity(order.len());
    if let Some(index) = kept.iter().position(|(provider, _)| provider == selected) {
        ordered.push(kept.remove(index));
    }
    ordered.append(&mut kept);
    ordered.extend(demoted_entries);
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
    let ready = resolved
        .iter()
        .filter(|entry| provider_state(entry) == ProviderState::Ready)
        .count();
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
        header_right.push_str(&format!(
            " {}\u{2605} head{}",
            palette::ACCENT,
            palette::RESET
        ));
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
        format!(
            "{DIM}\u{2502}{RESET} {TEXT}{url}{RESET}",
            DIM = palette::DIM,
            RESET = palette::RESET,
            TEXT = palette::TEXT,
            url = entry.candidate.base_url
        ),
        format!(
            "{DIM}\u{2502}{RESET} {TEXT}key {name} \u{b7} {state} \u{b7} model {model}{RESET}",
            DIM = palette::DIM,
            RESET = palette::RESET,
            TEXT = palette::TEXT,
            name = entry.candidate.key_env,
            state = key_state,
            model = entry.candidate.model(),
        ),
    ];
    if let Some(reason) = &entry.candidate.disabled_reason {
        lines.push(format!(
            "{DIM}\u{2502}{RESET} {DIM}{reason}{RESET}",
            DIM = palette::DIM,
            RESET = palette::RESET,
            reason = reason
        ));
    }
    let bottom = format!(
        "{DIM}\u{2514}{bottom}\u{2518}{RESET}",
        DIM = palette::DIM,
        RESET = palette::RESET,
        bottom = "\u{2500}".repeat(CARD_WIDTH - 2)
    );
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
        ClientError::Api(crate::api::ApiError {
            status,
            message: "x".to_string(),
        })
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
        assert_eq!(
            failure_class(&ClientError::Network("connect refused".to_string())),
            "network"
        );
        assert_eq!(
            failure_class(&ClientError::Network("the timeout expired".to_string())),
            "timeout"
        );
        assert_eq!(
            failure_class(&ClientError::Decode("bad sse".to_string())),
            "decode"
        );
    }

    #[test]
    fn every_failure_class_but_a_client_error_fails_over() {
        for status in [401u16, 402, 403, 408, 429, 500, 502, 503, 504] {
            assert!(failover_eligible(&api(status)), "{status} must fail over");
        }
        for status in [400u16, 404] {
            assert!(
                !failover_eligible(&api(status)),
                "{status} must not fail over"
            );
        }
        assert!(failover_eligible(&ClientError::Network(
            "refused".to_string()
        )));
        assert!(failover_eligible(&ClientError::Decode(
            "garbage".to_string()
        )));
    }

    // --- the chain walk ---

    #[test]
    fn the_walk_lands_on_the_first_healthy_candidate_and_records_attempts() {
        // head 402, then 429, then 500, then the fourth serves.
        let (served, attempts) = simulate_walk(
            0,
            &[
                WalkOutcome::Failed {
                    class: "quota",
                    streamed: false,
                },
                WalkOutcome::Failed {
                    class: "rate-limit",
                    streamed: false,
                },
                WalkOutcome::Failed {
                    class: "server",
                    streamed: false,
                },
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
                WalkOutcome::Failed {
                    class: "auth",
                    streamed: false,
                },
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
                WalkOutcome::Failed {
                    class: "network",
                    streamed: true,
                },
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
            &[
                WalkOutcome::Failed {
                    class: "client-error",
                    streamed: false,
                },
                WalkOutcome::Served,
            ],
        );
        assert_eq!(served, None);
        assert_eq!(attempts.len(), 1);
    }

    #[test]
    fn an_exhausted_walk_serves_no_one() {
        let (served, attempts) = simulate_walk(
            0,
            [WalkOutcome::Failed {
                class: "quota",
                streamed: false,
            }; 3]
                .as_slice(),
        );
        assert_eq!(served, None);
        assert_eq!(attempts.len(), 3);
    }

    // --- the sys1 verdict → candidate mapping ---

    #[test]
    fn the_preference_table_is_the_kits_live_table() {
        // The compiled-in default is the live zcode-router-kit's
        // routing.workloads, at (provider, model) granularity, with
        // quick's duplicated stepfun entry deduped.
        let table = default_preferences();
        let quick = preference_for(&table, "quick").expect("quick");
        assert_eq!(
            quick.candidates,
            vec![
                ("stepfun", "step-3.7-flash"),
                ("xiaomi-mimo", "mimo-v2.6-flash"),
                ("zai-coding-plan", "GLM-5.3-Flash"),
            ]
        );
        let hard = preference_for(&table, "hard").expect("hard");
        assert_eq!(hard.candidates[0], ("stepfun", "step-5-preview"));
        // The composed workload targets the grounding provider.
        let composed = preference_for(&table, "composed").expect("composed");
        assert_eq!(composed.candidates[0], ("blend1", "blend1"));
        // No default workload targets the payg reserve — the
        // allowPayg:false policy, so deepseek cannot serve unpinned.
        for preference in &table {
            assert!(
                !preference
                    .candidates
                    .iter()
                    .any(|(provider, _)| *provider == "deepseek"),
                "`{}` targets the reserve",
                preference.workload
            );
        }
        assert!(preference_for(&table, "something_else").is_none());
    }

    #[test]
    fn the_gates_default_to_the_kits_routing_block() {
        let gates = Gates::default();
        assert!((gates.min_confidence - 0.6).abs() < 1e-9);
        assert_eq!(gates.default_workload, "standard_code");
    }

    #[test]
    fn the_workload_map_names_the_first_candidate_of_each_preference() {
        assert_eq!(workload_candidate("quick"), Some("stepfun"));
        assert_eq!(workload_candidate("standard_code"), Some("stepfun"));
        assert_eq!(workload_candidate("hard"), Some("stepfun"));
        assert_eq!(workload_candidate("prose"), Some("stepfun"));
        assert_eq!(workload_candidate("deep_context"), Some("stepfun"));
        assert_eq!(workload_candidate("composed"), Some("blend1"));
        assert_eq!(workload_candidate("something_else"), None);
    }

    #[test]
    fn a_usable_verdict_maps_to_its_candidate() {
        let workload = parse_workload(
            r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard","confidence":0.9}}}}"#,
        );
        assert_eq!(
            workload,
            Some(("hard".to_string(), 0.9, "pass".to_string()))
        );
        // A missing confidence reads as 0.0 — the gate then routes to
        // the default workload rather than trusting a guess.
        let no_confidence = parse_workload(
            r#"{"verdict":"pass","answers":{"glide":{"workload":{"label":"hard"}}}}"#,
        );
        assert_eq!(
            no_confidence,
            Some(("hard".to_string(), 0.0, "pass".to_string()))
        );
    }

    #[test]
    fn an_error_or_absent_verdict_is_unusable() {
        // No provider succeeded: verdict error, no workload answer.
        assert_eq!(parse_workload(r#"{"verdict":"error","answers":{}}"#), None);
        // A declined workload head carries no label.
        assert_eq!(
            parse_workload(
                r#"{"verdict":"pass","answers":{"glide":{"workload":{"_declined":"no-answer"}}}}"#
            ),
            None
        );
        // Not JSON at all.
        assert_eq!(parse_workload("not json"), None);
    }

    #[tokio::test]
    async fn an_unusable_verdict_falls_back_to_the_default_workload() {
        // A roster whose only keyed candidate is the deepseek reserve;
        // the judge cannot run, so the gates' default workload's
        // preference is the walk.
        let resolved = vec![
            Resolved {
                candidate: Candidate {
                    id: "router-kit".into(),
                    base_url: "http://127.0.0.1:8300/v1".into(),
                    key_env: "AGNOSTIC_ROUTER_TOKEN".into(),
                    models: vec!["auto".into()],
                    billing: Billing::Plan,
                    kind: CandidateKind::Http,
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
                    models: vec!["GLM-5.3-Flash".into()],
                    billing: Billing::Plan,
                    kind: CandidateKind::Http,
                    enabled: true,
                    key_required: true,
                    disabled_reason: None,
                },
                key: None,
            },
        ];
        // The judge cannot run: the injected spec names a program that
        // cannot spawn, so `select` falls back. Hermetic on purpose —
        // the vendored sys1 lane IS present in this repo and answers
        // (KI-191's lane fix), and before that fix this test passed
        // only because the lane crashed on every call, which is a test
        // passing for the wrong reason (KI-202).
        let mut specs = HashMap::new();
        specs.insert(
            JUDGE_LANE.to_string(),
            ServerSpec {
                program: "fleet-test-no-such-lane-program".to_string(),
                args: Vec::new(),
                env: Vec::new(),
                passthrough: Vec::new(),
                cwd: None,
            },
        );
        let (decision, _latency) = select(
            &resolved,
            None,
            "do a thing",
            &specs,
            &default_preferences(),
            Gates::default(),
            None,
        )
        .await;
        assert_eq!(decision.how, How::Fallback);
        // The default workload's preference is the walk — stepfun
        // first, exactly as the table says.
        assert_eq!(decision.provider, "stepfun");
        assert_eq!(decision.model, "step-3.7-flash");
        assert_eq!(
            decision.order,
            vec![
                ("stepfun".to_string(), "step-3.7-flash".to_string()),
                ("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string()),
                ("xiaomi-mimo".to_string(), "mimo-v2.6-flash".to_string()),
            ]
        );
        assert!(
            decision.reason.contains("sys1 gave no usable verdict"),
            "{}",
            decision.reason
        );
    }

    #[tokio::test]
    async fn a_provider_pin_routes_every_shape_through_that_provider() {
        // The pin's purpose, in its test: a provider carrying a large
        // unused quota. The router still decides the MODEL — the
        // workload's preference filtered to the pinned provider.
        let resolved = resolve(&default_roster(), &HashMap::new());
        let mut specs = HashMap::new();
        specs.insert(
            JUDGE_LANE.to_string(),
            ServerSpec {
                program: "fleet-test-no-such-lane-program".to_string(),
                args: Vec::new(),
                env: Vec::new(),
                passthrough: Vec::new(),
                cwd: None,
            },
        );
        // No usable verdict → the default workload (standard_code) →
        // its list filtered to stepfun: stepfun's model for that shape.
        let (decision, _latency) = select(
            &resolved,
            Some("stepfun"),
            "anything",
            &specs,
            &default_preferences(),
            Gates::default(),
            None,
        )
        .await;
        assert_eq!(decision.how, How::Pinned);
        assert_eq!(decision.provider, "stepfun");
        assert_eq!(decision.model, "step-3.7-flash");
        assert!(
            decision.reason.contains("pinned to stepfun"),
            "{}",
            decision.reason
        );
    }

    #[tokio::test]
    async fn a_provider_model_pair_pins_both_halves_exactly() {
        let resolved = resolve(&default_roster(), &HashMap::new());
        // An exact pair skips the judgment entirely — no spec needed,
        // which is what makes this hermetic.
        let (decision, _latency) = select(
            &resolved,
            Some("stepfun/step-5-preview"),
            "anything",
            &HashMap::new(),
            &default_preferences(),
            Gates::default(),
            None,
        )
        .await;
        assert_eq!(decision.how, How::Pinned);
        assert_eq!(decision.provider, "stepfun");
        assert_eq!(decision.model, "step-5-preview");
        assert_eq!(
            decision.order,
            vec![("stepfun".to_string(), "step-5-preview".to_string())]
        );
    }

    #[tokio::test]
    async fn an_unknown_pin_is_named_not_silently_defaulted() {
        let resolved = resolve(&default_roster(), &HashMap::new());
        let mut specs = HashMap::new();
        specs.insert(
            JUDGE_LANE.to_string(),
            ServerSpec {
                program: "fleet-test-no-such-lane-program".to_string(),
                args: Vec::new(),
                env: Vec::new(),
                passthrough: Vec::new(),
                cwd: None,
            },
        );
        let (decision, _latency) = select(
            &resolved,
            Some("no-such-provider"),
            "anything",
            &specs,
            &default_preferences(),
            Gates::default(),
            None,
        )
        .await;
        // The selection proceeds normally, and the reason names the bad
        // pin — never a silent substitution.
        assert_eq!(decision.how, How::Fallback);
        assert!(
            decision
                .reason
                .contains("pin `no-such-provider` names no roster provider"),
            "{}",
            decision.reason
        );
    }

    // --- the config merge ---

    #[test]
    fn the_config_file_overrides_entries_field_by_field() {
        let file: ConfigFile = serde_json::from_str(
            r#"{
                "pin": "deepseek",
                "providers": [
                    { "id": "router-kit", "enabled": false },
                    { "id": "zai-coding-plan", "models": ["GLM-5.3-Pro"] },
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
        // The zai model list changed; its endpoint and key env did not.
        let zai = candidates
            .iter()
            .find(|c| c.id == "zai-coding-plan")
            .unwrap();
        assert_eq!(zai.models, vec!["GLM-5.3-Pro".to_string()]);
        assert_eq!(zai.key_env, "ZAI_CODING_API_KEY");
        // The deepseek endpoint changed; its key env did not.
        let deepseek = candidates.iter().find(|c| c.id == "deepseek").unwrap();
        assert_eq!(deepseek.base_url, "https://example.invalid/v1");
        assert_eq!(deepseek.key_env, "DEEPSEEK_API_KEY");
    }

    #[test]
    fn a_singular_model_override_sets_a_one_model_list() {
        // The backward-compatible spelling: `model` is `models: [model]`.
        let file: ConfigFile = serde_json::from_str(
            r#"{ "providers": [ { "id": "xiaomi-mimo", "model": "mimo-v2.6-pro" } ] }"#,
        )
        .unwrap();
        let (candidates, _) = merge(default_roster(), &file).unwrap();
        let mimo = candidates.iter().find(|c| c.id == "xiaomi-mimo").unwrap();
        assert_eq!(mimo.models, vec!["mimo-v2.6-pro".to_string()]);
    }

    #[test]
    fn a_fully_specified_new_entry_is_added() {
        let file: ConfigFile = serde_json::from_str(
            r#"{ "providers": [ { "id": "my-gateway", "base_url": "https://g.example/v1", "key_env": "MY_KEY", "model": "m1" } ] }"#,
        )
        .unwrap();
        let (candidates, _) = merge(default_roster(), &file).unwrap();
        let added = candidates
            .iter()
            .find(|c| c.id == "my-gateway")
            .expect("the new entry");
        assert_eq!(added.base_url, "https://g.example/v1");
        assert!(added.enabled);
        assert!(added.key_required);
    }

    #[test]
    fn a_partial_new_entry_is_a_named_error() {
        let file: ConfigFile =
            serde_json::from_str(r#"{ "providers": [ { "id": "my-gateway", "model": "m1" } ] }"#)
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
        assert_eq!(roster.candidates.len(), 8);
        assert_eq!(roster.candidates[0].id, HEAD_ID);
        assert_eq!(roster.candidates[5].id, "deepseek");
        // Liquid is declared after DeepSeek and enters no declared
        // position: it is available to the owner's config, and moving it
        // into the chain is that config's one-line change.
        assert_eq!(roster.candidates[6].id, "liquid");
        assert_eq!(roster.candidates[7].id, "blend1");
        assert_eq!(roster.pin, None);
        // The preference table and the gates are the compiled-in
        // defaults when no routing.json names any.
        assert_eq!(roster.preferences.len(), 6);
        assert!((roster.gates.min_confidence - 0.6).abs() < 1e-9);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_routing_plugin_file_overrides_the_preference_and_the_gates() {
        let home = std::env::temp_dir().join(format!("fleet-route-routing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        // `hard`'s list is replaced; every other workload keeps the
        // compiled-in default. The gates merge field by field.
        std::fs::write(
            home.join("routing.json"),
            r#"{
                "workloads": [
                    { "workload": "hard", "candidates": ["zai-coding-plan/GLM-5.3-Pro", "stepfun/step-5-preview"] }
                ],
                "gates": { "min_confidence": 0.75 }
            }"#,
        )
        .unwrap();
        let roster = load_roster(&home).unwrap();
        let hard = preference_for(&roster.preferences, "hard").expect("hard");
        assert_eq!(
            hard.candidates,
            vec![
                ("zai-coding-plan", "GLM-5.3-Pro"),
                ("stepfun", "step-5-preview")
            ]
        );
        // The untouched workload keeps its default.
        let quick = preference_for(&roster.preferences, "quick").expect("quick");
        assert_eq!(quick.candidates[0], ("stepfun", "step-3.7-flash"));
        // The named gate moved; the unnamed one kept its default.
        assert!((roster.gates.min_confidence - 0.75).abs() < 1e-9);
        assert_eq!(roster.gates.default_workload, "standard_code");
        // A candidate that is not a provider/model pair is a typed named
        // error, not a silent drop.
        std::fs::write(
            home.join("routing.json"),
            r#"{ "workloads": [ { "workload": "hard", "candidates": ["zai-coding-plan"] } ] }"#,
        )
        .unwrap();
        match load_roster(&home) {
            Err(ConfigError::BadEntry(detail)) => {
                assert!(detail.contains("hard"), "{detail}");
                assert!(detail.contains("provider/model"), "{detail}");
            }
            other => panic!("expected a BadEntry error, got {other:?}"),
        }
        // An unknown field is rejected the way a provider's is.
        std::fs::write(home.join("routing.json"), r#"{ "workload": [] }"#).unwrap();
        assert!(matches!(load_roster(&home), Err(ConfigError::Parse(_))));
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
            model: "GLM-5.3-Flash".to_string(),
            how: How::Judged,
            workload: Some("hard".to_string()),
            verdict: Some("pass".to_string()),
            reason: "sys1 judged `hard`".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![
                ("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string()),
                ("xiaomi-mimo".to_string(), "mimo-v2.6-pro".to_string()),
                ("stepfun".to_string(), "step-5-preview".to_string()),
            ],
        };
        let router = Router::new(resolved, decision);
        lock(&router.state).attempts.extend([
            Attempt {
                provider: "router-kit".to_string(),
                outcome: "failed",
                class: Some("auth"),
            },
            Attempt {
                provider: "zai-coding-plan".to_string(),
                outcome: "served",
                class: None,
            },
        ]);
        // Position 0 of the walk order is zai — the order the decision
        // carries, not the roster's declared order.
        lock(&router.state).served = Some(0);
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
        let order = vec![
            ("router-kit".to_string(), "auto".to_string()),
            ("zai-coding-plan".to_string(), "GLM-5.3-Flash".to_string()),
            ("xiaomi-mimo".to_string(), "mimo-v2.6-flash".to_string()),
        ];
        // A demoted candidate moves to the end; the selected one keeps
        // the head of the walk.
        let ordered = reorder_for_demotion(&order, "zai-coding-plan", &["router-kit".to_string()]);
        let ids: Vec<&str> = ordered
            .iter()
            .map(|(provider, _)| provider.as_str())
            .collect();
        assert_eq!(ids, vec!["zai-coding-plan", "xiaomi-mimo", "router-kit"]);

        // A candidate that is both selected and demoted stays the head:
        // demotion reorders the walk, it never overrides the judgment.
        let ordered = reorder_for_demotion(&order, "router-kit", &["router-kit".to_string()]);
        let ids: Vec<&str> = ordered
            .iter()
            .map(|(provider, _)| provider.as_str())
            .collect();
        assert_eq!(ids, vec!["router-kit", "zai-coding-plan", "xiaomi-mimo"]);
    }

    /// One resolved candidate for the reorder tests.
    fn resolved_for_test(id: &str) -> Resolved {
        Resolved {
            candidate: Candidate {
                id: id.to_string(),
                base_url: "http://127.0.0.1:1/v1".to_string(),
                key_env: format!("{}_KEY", id.to_uppercase().replace('-', "_")),
                models: vec![format!("{id}-model")],
                billing: Billing::Plan,
                kind: CandidateKind::Http,
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
        std::fs::write(
            home.join("config.json"),
            r#"{ "calib": { "logroot": "/x" } }"#,
        )
        .unwrap();
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

    // --- the calibration's ride-back (slice 18) ---

    /// A scratch FLEET_HOME with one config file's text.
    fn config_home(tag: &str, text: &str) -> std::path::PathBuf {
        let home =
            std::env::temp_dir().join(format!("fleet-calib-gate-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("config.json"), text).unwrap();
        home
    }

    #[test]
    fn the_calibration_gate_defaults_on_and_the_config_can_turn_it_off() {
        let home =
            std::env::temp_dir().join(format!("fleet-calib-gate-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        // Absent is on: the ride-back is the slice's point, and turning
        // it off is the owner's explicit choice.
        assert!(calibration_config(&home).informs_failover);
        // A `calib` section that names only the log root leaves it on.
        std::fs::write(
            home.join("config.json"),
            r#"{ "calib": { "log_root": "/tmp/somewhere-else/logs" } }"#,
        )
        .unwrap();
        assert!(calibration_config(&home).informs_failover);
        // Off is a named value, and a misspelled one is rejected the way
        // every other config key is — loud, not silently disabling.
        std::fs::write(
            home.join("config.json"),
            r#"{ "calib": { "informs_failover": false } }"#,
        )
        .unwrap();
        assert!(!calibration_config(&home).informs_failover);
        std::fs::write(
            home.join("config.json"),
            r#"{ "calib": { "inform_failover": false } }"#,
        )
        .unwrap();
        assert!(calibration_config(&home).informs_failover);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_gate_off_leaves_the_pre_slice_behavior_exactly() {
        // The pure half of "a config that turns it off leaves the
        // pre-slice behavior exactly": the skip set AND the companion
        // notes are computed by `prepare` only when the gate is on, so a
        // caller that never calls them gets an empty set and a walk that
        // attempts every candidate — and `Router::new` (the constructor
        // every pre-slice caller uses) carries no skips at all.
        let home = config_home("off", r#"{ "calib": { "informs_failover": false } }"#);
        assert!(!calibration_config(&home).informs_failover);
        let resolved = resolve(&default_roster(), &HashMap::new());
        let decision = RouteDecision {
            provider: HEAD_ID.to_string(),
            model: "auto".to_string(),
            how: How::Fallback,
            workload: None,
            verdict: None,
            reason: "test".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![(HEAD_ID.to_string(), "auto".to_string())],
        };
        let router = Router::new(resolved, decision);
        // A router built the pre-slice way skips nothing, whatever the
        // stream holds: the walk attempts every candidate in order.
        assert!(router.calibration_skips.is_empty());
        assert_eq!(router.calibration_skip(3, 0), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn a_committed_failure_records_the_provider_that_was_committed() {
        // The judged-walk shape: three candidates whose walk order is
        // not the roster order. Position 0 of the walk dials stepfun —
        // roster position 2 — so a committed failure there must record
        // stepfun, never router-kit (KI-208).
        let resolved = vec![
            resolved_for_test("router-kit"),
            resolved_for_test("zai-coding-plan"),
            resolved_for_test("stepfun"),
        ];
        let decision = RouteDecision {
            provider: "stepfun".to_string(),
            model: "stepfun-model".to_string(),
            how: How::Judged,
            workload: Some("hard".to_string()),
            verdict: None,
            reason: "test".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![
                ("stepfun".to_string(), "stepfun-model".to_string()),
                ("router-kit".to_string(), "router-kit-model".to_string()),
                (
                    "zai-coding-plan".to_string(),
                    "zai-coding-plan-model".to_string(),
                ),
            ],
        };
        let router = Router::new(resolved, decision);
        // A previous turn was served by the walk's position 0; this
        // turn's failure is committed, fatal, and recorded.
        {
            let mut state = lock(&router.state);
            state.index = 0;
            state.committed = true;
        }
        let message = crate::api::Message::text(crate::api::Role::User, "hi");
        let request = crate::api::ChatRequest {
            model: "stepfun-model",
            messages: std::slice::from_ref(&message),
            tools: Vec::new(),
        };
        // The endpoint is a closed loopback port: the attempt fails
        // fast, the walk does not advance (committed), and the record
        // is what this test is about.
        let outcome = router.chat_stream(&request, &mut |_| {}).await;
        assert!(outcome.is_err(), "the committed attempt fails");
        let recorded = lock(&router.state).attempts.clone();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].provider, "stepfun");
        assert_eq!(recorded[0].outcome, "failed");

        // The record is the demotion loop's input: three of these
        // failures in a day demote the provider that was dialed, and
        // only it (the loop's arithmetic is calib's own; what this pins
        // is that the name it reads is the dialed one).
        let rows: Vec<calib::ReadRow> = [&recorded[0]; 3]
            .iter()
            .map(|attempt| {
                serde_json::from_str(&format!(
                    r#"{{"op":"route","attempts":[{{"provider":"{}","outcome":"{}"}}]}}"#,
                    attempt.provider, attempt.outcome
                ))
                .unwrap()
            })
            .collect();
        assert_eq!(
            calib::demoted_providers(&rows, calib::DEMOTION_THRESHOLD),
            vec!["stepfun".to_string()]
        );
    }

    /// One judged row as a JSON line, the shape the judgment beat's
    /// writer lands. Written directly rather than through the row type
    /// so the socket-guard's "only calib and judge construct one" stays
    /// literally true of this file.
    fn judged_row(provider: &str, workload: &str, verdict: &str, confidence: f64) -> String {
        serde_json::json!({
            "ts": "2026-10-14T12:00:00Z",
            "op": "route_judged",
            "route_key": format!("{provider}-{workload}-{verdict}-{confidence}"),
            "provider": provider,
            "workload": workload,
            "verdict": verdict,
            "confidence": confidence,
            "reason": "the lane judged the workload",
            "cost_usd": 0.0002,
            "latency_ms": 900,
            "lane": "sys1",
        })
        .to_string()
    }

    /// Write `lines` as one day's stream file under `root`.
    fn write_day(root: &std::path::Path, day: &str, lines: &[String]) {
        let path = calib::day_path(root, day);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
    }

    #[test]
    fn a_fit_and_wrong_cell_is_skipped_for_its_own_shape_only() {
        let root = std::env::temp_dir().join(format!("fleet-calib-skips-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let day = "2026-10-14";
        // zai's `hard` cell: eight judged rows, all wrong, confidences
        // spanning 0.35 — fit, and at or over the floor.
        let mut lines: Vec<String> = (0..calib::FIT_MIN_ROWS)
            .map(|index| {
                judged_row(
                    "zai-coding-plan",
                    "hard",
                    calib::VERDICT_WRONG,
                    0.95 - index as f64 * 0.05,
                )
            })
            .collect();
        // zai's `prose` cell: the same provider, right for a different
        // shape — eight rows, all right. It must never move a `hard`
        // walk, and the `hard` cell must never move a `prose` one.
        lines.extend((0..calib::FIT_MIN_ROWS).map(|index| {
            judged_row(
                "zai-coding-plan",
                "prose",
                calib::VERDICT_RIGHT,
                0.95 - index as f64 * 0.05,
            )
        }));
        // mimo's `hard` cell: one wrong row at full confidence — wrong,
        // but unfit.
        lines.push(judged_row(
            "xiaomi-mimo",
            "hard",
            calib::VERDICT_WRONG,
            0.95,
        ));
        write_day(&root, day, &lines);

        let resolved = resolve(&default_roster(), &HashMap::new());
        // The `hard` walk skips zai, and only because of its `hard` cell.
        let hard = calibration_skips(&root, day, &resolved, HEAD_ID, "hard");
        assert_eq!(hard.len(), 1);
        assert_eq!(hard[0].provider, "zai-coding-plan");
        assert_eq!(hard[0].workload, "hard");
        // The `prose` walk skips nothing: zai's prose cell is right, and
        // its hard cell is not this walk's to consult.
        assert!(calibration_skips(&root, day, &resolved, HEAD_ID, "prose").is_empty());
        // A shape the day holds no cell for moves nothing at all.
        assert!(calibration_skips(&root, day, &resolved, HEAD_ID, "quick").is_empty());
        // The selection is never skipped: its place at the head of the
        // walk is sys1's judgment's.
        assert!(calibration_skips(&root, day, &resolved, "zai-coding-plan", "hard").is_empty());
        // A provider the roster does not carry is never named.
        let one = vec![resolved[0].clone()];
        assert!(calibration_skips(&root, day, &one, HEAD_ID, "hard").is_empty());
        // The two cells are separate records: neither hides the other.
        let cells = calib::calibration(&calib::read_day(&root, day, 512));
        assert_eq!(cells.len(), 3);
        assert_eq!(
            calib::cell(&cells, "zai-coding-plan", "hard").map(|cell| cell.informs_failover()),
            Some(true)
        );
        assert_eq!(
            calib::cell(&cells, "zai-coding-plan", "prose").map(|cell| cell.informs_failover()),
            Some(false)
        );
        assert_eq!(calib::cell(&cells, "zai-coding-plan", "quick"), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_fit_and_insufficient_cell_names_its_companions_and_skips_nothing() {
        // The third vector's read in the walk. zai's `hard` cell is eight
        // judged rows that are all INSUFFICIENT — the direction was right
        // every time and the minima was not reached for want of a
        // companion. The cell names the companions in the decision row's
        // reason, and it skips nothing: only the wrong-rate is an
        // anti-vector, and an incomplete solution is not a failure.
        let root = std::env::temp_dir().join(format!("fleet-calib-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let day = "2026-10-14";
        let lines: Vec<String> = (0..calib::FIT_MIN_ROWS)
            .map(|index| {
                judged_row(
                    "zai-coding-plan",
                    "hard",
                    calib::VERDICT_INSUFFICIENT,
                    0.95 - index as f64 * 0.05,
                )
            })
            .collect();
        write_day(&root, day, &lines);

        let resolved = resolve(&default_roster(), &HashMap::new());
        // The skip set is empty: an insufficient cell moves no candidate.
        assert!(calibration_skips(&root, day, &resolved, HEAD_ID, "hard").is_empty());
        // The note set names the cell, and the clause carries its own
        // numbers so a later reader sees which cell spoke.
        let notes = calibration_notes(&root, day, &resolved, HEAD_ID, "hard");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].provider, "zai-coding-plan");
        assert_eq!(notes[0].workload, "hard");
        assert!(notes[0].weighted_insufficient_rate >= calib::INSUFFICIENT_RATE_FLOOR);
        let clause = calibration_note_clause(&notes[0]);
        assert!(
            clause.contains("calibration notes zai-coding-plan"),
            "{clause}"
        );
        assert!(clause.contains("for shape `hard`"), "{clause}");
        assert!(clause.contains("1.00 insufficient"), "{clause}");
        assert!(clause.contains("is not skipped"), "{clause}");
        // The selection is never noted either: its place at the head of
        // the walk is sys1's judgment's.
        assert!(calibration_notes(&root, day, &resolved, "zai-coding-plan", "hard").is_empty());
        // A shape the day holds no cell for notes nothing.
        assert!(calibration_notes(&root, day, &resolved, HEAD_ID, "quick").is_empty());

        // The fit requirement outranks the rate in this direction too: a
        // single insufficient row at full confidence notes nothing,
        // however stark the number looks.
        let unfit =
            std::env::temp_dir().join(format!("fleet-calib-notes-unfit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&unfit);
        std::fs::create_dir_all(&unfit).unwrap();
        write_day(
            &unfit,
            day,
            &[judged_row(
                "zai-coding-plan",
                "hard",
                calib::VERDICT_INSUFFICIENT,
                0.95,
            )],
        );
        assert!(calibration_notes(&unfit, day, &resolved, HEAD_ID, "hard").is_empty());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&unfit);
    }

    #[test]
    fn the_walk_records_a_calibration_skip_and_names_its_cell_in_the_reason() {
        let resolved = vec![
            resolved_for_test("router-kit"),
            resolved_for_test("zai-coding-plan"),
            resolved_for_test("xiaomi-mimo"),
            resolved_for_test("stepfun"),
        ];
        let decision = RouteDecision {
            provider: "router-kit".to_string(),
            model: "auto".to_string(),
            how: How::Fallback,
            workload: None,
            verdict: None,
            reason: "sys1 gave no usable verdict".to_string(),
            judged_from: crate::route::JudgedFrom::Fresh,
            start_index: 0,
            order: vec![
                ("router-kit".to_string(), "auto".to_string()),
                (
                    "zai-coding-plan".to_string(),
                    "zai-coding-plan-model".to_string(),
                ),
                ("xiaomi-mimo".to_string(), "xiaomi-mimo-model".to_string()),
                ("stepfun".to_string(), "stepfun-model".to_string()),
            ],
        };
        // The walk's preference passes zai over: mimo and stepfun are
        // both usable, so a later healthy candidate exists.
        let cell = calib::CalibrationCell {
            provider: "zai-coding-plan".to_string(),
            workload: "hard".to_string(),
            judged: 8,
            right: 0,
            wrong: 8,
            insufficient: 0,
            waived: 0,
            overridden: 0,
            weighted_wrong_rate: 0.94,
            weighted_insufficient_rate: 0.0,
            confidence_span: 0.35,
            zero_confidence: 0,
            fit: calib::FitState::Fit,
            spend_usd: 0.0016,
            cost_per_right: None,
        };
        let router = Router::build(resolved, decision, vec![cell], &HashMap::new());
        // The head is the walk's start, so it is never skipped; zai is
        // skipped, and the clause names the cell that did it.
        assert_eq!(router.calibration_skip(0, 0), None);
        let clause = router.calibration_skip(1, 0).expect("zai is skipped");
        assert!(
            clause.contains("calibration skipped zai-coding-plan"),
            "{clause}"
        );
        assert!(clause.contains("for shape `hard`"), "{clause}");
        assert!(clause.contains("0.94"), "{clause}");
        assert!(clause.contains("8 rows"), "{clause}");
        // The last candidate is never skipped: a skip that would leave
        // the walk with nothing to try is not a skip.
        assert_eq!(router.calibration_skip(3, 0), None);
    }

    // --- the providers surface ---

    #[test]
    fn the_providers_render_is_cards_with_the_head_marked_and_never_a_table() {
        let resolved = resolve(&default_roster(), &HashMap::new());
        // No keys are set in this process, so the remote plans read `no key`
        // and the loopback kit reads `absent (optional)`.
        let rendered = render(&resolved);
        assert_eq!(rendered.matches('\u{250c}').count(), 8, "{rendered}");
        // The head is marked and named.
        assert!(rendered.contains("head router-kit"), "{rendered}");
        // Key env-var NAMES appear; no value ever does.
        assert!(rendered.contains("ZAI_CODING_API_KEY"), "{rendered}");
        assert!(rendered.contains("DEEPSEEK_API_KEY"), "{rendered}");
        // Liquid rides as a declared card: available to the owner's
        // config, its endpoint and model named.
        assert!(rendered.contains("LIQUID_API_KEY"), "{rendered}");
        assert!(
            rendered.contains("https://api.liquid.ai/openai/v1"),
            "{rendered}"
        );
        assert!(rendered.contains("model lfm2.5-24b-a2b"), "{rendered}");
        // blend1 rides as the grounding provider, a lane-kind candidate.
        assert!(rendered.contains("blend1"), "{rendered}");
        assert!(rendered.contains("http://127.0.0.1:8420"), "{rendered}");
        assert!(rendered.contains("BLEND1_BEARER_TOKEN"), "{rendered}");
        // The endpoints and the default models ride along.
        assert!(
            rendered.contains("https://api.z.ai/api/coding/paas/v4"),
            "{rendered}"
        );
        assert!(rendered.contains("model step-3.7-flash"), "{rendered}");
        assert!(rendered.contains("model deepseek-chat"), "{rendered}");
        // No plain-text table.
        assert!(!rendered.contains("| id |"), "{rendered}");
    }

    #[test]
    fn the_declared_chain_is_the_owners_order() {
        let roster = default_roster();
        let ids: Vec<&str> = roster
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect();
        // blend1 is appended after liquid and asserted here because it
        // changes the compiled-in list — but it enters no declared chain
        // position of its own: the workload table is what targets it.
        // The chain's order is the owner's standing directive and this
        // table does not reorder it.
        assert_eq!(
            ids,
            vec![
                "router-kit",
                "zai-coding-plan",
                "xiaomi-mimo",
                "stepfun",
                "token-plan",
                "deepseek",
                "liquid",
                "blend1"
            ]
        );
        // The kit is the head and needs no key; DeepSeek is the
        // declared order's last plan entry and does; blend1 is a
        // lane-kind candidate that needs no key.
        assert!(!roster[0].key_required);
        assert!(roster[5].key_required);
        assert_eq!(roster[7].id, "blend1");
        assert_eq!(roster[7].kind, CandidateKind::Lane);
        assert!(!roster[7].key_required);
    }
}
