//! The guided onboarding: one sequence, narrated over the registries.
//!
//! The onboarding is narration over a registry, never a second place
//! where truth lives. Every fact a step presents — a lane's name and
//! role, the chain a home's keys build, the projects root the tower
//! scans, the window a model gets — is read from the structure the
//! runtime already reads ([`crate::lanes::LANES`], [`crate::route`],
//! [`crate::boards::resolve_sources`], [`crate::compaction`], the MCP
//! spec's env names). Change a lane's key and the narration follows
//! without an edit; nothing here restates a constant.
//!
//! The arc, in the declared order: the home silently, the first model
//! key, the lanes in lane order, the projects folder, the evidence
//! token, the strategy roster. Presence is free and always checked;
//! the live verify is one bounded call and always opt-in, because it
//! may cost money. A skip is a first-class answer whose consequence is
//! named in the consequence's own words.
//!
//! Nothing here stores state: the steps' completion *is* the machine's
//! state, recomputed at each launch, so `fleet onboard` resumes at the
//! first incomplete step whatever happened in between.

use crate::boards;
use crate::compaction;
use crate::lanes::{self, LaneHealth, LaneState};
use crate::route;
use crate::settings;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

/// The home itself: the layout and the config scaffold. Cheap, silent,
/// and the only thing the critical path needs beside a key.
pub const STEP_HOME: &str = "home";
/// One model key: before the lanes, because every lane composes with a
/// model, not instead of one.
pub const STEP_LLM: &str = "first-llm";
/// The lanes, in the registry's own order.
pub const STEP_LANES: &str = "lanes";
/// The folder the tower watches.
pub const STEP_PROJECTS: &str = "projects";
/// The evidence token: claims reconciled against git.
pub const STEP_GITHUB: &str = "github";
/// The strategy: the roster, its tiers, its ceilings.
pub const STEP_ROSTER: &str = "roster";

/// Every step, in the declared order. The order is the tutorial.
pub const STEPS: &[&str] = &[
    STEP_HOME,
    STEP_LLM,
    STEP_LANES,
    STEP_PROJECTS,
    STEP_GITHUB,
    STEP_ROSTER,
];

/// The layout directories the home step creates — the same names
/// `fleet install` creates, read here so the two cannot drift.
const HOME_LAYOUT: &[&str] = &["sessions", "memories", "skills"];

/// The setting the tower reads its projects root from. The name is the
/// tower's own ([`crate::boards`]); the narration never restates it.
const PROJECTS_SETTING: &str = "TOWER_LOG_PROJECTS_DIR";

/// One step's state, derived — never stored.
#[derive(Debug, Clone)]
pub struct StepState {
    pub id: &'static str,
    /// The step's proof already holds.
    pub done: bool,
    /// The owner declined it (a calibration `lifecycle` row says so).
    pub skipped: bool,
    /// What the machine can show for it, in the owner's words.
    pub evidence: String,
}

/// The whole onboarding's state: the steps, and whether the machine can
/// answer at all (which is what puts the GUI's door up).
#[derive(Debug, Clone)]
pub struct State {
    pub steps: Vec<StepState>,
}

impl State {
    /// The machine can answer: a model key resolves. This is the one
    /// gate the GUI's two states turn on.
    pub fn ready(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.id == STEP_LLM && step.done)
    }

    /// The first step that is neither done nor declined — where a run
    /// resumes. `None` when the sequence is complete.
    pub fn first_incomplete(&self) -> Option<&'static str> {
        self.steps
            .iter()
            .find(|step| !step.done && !step.skipped)
            .map(|step| step.id)
    }

    /// One step's row.
    pub fn step(&self, id: &str) -> Option<&StepState> {
        self.steps.iter().find(|step| step.id == id)
    }

    /// The document the GUI renders and the CLI's `--json` prints. It
    /// carries the state and the narration together: the face paints one
    /// read, and no second place can disagree about what a step says.
    pub fn to_json(&self, facts: &Facts) -> Value {
        json!({
            "type": "onboard",
            "ready": self.ready(),
            "first_incomplete": self.first_incomplete(),
            "steps": self.steps.iter().map(|step| {
                json!({
                    "id": step.id,
                    "done": step.done,
                    "skipped": step.skipped,
                    "evidence": step.evidence,
                    "narration": narration(step.id, facts).to_json(),
                })
            }).collect::<Vec<_>>(),
            "door": facts.door(),
        })
    }
}

/// What one step says. The default path reads `title` and `lede`; the
/// `mechanism` is the disclosure body and the only place a mechanism
/// word is allowed to appear.
#[derive(Debug, Clone)]
pub struct Narration {
    /// Uppercase by CSS in the GUI; lowercase here.
    pub eyebrow: String,
    /// The declarative headline — what is about to happen.
    pub title: String,
    /// The benefit: what the owner gets, in the owner's terms.
    pub lede: String,
    /// The one action's label.
    pub action: String,
    /// The consequence of declining, in the consequence's words. Empty
    /// for the steps that may not be declined.
    pub skip: String,
    /// The "how this works" body — every mechanism word lives here.
    pub mechanism: String,
}

impl Narration {
    fn to_json(&self) -> Value {
        json!({
            "eyebrow": self.eyebrow,
            "title": self.title,
            "lede": self.lede,
            "action": self.action,
            "skip": self.skip,
            "mechanism": self.mechanism,
        })
    }
}

/// The derived facts every narration reads. Built once per call from the
/// structures the runtime itself reads.
pub struct Facts {
    /// The roster's candidates with their keys resolved, in declared
    /// order — never a key's value, only whether it resolved.
    pub resolved: Vec<route::Resolved>,
    /// The providers whose keys resolve, in roster order — the chain a
    /// walk actually closes on.
    pub keyed: Vec<String>,
    /// The fan-in tail's provider id, when the roster declares one.
    pub tail: Option<String>,
    /// The folder the tower scans.
    pub projects_root: std::path::PathBuf,
    /// Each lane's name and role, in the registry's order.
    pub lanes: Vec<(&'static str, &'static str)>,
    /// Whether any lane answers right now.
    pub lanes_answering: usize,
    /// The token name the GitHub server card declares, when it declares
    /// one — the name the PAT writes under.
    pub github_env: Option<String>,
    /// Whether a token under that name is already set.
    pub github_set: bool,
    /// The models the roster declares with a known window, and the
    /// window floor an undeclared model gets.
    pub declared_windows: usize,
    pub window_floor: u64,
    /// The keys the machine already holds (names only), for the door's
    /// "it is already here" press. Never a value.
    pub present_keys: Vec<String>,
}

impl Facts {
    /// Read every fact the narration needs from the registries.
    pub fn read(home: &Path, dotenv: &HashMap<String, String>) -> Facts {
        let roster = route::load_roster(home).ok();
        let (resolved, keyed, tail, declared_windows) = match &roster {
            Some(roster) => {
                let resolved = route::resolve(&roster.candidates, dotenv);
                let keyed: Vec<String> = resolved
                    .iter()
                    .filter(|entry| route::provider_state(entry) == route::ProviderState::Ready)
                    .map(|entry| entry.candidate.id.clone())
                    .collect();
                let tail = roster
                    .candidates
                    .iter()
                    .rev()
                    .find(|candidate| matches!(candidate.billing, route::Billing::Payg))
                    .map(|candidate| candidate.id.clone());
                let declared: usize = resolved
                    .iter()
                    .map(|entry| {
                        entry
                            .candidate
                            .models
                            .iter()
                            .filter(|model| {
                                route::context_window(&entry.candidate.id, model).is_some()
                            })
                            .count()
                    })
                    .sum();
                (resolved, keyed, tail, declared)
            }
            None => (Vec::new(), Vec::new(), None, 0),
        };
        let sources = boards::resolve_sources(home, dotenv);
        let present_keys = resolved
            .iter()
            .filter(|entry| entry.key.is_some())
            .map(|entry| entry.candidate.key_env.clone())
            .collect();
        Facts {
            resolved,
            keyed,
            tail,
            projects_root: sources.tower_root,
            lanes: lanes::LANES
                .iter()
                .map(|lane| (lane.name, lane.role))
                .collect(),
            // Health is a probe and costs a process spawn per lane; the
            // state read says "answering" only from what is already
            // installed, and the live verify is what actually asks.
            lanes_answering: lanes::LANES
                .iter()
                .filter(|lane| lanes::resolve_home_is_present(lane))
                .count(),
            github_env: github_env_name(home),
            github_set: github_env_name(home)
                .map(|name| secret_is_set(home, dotenv, &name))
                .unwrap_or(false),
            declared_windows,
            window_floor: compaction::COMPACTION_TOKEN_BUDGET,
            present_keys,
        }
    }

    /// The door's own copy — the GUI's state one. Benefit first, and no
    /// mechanism word anywhere in the three sentences.
    pub fn door(&self) -> Value {
        let discovered = !self.present_keys.is_empty();
        json!({
            "eyebrow": "fleet · first run",
            "title": if discovered {
                "There is already an account here."
            } else {
                "One account, and fleet can think."
            },
            "lede": "Fleet answers in this window, reads and writes files in a folder you \
                     choose, and remembers what you decide. It needs one AI account, and \
                     nothing else.",
            "discovered": discovered,
            "discovered_label": if discovered {
                "Use the account already on this machine"
            } else {
                ""
            },
            "field_label": "paste the account's key",
            "action": "Use it",
            "mechanism": self.chain_words(),
        })
    }

    /// The chain a walk closes on, in the roster's own terms — the
    /// disclosure body for the steps that may name a provider.
    fn chain_words(&self) -> String {
        if self.keyed.is_empty() {
            return "Nothing here yet. Fleet picks a model per task among the accounts it \
                    can reach, preferring the strongest for hard work and the cheapest for \
                    quick work, and it names the model that served on the route bar."
                .to_string();
        }
        let mut words = format!(
            "Fleet picks a model per task among the accounts it can reach. On this machine \
             {} resolves, so every walk ends on {}.",
            self.keyed.join(", "),
            self.keyed[0]
        );
        if let Some(tail) = &self.tail {
            words.push_str(&format!(
                " {} is the declared last resort: it serves only after every other \
                 candidate has failed, and the decision row names it when it does.",
                tail
            ));
        }
        words
    }
}

/// The name the GitHub server card declares for its token. Read from the
/// MCP registry's default specs, never restated: the registry's rule is
/// names yes, values never.
fn github_env_name(home: &Path) -> Option<String> {
    crate::mcp_registry::github_token_name(home)
}

/// Whether a secret resolves for `name` — the environment first, then
/// the home's dotenv. The value is never read out.
fn secret_is_set(home: &Path, dotenv: &HashMap<String, String>, name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || dotenv
            .get(name)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || crate::home::read_dotenv_with_process_vault(home)
            .get(name)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

/// The state of every step, derived. `probes` supplies lane health when
/// the caller has asked for it (the live verify); the presence read
/// never spawns anything.
pub fn state(home: &Path, dotenv: &HashMap<String, String>) -> State {
    let facts = Facts::read(home, dotenv);
    let decided = decisions(home);
    let steps = STEPS
        .iter()
        .map(|id| {
            let (done, evidence) = presence(home, &facts, id);
            StepState {
                id,
                done,
                skipped: decided.skipped.contains(id),
                evidence,
            }
        })
        .collect();
    State { steps }
}

/// One step's free proof, and what it can show for itself.
fn presence(home: &Path, facts: &Facts, id: &str) -> (bool, String) {
    match id {
        STEP_HOME => {
            let dirs = HOME_LAYOUT
                .iter()
                .filter(|name| home.join(name).is_dir())
                .count();
            let config = home.join("config.json").is_file();
            (
                dirs == HOME_LAYOUT.len() && config,
                if dirs == HOME_LAYOUT.len() && config {
                    format!("{} · {}", home.display(), "the home is in place")
                } else {
                    String::new()
                },
            )
        }
        STEP_LLM => (
            !facts.keyed.is_empty(),
            if facts.keyed.is_empty() {
                String::new()
            } else {
                format!("answers as {}", facts.keyed.join(", "))
            },
        ),
        STEP_LANES => {
            let answering = facts.lanes_answering;
            (
                answering == facts.lanes.len(),
                if answering == 0 {
                    String::new()
                } else {
                    format!("{answering} of {} lanes ready", facts.lanes.len())
                },
            )
        }
        STEP_PROJECTS => {
            let root = &facts.projects_root;
            let known = tower_project_count(root);
            (
                known > 0,
                if known > 0 {
                    format!(
                        "{} · {known} project{}",
                        root.display(),
                        if known == 1 { "" } else { "s" }
                    )
                } else {
                    String::new()
                },
            )
        }
        STEP_GITHUB => (
            facts.github_set,
            if facts.github_set {
                "the token is set".to_string()
            } else {
                String::new()
            },
        ),
        STEP_ROSTER => {
            // The roster is always present (compiled in); what completes
            // this step is the owner's word — a confirmation row, or
            // their own routing choices already on disk.
            let confirmed = decisions(home).confirmed.contains(&STEP_ROSTER)
                || home.join("routing.json").is_file();
            (
                confirmed,
                if confirmed {
                    format!("{} models with declared windows", facts.declared_windows)
                } else {
                    String::new()
                },
            )
        }
        _ => (false, String::new()),
    }
}

/// Whether a path has a `.git` directory — a project the tower can name.
fn tower_project_count(root: &Path) -> usize {
    let Ok(read) = std::fs::read_dir(root) else {
        return 0;
    };
    read.flatten()
        .filter(|entry| entry.path().join(".git").is_dir())
        .count()
}

/// The decisions a previous run recorded: `lifecycle` rows with kind
/// `onboarding`, read from the day files the stream already writes — so
/// a skip or a confirmation survives a restart without any new storage.
#[derive(Debug, Default)]
pub struct Decisions {
    pub skipped: Vec<&'static str>,
    pub confirmed: Vec<&'static str>,
}

/// Read the onboarding's own decisions from the stream's recent days.
pub fn decisions(home: &Path) -> Decisions {
    let root = crate::calib::resolve_root(home, std::env::var("HOME").ok().as_deref());
    let day = crate::calib::today();
    let mut out = Decisions::default();
    for row in crate::calib::lifecycle_rows_recent(&root, &day, 30) {
        if row.kind != "onboarding" {
            continue;
        }
        let Some(step) = STEPS.iter().find(|step| **step == row.name) else {
            continue;
        };
        match row.action.as_str() {
            "skip" if !out.skipped.contains(step) => out.skipped.push(*step),
            "confirm" if !out.confirmed.contains(step) => out.confirmed.push(*step),
            _ => {}
        }
    }
    out
}

/// One step's copy. Every derived fact reads through `facts`; the
/// mechanism body is the only place a mechanism word appears.
pub fn narration(id: &str, facts: &Facts) -> Narration {
    match id {
        STEP_HOME => Narration {
            eyebrow: "fleet · first run".to_string(),
            title: "Your machine, prepared.".to_string(),
            lede: "Fleet keeps everything on this machine: the conversations, the things \
                   it learns, the files it works on. One press sets that up."
                .to_string(),
            action: "Set it up".to_string(),
            skip: String::new(),
            mechanism: format!(
                "The home is {}. It holds {}, plus config.json. Nothing leaves the machine.",
                crate::home::fleet_home(
                    std::env::var("FLEET_HOME").ok().as_deref(),
                    std::env::var("HOME").ok().as_deref(),
                )
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "this machine".to_string()),
                HOME_LAYOUT.join(", ")
            ),
        },
        STEP_LLM => Narration {
            eyebrow: "fleet · a mind".to_string(),
            title: if facts.keyed.is_empty() {
                "One account, and fleet can think.".to_string()
            } else {
                "Fleet can think already.".to_string()
            },
            lede: "Everything else builds on this: answers, file work, and the memory of \
                   what you decided."
                .to_string(),
            action: if facts.keyed.is_empty() {
                "Use an account".to_string()
            } else {
                "Add another".to_string()
            },
            skip: String::new(),
            mechanism: facts.chain_words(),
        },
        STEP_LANES => {
            let names = facts
                .lanes
                .iter()
                .map(|(name, role)| format!("{name} ({role})"))
                .collect::<Vec<_>>()
                .join(", ");
            Narration {
                eyebrow: "fleet · specialists".to_string(),
                title: "Small specialists, called when they help.".to_string(),
                lede: "Fleet can hand narrow jobs to small models that are cheaper and \
                       sharper at them: judging, tables, meaning, images, and composing \
                       the four into prose. Each one is optional."
                    .to_string(),
                action: "Set them up".to_string(),
                skip: "Without them fleet still works; it just does every job with one \
                       model instead of the best tool for each."
                    .to_string(),
                mechanism: format!(
                    "The lanes, in the order they compose: {names}. Each is a separate \
                     local service; fleet starts one only when a task reaches for it."
                ),
            }
        }
        STEP_PROJECTS => Narration {
            eyebrow: "fleet · your landscape".to_string(),
            title: "Point fleet at your work.".to_string(),
            lede: "Give it the folder your repositories live in, and it can read their \
                   branches and commits when you ask about them — and never edits them."
                .to_string(),
            action: "Use this folder".to_string(),
            skip: "Without it, fleet works in whatever folder you name per task and cannot \
                   answer questions about the rest of your projects."
                .to_string(),
            mechanism: format!(
                "The folder is {}. Any repository under it is one fleet can name; the \
                 setting is {}.",
                facts.projects_root.display(),
                PROJECTS_SETTING
            ),
        },
        STEP_GITHUB => Narration {
            eyebrow: "fleet · evidence".to_string(),
            title: "Check claims against the record.".to_string(),
            lede: "When fleet says something was done, this lets it check the claim \
                   against the commits instead of taking anyone's word."
                .to_string(),
            action: "Connect it".to_string(),
            skip: "Without it, claims are still recorded — they just stay unchecked.".to_string(),
            mechanism: format!(
                "It is a read token for GitHub, stored the same way every other secret \
                 is{}.",
                facts
                    .github_env
                    .as_ref()
                    .map(|name| format!(" (under {name})"))
                    .unwrap_or_default()
            ),
        },
        STEP_ROSTER => Narration {
            eyebrow: "fleet · your strategy".to_string(),
            title: "Which model does what.".to_string(),
            lede: "Fleet chooses a model per task. Say which ones you trust for which work, \
                   and what a day may cost, and it follows that instead of guessing."
                .to_string(),
            action: "Review the strategy".to_string(),
            skip: "Without your choices, fleet uses the built-in order: strongest first for \
                   hard work, cheapest first for quick work."
                .to_string(),
            mechanism: format!(
                "{} models carry a declared context window; any other model gets the {} \
                 token floor for compaction. Tiers, billing and the failover order are in \
                 the routing settings.",
                facts.declared_windows, facts.window_floor
            ),
        },
        _ => Narration {
            eyebrow: String::new(),
            title: String::new(),
            lede: String::new(),
            action: String::new(),
            skip: String::new(),
            mechanism: String::new(),
        },
    }
}

/// What a live verify can say. Bounded and opt-in: every arm is one call.
#[derive(Debug, Clone)]
pub struct Verified {
    pub step: &'static str,
    pub ok: bool,
    /// What the owner will see, in plain words.
    pub said: String,
}

/// The lanes' live verify: probe every lane once, name how many answer.
pub async fn verify_lanes() -> Verified {
    let mut answering = 0;
    let mut named: Vec<String> = Vec::new();
    let mut first_reason = String::new();
    for lane in lanes::LANES {
        let health: LaneHealth = lanes::probe_lane(lane).await;
        if health.state == LaneState::Ok {
            answering += 1;
            named.push(lane.name.to_string());
        } else if first_reason.is_empty() {
            first_reason = health.detail;
        }
    }
    if answering == 0 {
        return Verified {
            step: STEP_LANES,
            ok: false,
            said: if first_reason.is_empty() {
                "no specialist answered".to_string()
            } else {
                format!("no specialist answered: {first_reason}")
            },
        };
    }
    Verified {
        step: STEP_LANES,
        ok: true,
        said: format!(
            "{answering} of {} answered: {}",
            lanes::LANES.len(),
            named.join(", ")
        ),
    }
}

/// Record a decline as a calibration `lifecycle` row — the stream's own
/// vocabulary, no new op. The consequence rides the row's `reason`, so
/// the decision is auditable exactly like every other one.
pub fn skip(home: &Path, step: &str, consequence: &str) -> Result<(), String> {
    if !STEPS.contains(&step) {
        return Err(format!("`{step}` is not an onboarding step"));
    }
    if step == STEP_LLM {
        return Err(
            "the first account is the one step that cannot be skipped — without it fleet \
             cannot answer"
                .to_string(),
        );
    }
    let root = crate::calib::resolve_root(home, std::env::var("HOME").ok().as_deref());
    let stream = crate::calib::Stream::open(root);
    crate::calib::record_lifecycle(&stream, "onboarding", step, "skip", consequence);
    stream.flush();
    Ok(())
}

/// Write the first key through the settings writer's secret path. The
/// value is validated and then never read back; a refusal names what was
/// wrong in plain words.
pub fn set_key(home: &Path, name: &str, value: &str) -> Result<(), String> {
    let name = name.trim();
    let value = value.trim();
    if value.is_empty() {
        return Err("that key is empty".to_string());
    }
    if !settings::is_secret_name(name) {
        return Err(format!(
            "`{name}` is not a credential-shaped name; a secret ends in _KEY, _TOKEN, or _SECRET"
        ));
    }
    settings::set_secret(home, name, value).map(|_| ())
}

/// The provider whose key env a pasted value most plausibly belongs to…
/// no: guessing is worse than asking. The door names the provider the
/// owner picked, and this maps a friendly provider id to the env name
/// the roster declares for it.
pub fn key_env_for(home: &Path, provider: &str) -> Option<String> {
    let roster = route::load_roster(home).ok()?;
    roster
        .candidates
        .iter()
        .find(|candidate| candidate.id == provider)
        .map(|candidate| candidate.key_env.clone())
}

/// The provider a key value most plausibly belongs to, from its shape —
/// used only to pre-select the field when the owner pastes without
/// naming an account. Never a guess about validity: a wrong provider is
/// a refused proof, not a silent misroute.
pub fn provider_for_key_shape(value: &str) -> Option<&'static str> {
    let value = value.trim();
    if value.starts_with("sk-ant-") {
        Some("anthropic")
    } else if value.starts_with("sk-proj-") || value.starts_with("sk-svcacct-") {
        Some("openai")
    } else if value.starts_with("gsk_") {
        Some("groq")
    } else if value.starts_with("sk-or-") {
        Some("openrouter")
    } else {
        None
    }
}

/// Remember the onboarding itself — the fact the completion probe reads
/// back. Written through the corpus's own writer, so a fresh home ends
/// with one true fact rather than an empty corpus pretending otherwise.
pub fn remember_onboarding(home: &Path, facts: &Facts) -> Result<String, String> {
    let root = crate::memory::resolve_root_from(home, Some(home));
    std::fs::create_dir_all(&root).map_err(|error| format!("could not make {root:?}: {error}"))?;
    let memory = crate::memory::Memory::load(&root);
    let name = format!("onboarding-{}", crate::boards::now_stamp());
    let body = format!(
        "Fleet was set up on this machine on {}. The account it answers with is {}. The \
         folder it watches for projects is {}. This fact exists so a fresh corpus holds \
         the onboarding's own record.",
        crate::boards::now_stamp(),
        if facts.keyed.is_empty() {
            "not yet chosen".to_string()
        } else {
            facts.keyed.join(", ")
        },
        facts.projects_root.display()
    );
    let request = crate::memory::Remember {
        name: name.clone(),
        description: "fleet's onboarding: when it was set up, what it answers with, and \
                      the folder it watches"
            .to_string(),
        fact_type: "fact".to_string(),
        shape: vec!["onboarding".to_string(), "setup".to_string()],
        phrases: vec!["when was fleet set up".to_string()],
        missing: Vec::new(),
        basis: crate::memory::Basis::ConfirmedBy,
        tier: crate::memory::Tier::Durable,
        valid_until: String::new(),
        body,
    };
    memory
        .remember(&request)
        .map(|fact| fact.name)
        .map_err(|error| format!("the onboarding fact was not written: {error}"))
}

/// The completion probe: the four things that are true when a fleet is
/// ready, each a real read rather than a tick.
#[derive(Debug, Clone)]
pub struct Completion {
    pub doctor: bool,
    pub routed_turn: bool,
    pub recalled_fact: bool,
    pub tower_project: bool,
}

impl Completion {
    pub fn green(&self) -> bool {
        self.doctor && self.routed_turn && self.recalled_fact && self.tower_project
    }

    pub fn to_json(&self) -> Value {
        json!({
            "doctor": self.doctor,
            "routed_turn": self.routed_turn,
            "recalled_fact": self.recalled_fact,
            "tower_project": self.tower_project,
            "green": self.green(),
        })
    }
}

/// Read the completion state.
pub fn completion(home: &Path, dotenv: &HashMap<String, String>) -> Completion {
    let facts = Facts::read(home, dotenv);
    let doctor = facts.ready_providers_gate();
    let recalled_fact =
        !crate::memory::Memory::load(&crate::memory::resolve_root_from(home, Some(home)))
            .recall("when was fleet set up")
            .is_empty();
    let tower_project = tower_project_count(&facts.projects_root) > 0;
    Completion {
        doctor,
        routed_turn: routed_today(home),
        recalled_fact,
        tower_project,
    }
}

/// Whether any route row landed today — the "one turn that routed" the
/// plan names, read from the stream rather than a session scan.
fn routed_today(home: &Path) -> bool {
    let root = crate::calib::resolve_root(home, std::env::var("HOME").ok().as_deref());
    let day = crate::calib::today();
    crate::calib::read_day(&root, &day, 512)
        .iter()
        .any(|row| row.op == crate::calib::OP_ROUTE)
}

impl Facts {
    /// The doctor's credential gate in its own terms: at least one
    /// candidate resolves.
    fn ready_providers_gate(&self) -> bool {
        !self.keyed.is_empty()
    }
}

#[cfg(test)]
mod tests;
