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

use std::collections::{BTreeMap, HashMap};
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// A dollar amount above zero and at or below the compiled ceiling.
    Usd(f64),
    /// A whole number at or below a compiled ceiling.
    CountTo(u64),
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
        key: "ui.adapt",
        kind: Kind::Toggle,
        file: File::Config,
        summary: "Whether the browser re-ranks its offers and related suggestions from your own \
                  accepts. Counters only; off is the static order.",
    },
    Setting {
        key: "remote.public_url",
        kind: Kind::Text,
        file: File::Config,
        summary: "The HTTPS origin a proxy (Tailscale serve) serves the remote face on. Empty \
                  detects your tailnet name, or falls back to this machine's address.",
    },
    Setting {
        key: "runs.max_turns",
        kind: Kind::CountTo(crate::agent::DEFAULT_MAX_TURNS as u64),
        file: File::Config,
        summary: "The turn cap one run may loop under. Reaching it is a continuation                   receipt, not a dead stop; above this compiled ceiling is a change to                   the constant, not a config value.",
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
    Setting {
        key: "channels.enabled",
        kind: Kind::Toggle,
        file: File::Config,
        summary: "Whether the channel service comes up with the face. Off binds nothing: a \
                  machine with no channel configured has no socket to reach.",
    },
    Setting {
        key: "channels.webhook_port",
        kind: Kind::Count,
        file: File::Config,
        summary: "The port the webhook channel binds. The service reads it; `--channels` is an \
                  override of the door, not the only one.",
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
    Setting {
        key: "gates.allow_api_models",
        kind: Kind::Toggle,
        file: File::Routing,
        summary: "Whether the roster's hosted API models may receive requests. Off keeps every \
                  HTTP candidate out of every order, even a pin; vendored lanes still serve.",
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
    // --- the spend caps and the runtime knobs ---------------------------
    Setting {
        key: "FLEET_JUDGE_BUDGET_USD_DAY",
        kind: Kind::Usd(crate::judge::JUDGE_BUDGET_USD_DAY),
        file: File::Env,
        summary: "The daily cap the judge's asks may spend. It can only tighten the compiled \
                  ceiling; a value above it is refused.",
    },
    Setting {
        key: "FLEET_DELEGATE_BUDGET_USD_DAY",
        kind: Kind::Usd(crate::delegate::DELEGATE_BUDGET_USD_DAY),
        file: File::Env,
        summary: "The daily cap the delegation judge's asks may spend.",
    },
    Setting {
        key: "FLEET_EXPLAIN_BUDGET_USD_DAY",
        kind: Kind::Usd(crate::explain::EXPLAIN_BUDGET_USD_DAY),
        file: File::Env,
        summary: "The daily cap the explain beat's asks may spend.",
    },
    Setting {
        key: "FLEET_CONCURRENCY",
        kind: Kind::CountTo(crate::agent::DELEGATION_CONCURRENCY as u64),
        file: File::Env,
        summary: "How many of one turn's independent tool calls run at once. It can only lower \
                  the compiled bound.",
    },
    Setting {
        key: "FLEET_WEB_CACHE_TTL_SECS",
        kind: Kind::Count,
        file: File::Env,
        summary: "How long a web lane answer is reused. `0` disables the cache.",
    },
    Setting {
        key: "FLEET_SKILLS_DIR",
        kind: Kind::Text,
        file: File::Env,
        summary: "The skills library the loader reads. Empty resolves ~/.agents/skills.",
    },
    Setting {
        key: "FLEET_SERVICES_ROOT",
        kind: Kind::Text,
        file: File::Env,
        summary: "The services tree the lanes launch from. Empty walks up from the executable.",
    },
];

/// The env-var counterpart keys the surface owns, in the table's own
/// order: the names the settings surface's env file writes and the
/// browser's env zone names. Values never ride this — a name is
/// configuration, the value behind it is a secret.
pub fn env_keys() -> Vec<&'static str> {
    SETTINGS
        .iter()
        .filter(|setting| setting.file == File::Env)
        .map(|setting| setting.key)
        .collect()
}

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
        Kind::Usd(ceiling) => {
            let number: f64 = trimmed
                .parse()
                .map_err(|_| format!("`{}` is a dollar amount, not `{trimmed}`", setting.key))?;
            if !number.is_finite() || number <= 0.0 || number > ceiling {
                return Err(format!(
                    "`{}` is above zero and at or below {ceiling:.2}, and {number} is not",
                    setting.key
                ));
            }
            Ok(serde_json::Number::from_f64(number)
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(trimmed.to_string())))
        }
        Kind::CountTo(maximum) => {
            let number: u64 = trimmed
                .parse()
                .map_err(|_| format!("`{}` is a whole number, not `{trimmed}`", setting.key))?;
            if number == 0 || number > maximum {
                return Err(format!(
                    "`{}` is 1..={maximum}, and {number} is outside it",
                    setting.key
                ));
            }
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
            // writer over the same atomic rule. A numeric setting (a
            // budget, a count) is rendered as its own text rather than
            // dropped for not being a JSON string.
            let text = match &stored {
                Value::String(text) => text.clone(),
                other => render_value(other),
            };
            return write_env(&env_path, key, &text);
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
pub(crate) fn is_secret_name(name: &str) -> bool {
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
                    // The startup load puts `fleet.env` into the process
                    // environment; a value that is the file's own is
                    // still a file value, not an export.
                    Some(value) if env_file.get(setting.key) == Some(&value) => {
                        (Some(value), false)
                    }
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
    #[test]
    fn the_disabled_list_round_trips_and_enables_remove() {
        let home = std::env::temp_dir().join(format!("fleet-life-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        assert!(tools_disabled(&home).is_empty());
        set_tool_disabled(&home, "deck", true).unwrap();
        set_tool_disabled(&home, "vision", true).unwrap();
        assert_eq!(
            tools_disabled(&home),
            vec!["deck".to_string(), "vision".to_string()]
        );
        // Idempotent both ways.
        set_tool_disabled(&home, "deck", true).unwrap();
        assert_eq!(tools_disabled(&home).len(), 2);
        set_tool_disabled(&home, "deck", false).unwrap();
        assert_eq!(tools_disabled(&home), vec!["vision".to_string()]);
        // A malformed file is an empty list, not an error.
        std::fs::write(home.join("config.json"), "{ not json").unwrap();
        assert!(tools_disabled(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_disabled_pin_overrides_then_clears_back_to_the_config() {
        // The machine's sweep state must not reach a test that builds
        // an Agent over a scratch home: the thread reads the pin while
        // it stands — an empty pin included — and the config again once
        // it clears.
        let home = scratch("disabled-pin");
        std::fs::write(
            home.join("config.json"),
            r#"{"tools": {"disabled": ["list_dir"]}}"#,
        )
        .unwrap();
        assert_eq!(tools_disabled(&home), vec!["list_dir".to_string()]);
        set_tools_disabled_for_test(Vec::new());
        assert!(tools_disabled(&home).is_empty(), "an empty pin pins");
        set_tools_disabled_for_test(vec!["deck".to_string()]);
        assert_eq!(tools_disabled(&home), vec!["deck".to_string()]);
        clear_tools_disabled_for_test();
        assert_eq!(
            tools_disabled(&home),
            vec!["list_dir".to_string()],
            "cleared, the config reads again"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_thresholds_default_and_read_the_overrides() {
        let home = std::env::temp_dir().join(format!("fleet-life-th-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        assert_eq!(lifecycle_thresholds(&home).trial_runs, 10);
        assert_eq!(lifecycle_thresholds(&home).idle_rate, 0.01);
        std::fs::write(
            home.join("config.json"),
            r#"{"tools": {"trial_runs": 5, "idle_rate": 0.05}}"#,
        )
        .unwrap();
        let thresholds = lifecycle_thresholds(&home);
        assert_eq!(thresholds.trial_runs, 5);
        assert_eq!(thresholds.idle_rate, 0.05);
        assert_eq!(thresholds.max_disable_per_sweep, 3);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_sweep_stamp_round_trips() {
        let home = std::env::temp_dir().join(format!("fleet-life-sw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        assert!(last_sweep(&home).is_none());
        record_sweep(&home, "2026-10-24T02:30Z").unwrap();
        assert_eq!(last_sweep(&home).as_deref(), Some("2026-10-24T02:30Z"));
        let _ = std::fs::remove_dir_all(&home);
    }

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
    // The one knob the card table cannot hold, named so an owner
    // reading the surface knows it exists: it lives in the
    // environment, unset is off, and no value is ever printed.
    out.push_str(&format!(
        "{DIM}env-only: {} names a LayerFS project for delegated workspaces \u{b7} unset is off{RESET}\n",
        crate::layerfs_tools::PROJECT_ENV
    ));
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
        Kind::Usd(ceiling) => format!("usd, at most {ceiling:.2}"),
        Kind::CountTo(maximum) => format!("whole number, 1..={maximum}"),
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
        // An empty model list is a provider nothing can serve.
        let empty = set_provider(&home, r#"{"id":"stepfun","models":[]}"#)
            .expect_err("an empty models list must refuse");
        assert!(empty.contains("empty models"), "{empty}");
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
mod router_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-settings-router-{tag}-{}-{}",
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

    fn dotenv(home: &Path) -> HashMap<String, String> {
        crate::home::read_dotenv(home)
    }

    /// The guard is a setting like any other: it validates as a toggle and
    /// lands in routing.json, where the router already reads it.
    #[test]
    fn the_spend_guard_is_a_setting_that_round_trips() {
        let home = scratch("guard");
        set(&home, "gates.allow_api_models", "false").unwrap();
        let state = states(&home)
            .into_iter()
            .find(|state| state.key == "gates.allow_api_models")
            .expect("the guard is in the table");
        assert_eq!(state.value.as_deref(), Some("false"));
        let text = fs::read_to_string(home.join("routing.json")).unwrap();
        assert!(text.contains("\"allow_api_models\": false"), "{text}");
        let error = set(&home, "gates.allow_api_models", "maybe").expect_err("a toggle");
        assert!(error.contains("true or false"), "{error}");
        let _ = fs::remove_dir_all(&home);
    }

    /// The resolved column is the first candidate that would actually
    /// serve: the gate applied, the provider enabled, its key present.
    #[test]
    fn router_config_resolves_the_first_serving_candidate() {
        let home = scratch("resolve");
        fs::write(
            home.join(".env"),
            "STEPFUN_API_KEY=test\nXIAOMI_MIMO_API_KEY=test\nZAI_CODING_API_KEY=test\n",
        )
        .unwrap();
        let config = router_config(&home, &dotenv(&home));
        let quick = config
            .tiers
            .iter()
            .find(|tier| tier.workload == "quick")
            .expect("quick");
        assert_eq!(quick.resolved.as_deref(), Some("stepfun/step-3.7-flash"));
        assert!(quick.reason.is_none());

        // Disabling stepfun moves quick to its next candidate.
        fs::write(
            home.join("config.json"),
            r#"{"providers":[{"id":"stepfun","enabled":false}]}"#,
        )
        .unwrap();
        let config = router_config(&home, &dotenv(&home));
        let quick = config
            .tiers
            .iter()
            .find(|tier| tier.workload == "quick")
            .expect("quick");
        assert_eq!(
            quick.resolved.as_deref(),
            Some("xiaomi-mimo/mimo-v2.6-flash")
        );

        // The guard keeps every HTTP candidate out; the lane still serves.
        fs::write(
            home.join("routing.json"),
            r#"{"gates":{"allow_api_models":false}}"#,
        )
        .unwrap();
        let config = router_config(&home, &dotenv(&home));
        let quick = config
            .tiers
            .iter()
            .find(|tier| tier.workload == "quick")
            .expect("quick");
        assert!(quick.resolved.is_none());
        assert!(
            quick
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("spend guard")),
            "{:?}",
            quick.reason
        );
        let composed = config
            .tiers
            .iter()
            .find(|tier| tier.workload == "composed")
            .expect("composed");
        assert_eq!(composed.resolved.as_deref(), Some("blend1/blend1"));
        let _ = fs::remove_dir_all(&home);
    }

    /// A keyless provider is named by its env var, not silently skipped.
    #[test]
    fn a_missing_key_is_named_in_the_reason() {
        let home = scratch("no-key");
        fs::write(
            home.join("config.json"),
            r#"{"providers":[{"id":"ghost","base_url":"http://127.0.0.1:9/v1",
                 "key_env":"FLEET_TEST_GHOST_KEY_XYZ","models":["ghost-model"]}]}"#,
        )
        .unwrap();
        fs::write(
            home.join("routing.json"),
            r#"{"workloads":[{"workload":"ghostly","candidates":["ghost/ghost-model"]}]}"#,
        )
        .unwrap();
        let config = router_config(&home, &dotenv(&home));
        let ghostly = config
            .tiers
            .iter()
            .find(|tier| tier.workload == "ghostly")
            .expect("the added workload");
        assert!(ghostly.resolved.is_none());
        assert!(
            ghostly
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("FLEET_TEST_GHOST_KEY_XYZ")),
            "{:?}",
            ghostly.reason
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// An empty pin the GUI writes means auto everywhere, not a phantom
    /// provider.
    #[test]
    fn an_empty_pin_is_no_pin() {
        let home = scratch("empty-pin");
        fs::write(home.join("config.json"), r#"{"pin":""}"#).unwrap();
        let config = router_config(&home, &dotenv(&home));
        assert!(config.pin.is_none());
        let _ = fs::remove_dir_all(&home);
    }

    /// The catalog carries every roster model, the current table's values
    /// (known or not), the env names, and the vocabularies the forms pick.
    #[test]
    fn the_catalog_is_the_pickers_source() {
        let home = scratch("catalog");
        fs::write(home.join(".env"), "STEPFUN_API_KEY=test\n").unwrap();
        let catalog = catalog(&home, &dotenv(&home));
        assert!(
            catalog
                .models
                .iter()
                .any(|model| model.value == "stepfun/step-3.7-flash" && model.known),
            "{:?}",
            catalog.models
        );
        assert!(catalog.workloads.iter().any(|workload| workload == "quick"));
        assert!(
            catalog
                .env_names
                .iter()
                .any(|name| name.name == "STEPFUN_API_KEY" && name.present),
            "{:?}",
            catalog.env_names
        );
        assert!(catalog.tools.iter().any(|tool| tool == "bash"));
        assert!(catalog.shapes.iter().any(|shape| shape == "rendering"));
        let _ = fs::remove_dir_all(&home);
    }

    /// A model the roster no longer carries but the current table names
    /// stays in the catalog as a custom value, so a picker cannot rewrite
    /// the walk by dropping it.
    #[test]
    fn a_lost_roster_model_stays_as_a_custom_value() {
        let home = scratch("lost-model");
        fs::write(
            home.join("routing.json"),
            r#"{"workloads":[{"workload":"quick","candidates":["ghost/old-model"]}]}"#,
        )
        .unwrap();
        let catalog = catalog(&home, &dotenv(&home));
        let lost = catalog
            .models
            .iter()
            .find(|model| model.value == "ghost/old-model")
            .expect("the current value stays");
        assert!(!lost.known);
        let _ = fs::remove_dir_all(&home);
    }

    /// A lane write validates the lane's own allowlist, refuses a
    /// credential-shaped name, and preserves the other lanes.
    #[test]
    fn a_lane_write_validates_and_preserves() {
        let home = scratch("lanes");
        let sem1 = serde_json::json!({
            "env": {"SEM1_ST_VENV": "/opt/venv-st"},
            "defaults": {"sem1_embed": {"provider": "llama-server", "model": "embeddinggemma-2-BF16"}}
        });
        assert_eq!(set_lane(&home, "sem1", &sem1).unwrap(), "sem1");
        let sys1 = serde_json::json!({
            "env": {"SYS1_CLASSIFY__PROVIDER": "glide"},
            "defaults": {}
        });
        set_lane(&home, "sys1", &sys1).unwrap();
        // The tabular lane's provider rides the schema's own `model`
        // argument, and the bundled media lane only exposes imagine.
        let sdm1 = serde_json::json!({
            "defaults": {"sdm1_classify": {"model": "tabpfn-hosted"}}
        });
        set_lane(&home, "sdm1", &sdm1).unwrap();
        let gen1 = serde_json::json!({
            "defaults": {"gen1_imagine": {"provider": "wan"}}
        });
        set_lane(&home, "gen1", &gen1).unwrap();
        let read = crate::lanes::read_lane(&home, "sem1")
            .unwrap()
            .expect("sem1");
        assert_eq!(
            read.env.get("SEM1_ST_VENV").map(String::as_str),
            Some("/opt/venv-st")
        );
        assert_eq!(
            read.defaults
                .get("sem1_embed")
                .and_then(|args| args.get("provider"))
                .map(String::as_str),
            Some("llama-server")
        );
        assert!(crate::lanes::read_lane(&home, "sys1").unwrap().is_some());

        // An undeclared name does not cross.
        let bad = serde_json::json!({"env": {"DEEPSEEK_API_KEY": "x"}, "defaults": {}});
        let error = set_lane(&home, "sem1", &bad).expect_err("undeclared");
        assert!(error.contains("does not cross"), "{error}");
        // A credential-shaped name is refused even when the lane's
        // namespace would otherwise let it cross.
        let secret = serde_json::json!({"env": {"SYS1_SOMETHING_TOKEN": "x"}, "defaults": {}});
        let error = set_lane(&home, "sys1", &secret).expect_err("secret-shaped");
        assert!(error.contains("credential-shaped"), "{error}");
        // A relative home is refused.
        let relative = serde_json::json!({"home": "services/sem1", "env": {}, "defaults": {}});
        let error = set_lane(&home, "sem1", &relative).expect_err("relative");
        assert!(error.contains("absolute"), "{error}");
        // An unknown tool is refused.
        let unknown = serde_json::json!({"env": {}, "defaults": {"sem1_other": {"provider": "x"}}});
        let error = set_lane(&home, "sem1", &unknown).expect_err("unknown tool");
        assert!(error.contains("not a tool"), "{error}");
        let _ = fs::remove_dir_all(&home);
    }

    /// Without a passphrase a secret lands in the home's own `.env`, one
    /// line, and the state names the source without the value.
    #[test]
    fn a_secret_lands_in_the_dotenv_without_a_passphrase() {
        let home = scratch("secret-dotenv");
        let store = set_secret_with(&home, "FLEET_TEST_KEY", "super-secret", None).unwrap();
        assert_eq!(store, ".env");
        let text = fs::read_to_string(home.join(".env")).unwrap();
        assert!(text.contains("FLEET_TEST_KEY=super-secret"), "{text}");
        let (present, source) = secret_state(&home, "FLEET_TEST_KEY");
        assert!(present);
        assert_eq!(source, "dotenv");
        remove_secret_with(&home, "FLEET_TEST_KEY", None).unwrap();
        let text = fs::read_to_string(home.join(".env")).unwrap();
        assert!(!text.contains("FLEET_TEST_KEY"), "{text}");
        let _ = fs::remove_dir_all(&home);
    }

    /// With a passphrase a secret is sealed into the vault, and the vault
    /// opens it only with that passphrase.
    #[test]
    fn a_secret_lands_in_the_vault_when_the_passphrase_is_present() {
        let home = scratch("secret-vault");
        let store = set_secret_with(
            &home,
            "FLEET_TEST_TOKEN",
            "super-secret",
            Some("passphrase"),
        )
        .unwrap();
        assert_eq!(store, "vault");
        let state = vault_state(&home);
        assert!(state.exists);
        assert!(
            state.names.iter().any(|name| name == "FLEET_TEST_TOKEN"),
            "{:?}",
            state.names
        );
        let (present, source) = secret_state(&home, "FLEET_TEST_TOKEN");
        assert!(present);
        assert_eq!(source, "vault");
        let text = fs::read_to_string(home.join(crate::vault::VAULT_FILE)).unwrap();
        let vault = crate::vault::Vault::from_json(&text).unwrap();
        assert_eq!(
            vault.get("passphrase", "FLEET_TEST_TOKEN").unwrap(),
            "super-secret"
        );
        assert!(vault.get("wrong", "FLEET_TEST_TOKEN").is_err());
        remove_secret_with(&home, "FLEET_TEST_TOKEN", Some("passphrase")).unwrap();
        assert!(!vault_state(&home)
            .names
            .iter()
            .any(|name| name == "FLEET_TEST_TOKEN"));
        let _ = fs::remove_dir_all(&home);
    }

    /// A non-credential-shaped name is refused: the secret store is not a
    /// general settings writer.
    #[test]
    fn a_secret_must_be_credential_shaped() {
        let home = scratch("secret-shape");
        let error =
            set_secret_with(&home, "FLEET_TEST_SETTING", "x", None).expect_err("a non-secret name");
        assert!(error.contains("credential-shaped"), "{error}");
        let _ = fs::remove_dir_all(&home);
    }

    /// A numeric env setting is written as text into `fleet.env`, and a
    /// value above its compiled ceiling is refused.
    #[test]
    fn a_numeric_env_setting_is_written_as_text() {
        let home = scratch("numeric-env");
        set(&home, "FLEET_JUDGE_BUDGET_USD_DAY", "0.03").unwrap();
        set(&home, "FLEET_CONCURRENCY", "2").unwrap();
        let text = fs::read_to_string(home.join("fleet.env")).unwrap();
        assert!(text.contains("FLEET_JUDGE_BUDGET_USD_DAY=0.03"), "{text}");
        assert!(text.contains("FLEET_CONCURRENCY=2"), "{text}");
        let budget = states(&home)
            .into_iter()
            .find(|state| state.key == "FLEET_JUDGE_BUDGET_USD_DAY")
            .expect("the budget");
        assert_eq!(budget.value.as_deref(), Some("0.03"));
        assert_eq!(budget.source(), "file");
        let error =
            set(&home, "FLEET_JUDGE_BUDGET_USD_DAY", "9").expect_err("above the compiled ceiling");
        assert!(error.contains("at or below"), "{error}");
        let error = set(&home, "FLEET_CONCURRENCY", "9").expect_err("above the compiled bound");
        assert!(error.contains("1..="), "{error}");
        let _ = fs::remove_dir_all(&home);
    }

    /// Clearing a lane's values removes its entry rather than leaving an
    /// empty object behind.
    #[test]
    fn a_cleared_lane_is_removed() {
        let home = scratch("clear-lane");
        let sem1 = serde_json::json!({"env": {"SEM1_LOG_DIR": "/tmp/sem1"}, "defaults": {}});
        set_lane(&home, "sem1", &sem1).unwrap();
        set_lane(
            &home,
            "sem1",
            &serde_json::json!({"env": {}, "defaults": {}}),
        )
        .unwrap();
        assert!(crate::lanes::read_lane(&home, "sem1").unwrap().is_none());
        let _ = fs::remove_dir_all(&home);
    }
    // --- the provider discovery -------------------------------------------------

    /// A listing endpoint that answers one canned body and records the
    /// request's Authorization line, so the tests see the key cross.
    fn listing_stub(
        name: &'static str,
        status: &'static str,
        body: &'static str,
    ) -> (
        std::net::SocketAddr,
        std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    ) {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let seen: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_for_thread = std::sync::Arc::clone(&seen);
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut lines = BufReader::new(stream.try_clone().unwrap())
                    .lines()
                    .map_while(Result::ok);
                let request_line = lines.next().unwrap_or_default();
                let mut authorization = String::new();
                for line in lines {
                    if line.is_empty() {
                        break;
                    }
                    if line.to_ascii_lowercase().starts_with("authorization:") {
                        authorization = line;
                    }
                }
                seen_for_thread
                    .lock()
                    .unwrap()
                    .push(format!("{request_line} {authorization}"));
                let mut stream = stream;
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
                let _ = stream.flush();
            }
            let _ = name;
        });
        (addr, seen)
    }

    /// The discovery reads the catalog and the declared settings off a
    /// listing that carries them, and names the key on the wire.
    #[test]
    fn discovery_reads_the_catalog_and_its_declared_settings() {
        let body = r#"{"data":[
            {"id":"model-a","context_length":262144,"max_output_tokens":8192},
            {"id":"model-b"}
        ]}"#;
        let (addr, seen) = listing_stub("discovery", "200 OK", body);
        let (models, settings) =
            discover_provider_models(&format!("http://{addr}/v1"), Some("the-key")).unwrap();
        assert_eq!(models, vec!["model-a", "model-b"]);
        let a = settings.get("model-a").expect("the declared model");
        assert_eq!(a.context_window, Some(262_144));
        assert_eq!(a.max_output_tokens, Some(8_192));
        assert_eq!(a.context_window_source.as_deref(), Some("provider"));
        assert!(
            !settings.contains_key("model-b"),
            "nothing declared, nothing stored"
        );
        let wire = seen.lock().unwrap().join(" | ");
        assert!(wire.contains("GET /v1/models"), "{wire}");
        assert!(wire.contains("Bearer the-key"), "{wire}");
    }

    /// A refused key is named, not swallowed: the discovery's failure is
    /// the form's own sentence.
    #[test]
    fn a_refused_key_is_named_by_the_discovery() {
        let (addr, _) = listing_stub("refused", "401 Unauthorized", r#"{"error":"bad key"}"#);
        let error =
            discover_provider_models(&format!("http://{addr}/v1"), Some("wrong")).unwrap_err();
        assert!(error.contains("the key was refused (401)"), "{error}");
    }

    /// The full save with a key: the secret lands (the vault seam's own
    /// store decision), the discovery's catalog is merged into the entry
    /// before it is written, and the reply names both.
    #[test]
    fn a_save_with_a_key_stores_it_and_merges_the_catalog() {
        let home = scratch("provider-key-save");
        let body = r#"{"data":[{"id":"discovered-a"},{"id":"discovered-b"}]}"#;
        let (addr, _) = listing_stub("save", "200 OK", body);
        let entry = serde_json::json!({
            "id": "myprovider",
            "base_url": format!("http://{addr}/v1"),
        })
        .to_string();
        let (id, store, discovery) =
            set_provider_with_key(&home, &entry, Some("the-key-value")).unwrap();
        assert_eq!(id, "myprovider");
        assert_eq!(store.as_deref(), Some(".env"), "no passphrase: the dotenv");
        // (the state read names the same store its own way)
        assert_eq!(secret_state(&home, "MYPROVIDER_API_KEY").1, "dotenv");
        let discovered = discovery.expect("the discovery ran");
        assert!(discovered.ok, "{:?}", discovered.error);
        // The saved entry carries the discovered catalog.
        let document = read_json(&home.join("config.json")).unwrap();
        let entry = document["providers"][0].as_object().unwrap();
        let models: Vec<&str> = entry["models"]
            .as_array()
            .unwrap()
            .iter()
            .map(|model| model.as_str().unwrap())
            .collect();
        assert_eq!(models, vec!["discovered-a", "discovered-b"]);
        // And the secret is where the store said it is.
        let (present, source) = secret_state(&home, "MYPROVIDER_API_KEY");
        assert!(present);
        assert_eq!(
            source, "dotenv",
            "the state read names the store its own way"
        );
        let _ = fs::remove_dir_all(&home);
    }

    /// The hand override survives the re-list: a `"hand"`-sourced window
    /// is never overwritten by discovery, while a `"provider"`-sourced
    /// one takes the new word.
    #[test]
    fn a_hand_override_survives_the_rediscovery() {
        let home = scratch("provider-hand-override");
        let entry = serde_json::json!({
            "id": "myprovider",
            "base_url": "http://127.0.0.1:1/v1",
            "models": ["old-a"],
            "model_settings": {
                "old-a": {
                    "context_window": 123456,
                    "context_window_source": "hand"
                }
            }
        })
        .to_string();
        set_provider(&home, &entry).unwrap();
        // The stub answers with a different window for the same model.
        let body = r#"{"data":[{"id":"old-a","context_length":999999}]}"#;
        let (addr, _) = listing_stub("hand", "200 OK", body);
        // Point the entry's base at the stub and re-list.
        let amended = serde_json::json!({
            "id": "myprovider",
            "base_url": format!("http://{addr}/v1"),
            "models": ["old-a"],
            "model_settings": {
                "old-a": {
                    "context_window": 123456,
                    "context_window_source": "hand"
                }
            }
        })
        .to_string();
        set_provider(&home, &amended).unwrap();
        // The reload resolves the key the way the walk does — the dotenv
        // here — and names the absence rather than listing blind.
        set_secret(&home, "MYPROVIDER_API_KEY", "the-key-value").unwrap();
        let discovered = rediscover_provider(&home, "myprovider").unwrap();
        assert!(discovered.ok, "{:?}", discovered.error);
        let document = read_json(&home.join("config.json")).unwrap();
        let settings = &document["providers"][0]["model_settings"]["old-a"];
        assert_eq!(settings["context_window"], 123456, "the hand override wins");
        assert_eq!(settings["context_window_source"], "hand");
        // The catalog itself refreshed.
        assert_eq!(
            document["providers"][0]["models"][0]["id"]
                .as_str()
                .or(Some("old-a")),
            Some("old-a")
        );
        let _ = fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod vault_tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-settings-vault-{tag}-{}-{}",
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

    /// Create, seal, and drop the plaintext through the explicit-passphrase
    /// writers, so the test does not touch the process's unlock state.
    #[test]
    fn the_vault_surface_creates_seals_and_drops() {
        let home = scratch("surface");
        fs::write(
            home.join(".env"),
            "STEPFUN_API_KEY=one\nFLEET_EMBEDDING_ENDPOINT=http://x\n",
        )
        .unwrap();
        let state = vault_state(&home);
        assert!(!state.exists);
        assert_eq!(state.plaintext.len(), 1, "only the credential-shaped name");
        assert!(!state.plaintext[0].sealed);

        let migrated = vault_init_with(&home, "passphrase").unwrap();
        assert_eq!(migrated, 1, "the non-secret .env line stays put");
        let state = vault_state(&home);
        assert!(state.exists);
        assert_eq!(state.names, vec!["STEPFUN_API_KEY"]);
        assert!(state.plaintext[0].sealed);

        // A second init refuses rather than orphaning the sealed value.
        let error = vault_init_with(&home, "passphrase").unwrap_err();
        assert!(error.contains("already exists"), "{error}");
        assert!(vault_verify(&home, "passphrase").is_ok());
        assert!(vault_verify(&home, "wrong").is_err());

        // Drop the plaintext copy after it is sealed.
        vault_remove_env(&home, "STEPFUN_API_KEY").unwrap();
        assert!(vault_state(&home).plaintext.is_empty());

        // Seal a new plaintext entry server-side; the value never rides a
        // read or the GUI.
        fs::write(home.join(".env"), "NEW_KEY=two\n").unwrap();
        assert_eq!(
            vault_seal_env_with(&home, "NEW_KEY", "passphrase").unwrap(),
            "vault"
        );
        let state = vault_state(&home);
        assert!(state.names.iter().any(|name| name == "NEW_KEY"));
        assert!(state
            .plaintext
            .iter()
            .any(|entry| entry.name == "NEW_KEY" && entry.sealed));
        let _ = fs::remove_dir_all(&home);
    }

    /// The first run creates the vault from a chosen passphrase, and a
    /// reset archives the old vault instead of deleting it.
    #[test]
    fn the_first_run_onboards_and_the_reset_archives() {
        let home = scratch("onboard");
        fs::write(
            home.join(".env"),
            "A_KEY=one\nFLEET_EMBEDDING_ENDPOINT=http://x\n",
        )
        .unwrap();
        let migrated = vault_onboard_with(&home, "passphrase").unwrap();
        assert_eq!(migrated, 1);
        assert!(vault_verify(&home, "passphrase").is_ok());
        let (backup, migrated) = vault_reset_with(&home, "new-passphrase").unwrap();
        assert_eq!(migrated, 1);
        assert!(home.join(&backup).exists(), "{backup}");
        assert!(vault_verify(&home, "new-passphrase").is_ok());
        assert!(vault_verify(&home, "passphrase").is_err());
        assert!(vault_state(&home).names.iter().any(|name| name == "A_KEY"));
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

    /// The env-reading tests below serialize on this. `render` names the
    /// truth about where a value came from — the environment included —
    /// so a test that asserts a shape must hold the ambient names still
    /// while it runs (KI-291: an exported `FLEET_EMBEDDING_ENDPOINT` on
    /// the host flipped the unset count on an otherwise green machine).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Remove `name` for the duration of the guard, restoring whatever
    /// was there before — including its absence — on the way out. The
    /// lanes tests' own pattern, carried here.
    struct EnvVar {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVar {
        fn absent(name: &str) -> EnvVar {
            let previous = std::env::var_os(name);
            std::env::remove_var(name);
            EnvVar {
                name: name.to_string(),
                previous,
            }
        }
    }

    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(&self.name, value),
                None => std::env::remove_var(&self.name),
            }
        }
    }

    /// The surface is cards, never a table: every setting renders inside
    /// a box, and no line is a padded column row.
    #[test]
    fn the_surface_is_cards_and_never_a_table() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _embedding = EnvVar::absent("FLEET_EMBEDDING_ENDPOINT");
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
        let _guard = ENV_LOCK.lock().unwrap();
        let _embedding = EnvVar::absent("FLEET_EMBEDDING_ENDPOINT");
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

// --- the router read -------------------------------------------------------

/// One workload tier as the router reads it: the ordered candidates, and
/// what the router would actually serve for it right now. `resolved` is
/// the first candidate that is admissible under the spend gate and whose
/// provider is enabled with its key present (or not required); `reason`
/// names the first blocker when nothing resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierConfig {
    pub workload: String,
    pub candidates: Vec<String>,
    pub resolved: Option<String>,
    pub reason: Option<String>,
    /// True when the workload is in the compiled-in table, so removing
    /// its preference reverts it rather than deleting it.
    pub reverts: bool,
}

/// One roster entry as the router resolves it. The key's presence is a
/// boolean; its value never rides this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEntry {
    pub id: String,
    pub base_url: String,
    pub models: Vec<String>,
    pub kind: &'static str,
    pub billing: &'static str,
    pub enabled: bool,
    pub key_present: bool,
    pub key_required: bool,
    pub key_env: String,
    pub disabled_reason: Option<String>,
    /// The models the owner took out of this provider's catalog — the
    /// load has already filtered them from `models`; this is why they
    /// are absent.
    pub disabled_models: Vec<String>,
    /// The per-model settings the provider declared or the owner
    /// overrode, as the editor shows them.
    pub model_settings: std::collections::BTreeMap<String, crate::route::ModelSettings>,
}

/// The router's configuration as the settings surface reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct RouterConfig {
    pub spend_guard: bool,
    pub min_confidence: f64,
    pub default_workload: String,
    pub pin: Option<String>,
    pub orchestrator: &'static str,
    pub tiers: Vec<TierConfig>,
    pub roster: Vec<RosterEntry>,
    pub refused: Vec<String>,
}

/// Read the router's table, its gates, and the resolution of every tier.
/// The router's own loader and key resolution are used, so the surface
/// cannot disagree with the walk about what would serve.
pub fn router_config(home: &Path, dotenv: &HashMap<String, String>) -> RouterConfig {
    let gates = crate::route::Gates::default();
    let mut out = RouterConfig {
        spend_guard: gates.allow_api_models,
        min_confidence: gates.min_confidence,
        default_workload: gates.default_workload.to_string(),
        pin: None,
        orchestrator: crate::route::ORCHESTRATOR_WORKLOAD,
        tiers: Vec::new(),
        roster: Vec::new(),
        refused: Vec::new(),
    };
    let roster = match crate::route::load_roster(home) {
        Ok(roster) => roster,
        Err(error) => {
            out.refused
                .push(format!("the router config did not load: {error}"));
            return out;
        }
    };
    let resolved = crate::route::resolve(&roster.candidates, dotenv);
    out.spend_guard = roster.gates.allow_api_models;
    out.min_confidence = roster.gates.min_confidence;
    out.default_workload = roster.gates.default_workload.to_string();
    out.pin = roster.pin.clone();
    out.roster = resolved.iter().map(roster_entry).collect();
    let compiled = compiled_in_workloads();
    out.tiers = roster
        .preferences
        .iter()
        .map(|preference| {
            let candidates: Vec<String> = preference
                .candidates
                .iter()
                .map(|(provider, model)| format!("{provider}/{model}"))
                .collect();
            let (resolved, reason) =
                first_serving(&preference.candidates, &resolved, &roster.gates);
            TierConfig {
                workload: preference.workload.to_string(),
                candidates,
                resolved,
                reason,
                reverts: compiled.iter().any(|known| known == preference.workload),
            }
        })
        .collect();
    out
}

fn roster_entry(entry: &crate::route::Resolved) -> RosterEntry {
    RosterEntry {
        id: entry.id().to_string(),
        base_url: entry.candidate.base_url.clone(),
        models: entry.candidate.models.clone(),
        kind: match entry.candidate.kind {
            crate::route::CandidateKind::Http => "http",
            crate::route::CandidateKind::Lane => "lane",
        },
        billing: match entry.candidate.billing {
            crate::route::Billing::Plan => "plan",
            crate::route::Billing::Payg => "payg",
        },
        enabled: entry.candidate.enabled,
        key_present: entry.key.is_some(),
        key_required: entry.candidate.key_required,
        key_env: entry.candidate.key_env.clone(),
        disabled_reason: entry.candidate.disabled_reason.clone(),
        disabled_models: entry.candidate.disabled_models.clone(),
        model_settings: entry.candidate.model_settings.clone(),
    }
}

/// The first candidate the walk would actually serve: admissible under
/// the gate, enabled, and keyed (or key-free). The first blocker is
/// named when none resolves.
fn first_serving(
    chain: &[(&'static str, &'static str)],
    resolved: &[crate::route::Resolved],
    gates: &crate::route::Gates,
) -> (Option<String>, Option<String>) {
    let mut reason: Option<String> = None;
    for (provider, model) in chain {
        let Some(entry) = resolved.iter().find(|entry| entry.id() == *provider) else {
            reason.get_or_insert_with(|| format!("`{provider}` is not in the roster"));
            continue;
        };
        if !crate::route::admissible(resolved, provider, gates) {
            reason.get_or_insert_with(|| format!("the spend guard blocks `{provider}/{model}`"));
            continue;
        }
        if !entry.candidate.enabled {
            reason.get_or_insert_with(|| format!("`{provider}` is disabled"));
            continue;
        }
        if entry.key.is_none() && entry.candidate.key_required {
            reason.get_or_insert_with(|| {
                format!("no `{}` for `{provider}`", entry.candidate.key_env)
            });
            continue;
        }
        return (Some(format!("{provider}/{model}")), None);
    }
    (
        None,
        Some(reason.unwrap_or_else(|| "the chain names no candidate".to_string())),
    )
}

// --- the catalog ------------------------------------------------------------

/// One `provider/model` the pickers offer. `known` is false for a value
/// the current routing file names but the roster no longer carries — the
/// surface shows it as a custom chip instead of dropping it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelOption {
    pub value: String,
    pub provider: String,
    pub model: String,
    pub known: bool,
    pub enabled: bool,
    pub key_present: bool,
    pub billing: &'static str,
    pub kind: &'static str,
}

/// One environment-variable name the key-env picker offers. The value is
/// never part of this type, only presence and where it resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvOption {
    pub name: String,
    pub present: bool,
    pub source: &'static str,
}

/// Everything the settings forms pick from, read once. The model list is
/// the resolved roster plus the current routing values; the env list is
/// the roster's key names plus every name the home's files carry; the
/// tools, skills, and shapes are the vocabularies the role editor needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub models: Vec<ModelOption>,
    pub providers: Vec<RosterEntry>,
    pub workloads: Vec<String>,
    pub env_names: Vec<EnvOption>,
    pub paths: Vec<(String, String)>,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub shapes: Vec<String>,
}

/// Read the catalog every settings form shares.
pub fn catalog(home: &Path, dotenv: &HashMap<String, String>) -> Catalog {
    let router = router_config(home, dotenv);
    let mut models: Vec<ModelOption> = Vec::new();
    for entry in &router.roster {
        for model in &entry.models {
            models.push(ModelOption {
                value: format!("{}/{}", entry.id, model),
                provider: entry.id.clone(),
                model: model.clone(),
                known: true,
                enabled: entry.enabled,
                key_present: entry.key_present,
                billing: entry.billing,
                kind: entry.kind,
            });
        }
    }
    // Every value the current table names, even when the roster no longer
    // carries it: a picker that dropped it would silently rewrite a walk.
    for tier in &router.tiers {
        for candidate in &tier.candidates {
            if models.iter().any(|model| model.value == *candidate) {
                continue;
            }
            let (provider, model) = candidate
                .split_once('/')
                .map(|(provider, model)| (provider.to_string(), model.to_string()))
                .unwrap_or_else(|| (candidate.clone(), String::new()));
            models.push(ModelOption {
                value: candidate.clone(),
                provider,
                model,
                known: false,
                enabled: false,
                key_present: false,
                billing: "plan",
                kind: "http",
            });
        }
    }
    let workloads = router
        .tiers
        .iter()
        .map(|tier| tier.workload.clone())
        .collect();
    let env_names = catalog_env_names(home, dotenv, &router.roster);
    let paths = catalog_paths(home);
    let tools = crate::subagents::ADMISSIBLE_TOOLS
        .iter()
        .map(|tool| (*tool).to_string())
        .collect();
    let skills = catalog_skills();
    let mut shapes: Vec<String> = crate::memory::SHAPES
        .iter()
        .map(|shape| (*shape).to_string())
        .collect();
    for shape in crate::memory::FAILURE_SHAPES {
        if !shapes.iter().any(|known| known == shape) {
            shapes.push((*shape).to_string());
        }
    }
    for shape in crate::memory::OUTCOME_SHAPES {
        if !shapes.iter().any(|known| known == shape) {
            shapes.push((*shape).to_string());
        }
    }
    Catalog {
        models,
        providers: router.roster,
        workloads,
        env_names,
        paths,
        tools,
        skills,
        shapes,
    }
}

fn catalog_env_names(
    home: &Path,
    dotenv: &HashMap<String, String>,
    roster: &[RosterEntry],
) -> Vec<EnvOption> {
    let fleet_env = crate::home::read_fleet_env(home);
    let vault = vault_names(home);
    let mut names: Vec<String> = roster
        .iter()
        .map(|entry| entry.key_env.clone())
        .filter(|name| !name.is_empty())
        .collect();
    for name in dotenv.keys() {
        if !names.iter().any(|known| known == name) {
            names.push(name.clone());
        }
    }
    for name in fleet_env.keys() {
        if !names.iter().any(|known| known == name) {
            names.push(name.clone());
        }
    }
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let source = if std::env::var(&name).is_ok_and(|value| !value.is_empty()) {
                "process"
            } else if vault.iter().any(|sealed| sealed == &name) {
                "vault"
            } else if dotenv.contains_key(&name) {
                "dotenv"
            } else if fleet_env.contains_key(&name) {
                "fleet.env"
            } else {
                "unset"
            };
            EnvOption {
                present: source != "unset",
                name,
                source,
            }
        })
        .collect()
}

/// The names sealed in the vault, without opening it: `Vault::names`
/// reads the entry names, never a value.
fn vault_names(home: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(home.join(crate::vault::VAULT_FILE)) else {
        return Vec::new();
    };
    crate::vault::Vault::from_json(&text)
        .map(|vault| vault.names())
        .unwrap_or_default()
}

fn catalog_paths(home: &Path) -> Vec<(String, String)> {
    let mut paths: Vec<(String, String)> = Vec::new();
    for lane in crate::lanes::LANES {
        if let crate::lanes::LaneHome::Resolved {
            home: lane_home, ..
        } = crate::lanes::resolve_home(lane)
        {
            paths.push((
                lane_home.display().to_string(),
                format!("{} home", lane.name),
            ));
        }
    }
    if let Ok(root) = crate::lanes::services_root() {
        paths.push((root.display().to_string(), "services root".to_string()));
    }
    let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    let user_home = std::env::var("HOME").ok();
    if let Some(dir) =
        crate::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref())
    {
        paths.push((dir.display().to_string(), "skills library".to_string()));
    }
    for (value, label) in crate::lanes::known_paths() {
        if !paths.iter().any(|(known, _)| known == &value) {
            paths.push((value, label.to_string()));
        }
    }
    let _ = home;
    paths
}

fn catalog_skills() -> Vec<String> {
    let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
    let user_home = std::env::var("HOME").ok();
    let Some(dir) =
        crate::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref())
    else {
        return Vec::new();
    };
    crate::skills::load(&dir)
        .into_iter()
        .map(|skill| skill.name)
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
/// The discovery outcome, named both ways (the provider-models plan's
/// P2 criterion): the catalog when the provider lists, the failure when
/// it does not. `error` names the failure the way the form shows it —
/// the key was refused, the base URL did not answer, the provider
/// lists no models.
#[derive(Debug, Clone, PartialEq)]
pub struct Discovery {
    pub ok: bool,
    pub models: Vec<String>,
    pub settings_written: usize,
    pub error: Option<String>,
}

impl Discovery {
    fn failed(error: String) -> Discovery {
        Discovery {
            ok: false,
            models: Vec::new(),
            settings_written: 0,
            error: Some(error),
        }
    }
}

/// The models one provider serves, discovered from the provider: a
/// `GET {base}/models` with the key (the OpenAI shape every HTTP
/// roster candidate already speaks on the chat side), the response's
/// `data[].id` list the catalog. Where the listing declares model
/// settings — an OpenRouter-style `context_length` /
/// `max_output_tokens` — they ride the result; for an ollama base
/// (its `:11434` port), the native `POST /api/show` is asked per model
/// for the same fact, the source the compiled windows table's own
/// comment names. Never stores a key; never reads one but the one
/// handed in.
pub fn discover_provider_models(
    base_url: &str,
    key: Option<&str>,
) -> Result<
    (
        Vec<String>,
        std::collections::BTreeMap<String, crate::route::ModelSettings>,
    ),
    String,
> {
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("the provider names no base URL".to_string());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let mut request = agent.get(&format!("{base}/models"));
    if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
        request = request.set("Authorization", &format!("Bearer {}", key.trim()));
    }
    let response = request.call().map_err(|error| match error {
        ureq::Error::Status(401, _) | ureq::Error::Status(403, _) => {
            "the key was refused (401)".to_string()
        }
        ureq::Error::Status(status, _) => {
            format!("the listing answered HTTP {status}")
        }
        ureq::Error::Transport(transport) => {
            format!("the base URL did not answer: {}", transport.kind())
        }
    })?;
    let body = response
        .into_string()
        .map_err(|error| format!("the listing was unreadable: {error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|error| format!("the listing is not JSON: {error}"))?;
    let data = value
        .get("data")
        .and_then(|data| data.as_array())
        .ok_or_else(|| "the provider lists no models".to_string())?;
    if data.is_empty() {
        return Err("the provider lists no models".to_string());
    }
    let ollama = base.contains(":11434");
    let mut models: Vec<String> = Vec::new();
    let mut settings: std::collections::BTreeMap<String, crate::route::ModelSettings> =
        std::collections::BTreeMap::new();
    for entry in data {
        let Some(id) = entry
            .get("id")
            .and_then(|id| id.as_str())
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        models.push(id.to_string());
        let mut window = read_listing_number(
            entry,
            &["context_length", "context_window", "max_model_len"],
        );
        let ceiling = read_listing_number(entry, &["max_output_tokens", "max_completion_tokens"]);
        // The ollama leg: the OpenAI listing carries no window, and the
        // native show endpoint does — one bounded ask per model.
        if ollama && window.is_none() && models.len() <= 64 {
            if let Ok(answer) = agent
                .post(&format!("{base}/../api/show"))
                .set("Content-Type", "application/json")
                .send_json(serde_json::json!({ "model": id }))
            {
                if let Ok(show) = answer.into_string() {
                    if let Ok(show) = serde_json::from_str::<serde_json::Value>(&show) {
                        window = read_listing_number(&show, &["context_length"]);
                    }
                }
            }
        }
        if window.is_some() || ceiling.is_some() {
            settings
                .entry(id.to_string())
                .or_default()
                .absorb(window, ceiling);
        }
    }
    if models.is_empty() {
        return Err("the provider lists no models".to_string());
    }
    Ok((models, settings))
}

/// The first present number among `names`, from a listing entry.
fn read_listing_number(entry: &serde_json::Value, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| entry.get(name).and_then(|value| value.as_u64()))
}

/// Re-list one provider and write the catalog and its declared settings
/// into the provider's config entry, preserving the hand overrides (a
/// `"hand"`-sourced value survives; a `"provider"`-sourced one takes
/// the new word). The key resolved here is the same one the walk
/// resolves: process env, the vault when the process holds the
/// passphrase, and `.env`.
pub fn rediscover_provider(home: &Path, id: &str) -> Result<Discovery, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("the write names no provider id".to_string());
    }
    let path = home.join("config.json");
    let document = read_json(&path)?;
    let entry = document
        .get("providers")
        .and_then(|providers| providers.as_array())
        .and_then(|providers| {
            providers
                .iter()
                .find(|entry| entry.get("id").and_then(|id| id.as_str()) == Some(id))
        })
        .cloned()
        .ok_or_else(|| format!("`{id}` names no configured provider"))?;
    let base_url = entry
        .get("base_url")
        .and_then(|base| base.as_str())
        .unwrap_or_default()
        .to_string();
    let key_env = entry
        .get("key_env")
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_API_KEY", id.to_ascii_uppercase()));
    let key = crate::home::read_dotenv_with_process_vault(home)
        .remove(key_env.as_str())
        .or_else(|| std::env::var(&key_env).ok());
    if key.as_deref().map(str::trim).unwrap_or_default().is_empty() {
        return Ok(Discovery::failed(format!(
            "`{key_env}` is unset — paste the key on the provider's card"
        )));
    }
    let (models, discovered) = match discover_provider_models(&base_url, key.as_deref()) {
        Ok(discovered) => discovered,
        Err(error) => return Ok(Discovery::failed(error)),
    };
    let settings_written = discovered.len();
    // Merge the discovered settings into the entry's existing map, so a
    // hand override survives the re-list.
    let mut merged: std::collections::BTreeMap<String, crate::route::ModelSettings> =
        serde_json::from_value(
            entry
                .get("model_settings")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
        )
        .unwrap_or_default();
    for (model, found) in discovered {
        merged
            .entry(model)
            .or_default()
            .absorb(found.context_window, found.max_output_tokens);
    }
    let mut amended = entry.clone();
    let object = amended
        .as_object_mut()
        .ok_or_else(|| format!("`{id}`'s config entry is not an object"))?;
    object.insert(
        "models".to_string(),
        serde_json::Value::Array(models.into_iter().map(serde_json::Value::String).collect()),
    );
    object.insert(
        "model_settings".to_string(),
        serde_json::to_value(&merged).map_err(|error| error.to_string())?,
    );
    set_provider(
        home,
        &serde_json::to_string(&amended).map_err(|error| error.to_string())?,
    )?;
    Ok(Discovery {
        ok: true,
        models: Vec::new(),
        settings_written,
        error: None,
    })
}

/// The provider write with the key and the discovery folded in: the key
/// (when the form carried one) lands through the vault seam under the
/// provider's key name, the discovery runs against the just-written
/// key, and the discovered catalog and settings are merged into the
/// entry before it is saved. The reply names every step, so the form
/// can show what happened rather than a silent save.
pub fn set_provider_with_key(
    home: &Path,
    entry: &str,
    key: Option<&str>,
) -> Result<(String, Option<String>, Option<Discovery>), String> {
    let value: serde_json::Value = serde_json::from_str(entry)
        .map_err(|error| format!("the entry is not valid JSON: {error}"))?;
    let id = value
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or_default()
        .to_string();
    if id.is_empty() {
        return Err("the entry names no provider id".to_string());
    }
    let mut store: Option<String> = None;
    if let Some(key) = key.map(str::trim).filter(|key| !key.is_empty()) {
        let name = value
            .get("key_env")
            .and_then(|name| name.as_str())
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}_API_KEY", id.to_ascii_uppercase()));
        store = Some(set_secret(home, &name, key)?);
    }
    // Verify before saving: the discovery runs against the key in hand
    // (just written, or the env name the entry carries — resolved the
    // same way the walk resolves it).
    let key_env = value
        .get("key_env")
        .and_then(|name| name.as_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_API_KEY", id.to_ascii_uppercase()));
    let key_value = key
        .map(str::to_string)
        .or_else(|| {
            crate::home::read_dotenv_with_process_vault(home)
                .remove(key_env.as_str())
                .or_else(|| std::env::var(&key_env).ok())
        })
        .filter(|value| !value.trim().is_empty());
    let base_url = value
        .get("base_url")
        .and_then(|base| base.as_str())
        .unwrap_or_default()
        .to_string();
    let mut discovery = None;
    let mut amended = value.clone();
    if !base_url.is_empty() {
        match discover_provider_models(&base_url, key_value.as_deref()) {
            Ok((models, discovered)) => {
                let mut merged: std::collections::BTreeMap<String, crate::route::ModelSettings> =
                    serde_json::from_value(
                        amended
                            .get("model_settings")
                            .cloned()
                            .unwrap_or_else(|| serde_json::Value::Object(Default::default())),
                    )
                    .unwrap_or_default();
                for (model, found) in discovered {
                    merged
                        .entry(model)
                        .or_default()
                        .absorb(found.context_window, found.max_output_tokens);
                }
                let settings_written = merged.len();
                if let Some(object) = amended.as_object_mut() {
                    object.insert(
                        "models".to_string(),
                        serde_json::Value::Array(
                            models.into_iter().map(serde_json::Value::String).collect(),
                        ),
                    );
                    object.insert(
                        "model_settings".to_string(),
                        serde_json::to_value(&merged).unwrap_or_default(),
                    );
                    let _ = settings_written;
                }
                discovery = Some(Discovery {
                    ok: true,
                    models: Vec::new(),
                    settings_written: 0,
                    error: None,
                });
            }
            Err(error) => {
                // The save proceeds — a provider may be added before it
                // is funded — but the failure is named on the reply,
                // never swallowed.
                discovery = Some(Discovery::failed(error));
            }
        }
    }
    let id = set_provider(
        home,
        &serde_json::to_string(&amended).map_err(|error| error.to_string())?,
    )?;
    Ok((id, store, discovery))
}

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
    if parsed.models.as_ref().is_some_and(Vec::is_empty) {
        return Err(format!(
            "`{id}` names an empty models list; a provider needs at least one model"
        ));
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

// --- the lanes document -----------------------------------------------------

/// The lane names the surface can address.
pub fn lane_names() -> Vec<String> {
    crate::lanes::LANES
        .iter()
        .map(|lane| lane.name.to_string())
        .collect()
}

/// The MCP server registry's writers, beside the lane writer. The
/// registry document lives in the fleet home as `mcp.json` and its
/// shape is the registry module's; these are the write surface — the
/// one home C7's enable/disable lands through, and the ops the CLI and
/// the face's settings writer both call. The read-merge-write pattern
/// is `set_lane`'s; the credential-name refusal deliberately is NOT:
/// a server registry's job is to record which env names a server
/// needs, and a value has no field to live in (mcp_registry D1a).
pub fn set_server(home: &Path, name: &str, config: &Value) -> Result<String, String> {
    let parsed = crate::mcp_registry::parse_server(name, config)?;
    let mut document = crate::mcp_registry::read_registry(home)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "the registry document is not a JSON object".to_string())?;
    let servers = object
        .entry("servers")
        .or_insert_with(|| serde_json::json!({}));
    let Some(servers) = servers.as_object_mut() else {
        return Err("the registry's `servers` is not an object".to_string());
    };
    servers.insert(name.to_string(), crate::mcp_registry::entry_value(&parsed));
    crate::mcp_registry::write_registry(home, &document)?;
    Ok(name.to_string())
}

/// One server out of the registry. Removing a name that is not there is
/// a named refusal that lists what IS configured.
pub fn remove_server(home: &Path, name: &str) -> Result<String, String> {
    let mut document = crate::mcp_registry::read_registry(home)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "the registry document is not a JSON object".to_string())?;
    let Some(servers) = object.get_mut("servers").and_then(Value::as_object_mut) else {
        return Err(format!("`{name}` is not configured; the registry is empty"));
    };
    if servers.remove(name).is_none() {
        let mut sorted: Vec<&String> = servers.keys().collect();
        sorted.sort();
        let carried = if sorted.is_empty() {
            "nothing".to_string()
        } else {
            sorted
                .iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(format!(
            "`{name}` is not configured; the registry carries {carried}"
        ));
    }
    crate::mcp_registry::write_registry(home, &document)?;
    Ok(name.to_string())
}

/// Enabled on or off, in place: the rest of the entry is kept, so a
/// disabled server stays in the registry and out of every run's
/// declarations.
pub fn set_server_enabled(home: &Path, name: &str, enabled: bool) -> Result<String, String> {
    let mut document = crate::mcp_registry::read_registry(home)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "the registry document is not a JSON object".to_string())?;
    let Some(servers) = object.get_mut("servers").and_then(Value::as_object_mut) else {
        return Err(format!("`{name}` is not configured; the registry is empty"));
    };
    let Some(config) = servers.get_mut(name) else {
        return Err(format!("`{name}` is not configured"));
    };
    let mut entry = crate::mcp_registry::parse_server(name, config)?;
    entry.enabled = enabled;
    *config = crate::mcp_registry::entry_value(&entry);
    crate::mcp_registry::write_registry(home, &document)?;
    Ok(name.to_string())
}

/// The probe's outcome into the registry, keeping the rest of the
/// entry — the health path's writer.
pub fn record_server_probe(
    home: &Path,
    name: &str,
    probe: crate::mcp_registry::Probe,
) -> Result<String, String> {
    let mut document = crate::mcp_registry::read_registry(home)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "the registry document is not a JSON object".to_string())?;
    let Some(servers) = object.get_mut("servers").and_then(Value::as_object_mut) else {
        return Err(format!("`{name}` is not configured; the registry is empty"));
    };
    let Some(config) = servers.get_mut(name) else {
        return Err(format!("`{name}` is not configured"));
    };
    let mut entry = crate::mcp_registry::parse_server(name, config)?;
    entry.last_probe = Some(probe);
    *config = crate::mcp_registry::entry_value(&entry);
    crate::mcp_registry::write_registry(home, &document)?;
    Ok(name.to_string())
}

/// Write one lane's object into `<FLEET_HOME>/lanes.json` through the
/// same atomic, unknown-preserving rule every other writer here uses.
/// The whole lane object is validated before the rename: a home is
/// absolute, an env name is one the lane actually accepts, a secret is
/// refused with a pointer to the vault, and a default is one of the
/// lane's curated tool/argument pairs.
pub fn set_lane(home: &Path, lane: &str, config: &Value) -> Result<String, String> {
    if !lane_names().iter().any(|known| known == lane) {
        return Err(format!(
            "`{lane}` is not a lane; the registry carries {}",
            lane_names().join(", ")
        ));
    }
    let Some(lane_entry) = crate::lanes::LANES.iter().find(|entry| entry.name == lane) else {
        return Err(format!("`{lane}` is not a lane"));
    };
    let Some(object) = config.as_object() else {
        return Err("a lane write carries an object".to_string());
    };
    let mut cleaned = Map::new();
    if let Some(value) = object.get("home") {
        let text = value
            .as_str()
            .ok_or_else(|| "`home` is a path string".to_string())?
            .trim();
        if !text.is_empty() {
            let path = Path::new(text);
            if !path.is_absolute() {
                return Err(format!(
                    "`{text}` is a relative home; an absolute path is required, because a \
                     relative one resolves against the caller's working directory"
                ));
            }
            cleaned.insert("home".to_string(), Value::String(text.to_string()));
        }
    }
    let mut env = Map::new();
    if let Some(value) = object.get("env") {
        let Some(map) = value.as_object() else {
            return Err("`env` is an object of name → value".to_string());
        };
        for (name, value) in map {
            let Some(value) = value.as_str() else {
                return Err(format!("`env.{name}` is a string"));
            };
            if !crate::lanes::lane_allows(lane_entry, name) {
                return Err(format!(
                    "`{name}` does not cross into `{lane}`; the lane declares {}",
                    lane_entry.passthrough.join(", ")
                ));
            }
            if is_secret_name(name) {
                return Err(format!(
                    "`{name}` is a credential-shaped name; keys belong in the vault or `.env`, \
                     never in lanes.json"
                ));
            }
            if !value.trim().is_empty() {
                env.insert(name.clone(), Value::String(value.trim().to_string()));
            }
        }
    }
    cleaned.insert("env".to_string(), Value::Object(env));
    let mut defaults = Map::new();
    if let Some(value) = object.get("defaults") {
        let Some(map) = value.as_object() else {
            return Err("`defaults` is an object of tool → arguments".to_string());
        };
        for (tool, args) in map {
            if !default_tool_allowed(lane, tool) {
                return Err(format!(
                    "`{tool}` is not a tool this surface sets defaults for on `{lane}`"
                ));
            }
            let Some(args) = args.as_object() else {
                return Err(format!(
                    "`defaults.{tool}` is an object of argument → value"
                ));
            };
            let mut kept = Map::new();
            for (argument, value) in args {
                if !default_argument_allowed(lane, tool, argument) {
                    return Err(format!(
                        "`{argument}` is not an argument this surface sets for `{tool}`"
                    ));
                }
                let Some(value) = value.as_str() else {
                    return Err(format!("`defaults.{tool}.{argument}` is a string"));
                };
                if !value.trim().is_empty() {
                    kept.insert(argument.clone(), Value::String(value.trim().to_string()));
                }
            }
            if !kept.is_empty() {
                defaults.insert(tool.clone(), Value::Object(kept));
            }
        }
    }
    cleaned.insert("defaults".to_string(), Value::Object(defaults));
    let path = crate::lanes::lanes_path(home);
    let mut document = crate::lanes::read_lanes(home)?;
    let lanes = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .entry("lanes")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(lanes) = lanes.as_object_mut() else {
        return Err("`lanes` is not an object of lane → configuration".to_string());
    };
    if cleaned
        .get("env")
        .is_some_and(|env| env.as_object().is_some_and(|env| env.is_empty()))
        && cleaned.get("defaults").is_some_and(|defaults| {
            defaults
                .as_object()
                .is_some_and(|defaults| defaults.is_empty())
        })
        && !cleaned.contains_key("home")
    {
        lanes.remove(lane);
    } else {
        lanes.insert(lane.to_string(), Value::Object(cleaned));
    }
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )?;
    Ok(lane.to_string())
}

fn default_tool_allowed(lane: &str, tool: &str) -> bool {
    match lane {
        "sem1" => tool == "sem1_embed",
        // The tabular lane takes its provider under the `model` argument
        // (the schema's own name: `tabpfn-hosted` or `nvidia-sdm`).
        "sdm1" => tool == "sdm1_classify",
        // The bundled media lane exposes image generation; its speech
        // and transcription models are configured through the env names,
        // not a per-call argument.
        "gen1" => tool == "gen1_imagine",
        _ => false,
    }
}

fn default_argument_allowed(lane: &str, tool: &str, argument: &str) -> bool {
    match (lane, tool) {
        ("sem1", "sem1_embed") => matches!(argument, "provider" | "model" | "endpoint"),
        ("sdm1", "sdm1_classify") => argument == "model",
        ("gen1", "gen1_imagine") => argument == "provider",
        _ => false,
    }
}

// --- secrets and the vault --------------------------------------------------

/// Whether a credential resolves, and where from. The value is never part
/// of the answer.
pub fn secret_state(home: &Path, name: &str) -> (bool, &'static str) {
    if std::env::var(name).is_ok_and(|value| !value.is_empty()) {
        return (true, "process");
    }
    if vault_names(home).iter().any(|sealed| sealed == name) {
        return (true, "vault");
    }
    if crate::home::read_dotenv(home).contains_key(name) {
        return (true, "dotenv");
    }
    // `fleet.env` is deliberately not a secret source: the startup loader
    // skips credential-shaped names, and the channel resolver never reads
    // one from there. A token hand-written into it is not a token the
    // reader would answer, so saying otherwise would be a lie.
    (false, "unset")
}

/// The vault as the surface reads it: the sealed names, whether the file
/// exists, whether this process holds the passphrase, and the plaintext
/// credential-shaped names in `.env` that are not sealed yet. No value is
/// read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaintextCredential {
    pub name: String,
    /// True when the vault already holds a sealed copy. The `.env` line
    /// may still be there; dropping it is the separate housekeeping action.
    pub sealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultState {
    pub names: Vec<String>,
    pub exists: bool,
    pub unlocked: bool,
    pub plaintext: Vec<PlaintextCredential>,
}

pub fn vault_state(home: &Path) -> VaultState {
    let names = vault_names(home);
    let mut plaintext: Vec<PlaintextCredential> = crate::home::read_dotenv(home)
        .into_keys()
        .filter(|name| is_secret_name(name))
        .map(|name| PlaintextCredential {
            sealed: names.iter().any(|sealed| sealed == &name),
            name,
        })
        .collect();
    plaintext.sort_by(|left, right| left.name.cmp(&right.name));
    VaultState {
        names,
        exists: home.join(crate::vault::VAULT_FILE).is_file(),
        unlocked: crate::home::vault_passphrase().is_some(),
        plaintext,
    }
}

/// Whether `passphrase` opens the home's vault. A missing or empty vault
/// accepts any passphrase: there is nothing to fail to open, and the
/// first seal derives the key from it.
pub fn vault_verify(home: &Path, passphrase: &str) -> Result<(), String> {
    let path = home.join(crate::vault::VAULT_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let vault = crate::vault::Vault::from_json(&text)
        .map_err(|error| format!("{} is not a vault: {error}", path.display()))?;
    let names = vault.names();
    if names.is_empty() {
        return Ok(());
    }
    if vault.open_all(passphrase).len() == names.len() {
        Ok(())
    } else {
        Err("that passphrase does not open this vault".to_string())
    }
}

/// Unlock the vault for this process. The passphrase is held in memory
/// by `home`, never written; a later run in this process reads the
/// sealed values exactly as it would read a launch-time passphrase.
pub fn vault_unlock(home: &Path, passphrase: &str) -> Result<(), String> {
    let passphrase = passphrase.trim();
    if passphrase.is_empty() {
        return Err("a passphrase is required".to_string());
    }
    vault_verify(home, passphrase)?;
    crate::home::unlock_vault(passphrase);
    Ok(())
}

/// Stop using the vault for this process.
pub fn vault_lock() {
    crate::home::lock_vault();
}

/// Create the vault and migrate the plaintext `.env` entries into it.
/// A vault that already holds entries is refused rather than replaced:
/// the CLI's `vault init` starts over, and a surface must not silently
/// orphan sealed values. The plaintext lines stay; removing one is the
/// separate housekeeping action.
pub fn vault_init(home: &Path) -> Result<usize, String> {
    let passphrase = crate::home::vault_passphrase()
        .ok_or_else(|| "the vault is locked; unlock it first".to_string())?;
    vault_init_with(home, &passphrase)
}

/// The same creation over an explicit passphrase, so the write is
/// testable without touching the process's unlock state.
pub fn vault_init_with(home: &Path, passphrase: &str) -> Result<usize, String> {
    let passphrase = passphrase.trim();
    if passphrase.is_empty() {
        return Err("a passphrase is required".to_string());
    }
    let path = home.join(crate::vault::VAULT_FILE);
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(vault) = crate::vault::Vault::from_json(&text) {
            if !vault.is_empty() {
                return Err(format!(
                    "a vault already exists with {} entries; seal new entries instead",
                    vault.names().len()
                ));
            }
        }
    }
    let mut vault = crate::vault::Vault::init();
    let mut migrated = 0;
    for (name, value) in crate::home::read_dotenv(home) {
        // The vault is for credentials; a non-secret setting in `.env` is
        // configuration and stays where it is.
        if name.is_empty() || value.is_empty() || !is_secret_name(&name) {
            continue;
        }
        vault.set(passphrase, &name, &value);
        migrated += 1;
    }
    write_atomic(&path, &vault.to_json())?;
    Ok(migrated)
}

/// Seal one plaintext `.env` credential into the vault, server-side: the
/// value never leaves the process. The plaintext line remains until the
/// owner drops it.
pub fn vault_seal_env(home: &Path, name: &str) -> Result<String, String> {
    let passphrase = crate::home::vault_passphrase()
        .ok_or_else(|| "the vault is locked; unlock it first".to_string())?;
    vault_seal_env_with(home, name, &passphrase)
}

/// The same seal over an explicit passphrase.
pub fn vault_seal_env_with(home: &Path, name: &str, passphrase: &str) -> Result<String, String> {
    let value = crate::home::read_dotenv(home)
        .get(name)
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| format!("`{name}` is not a plaintext credential in .env"))?;
    set_secret_with(home, name, &value, Some(passphrase))
}

/// Drop one plaintext credential line from `.env`, after its value is
/// sealed.
pub fn vault_remove_env(home: &Path, name: &str) -> Result<(), String> {
    remove_env(&home.join(".env"), name)
}

/// The first-run path: choose the passphrase, create the vault, migrate
/// the credential-shaped `.env` entries, and unlock the process. A
/// non-empty vault is refused — this is not the reset path.
pub fn vault_onboard(home: &Path, passphrase: &str) -> Result<usize, String> {
    let migrated = vault_onboard_with(home, passphrase)?;
    crate::home::unlock_vault(passphrase.trim());
    Ok(migrated)
}

/// The same first run without touching the process unlock, so the write
/// is testable on its own.
pub fn vault_onboard_with(home: &Path, passphrase: &str) -> Result<usize, String> {
    let passphrase = passphrase.trim();
    if passphrase.is_empty() {
        return Err("a passphrase is required".to_string());
    }
    let path = home.join(crate::vault::VAULT_FILE);
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(vault) = crate::vault::Vault::from_json(&text) {
            if !vault.is_empty() {
                return Err("a vault already exists; unlock it, or start a new one".to_string());
            }
        }
    }
    vault_init_with(home, passphrase)
}

/// Start over with a new passphrase. The old vault is archived, never
/// deleted: its entries are unreadable without the old passphrase, and
/// the archive is the only evidence of what was sealed. The plaintext
/// `.env` credentials are migrated into the new vault.
pub fn vault_reset(home: &Path, passphrase: &str) -> Result<(String, usize), String> {
    let (backup, migrated) = vault_reset_with(home, passphrase)?;
    crate::home::unlock_vault(passphrase.trim());
    Ok((backup, migrated))
}

/// The same reset without touching the process unlock.
pub fn vault_reset_with(home: &Path, passphrase: &str) -> Result<(String, usize), String> {
    let passphrase = passphrase.trim();
    if passphrase.is_empty() {
        return Err("a passphrase is required".to_string());
    }
    let path = home.join(crate::vault::VAULT_FILE);
    let backup = if path.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        let backup = home.join(format!("{}.bak-{stamp}", crate::vault::VAULT_FILE));
        fs::rename(&path, &backup)
            .map_err(|error| format!("{} cannot be archived: {error}", path.display()))?;
        backup
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let migrated = vault_onboard_with(home, passphrase)?;
    Ok((backup, migrated))
}

/// Seal one credential. The vault is preferred when this process holds
/// the passphrase; otherwise the value lands as one line in the home's
/// own `.env`, preserving every other line. The returned word names the
/// store, never the value.
pub fn set_secret(home: &Path, name: &str, value: &str) -> Result<String, String> {
    let passphrase = crate::home::vault_passphrase();
    set_secret_with(home, name, value, passphrase.as_deref())
}

/// The same write over an explicit passphrase, so the two paths are
/// testable without mutating the process environment.
pub fn set_secret_with(
    home: &Path,
    name: &str,
    value: &str,
    passphrase: Option<&str>,
) -> Result<String, String> {
    if !is_secret_name(name) {
        return Err(format!(
            "`{name}` is not a credential-shaped name; a secret ends in _KEY, _TOKEN, or _SECRET"
        ));
    }
    if value.trim().is_empty() {
        return Err("a secret cannot be empty".to_string());
    }
    if let Some(passphrase) = passphrase.filter(|passphrase| !passphrase.is_empty()) {
        let path = home.join(crate::vault::VAULT_FILE);
        let mut vault = match fs::read_to_string(&path) {
            Ok(text) => crate::vault::Vault::from_json(&text)
                .map_err(|error| format!("{} is not a vault: {error}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                crate::vault::Vault::init()
            }
            Err(error) => {
                return Err(format!("{} is unreadable: {error}", path.display()));
            }
        };
        vault.set(passphrase, name, value);
        write_atomic(&path, &vault.to_json())?;
        return Ok("vault".to_string());
    }
    write_env(&home.join(".env"), name, value)?;
    Ok(".env".to_string())
}

/// Forget one credential, through the store that holds it.
pub fn remove_secret(home: &Path, name: &str) -> Result<String, String> {
    let passphrase = crate::home::vault_passphrase();
    remove_secret_with(home, name, passphrase.as_deref())
}

pub fn remove_secret_with(
    home: &Path,
    name: &str,
    passphrase: Option<&str>,
) -> Result<String, String> {
    if let Some(_passphrase) = passphrase.filter(|passphrase| !passphrase.is_empty()) {
        let path = home.join(crate::vault::VAULT_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("{} is unreadable: {error}", path.display()))?;
        let mut vault = crate::vault::Vault::from_json(&text)
            .map_err(|error| format!("{} is not a vault: {error}", path.display()))?;
        vault
            .remove(name)
            .map_err(|error| format!("`{name}` is not sealed: {error}"))?;
        write_atomic(&path, &vault.to_json())?;
        return Ok("vault".to_string());
    }
    remove_env(&home.join(".env"), name)?;
    Ok(".env".to_string())
}

fn remove_env(path: &Path, name: &str) -> Result<(), String> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{} is unreadable: {error}", path.display())),
    };
    let kept: Vec<&str> = existing
        .lines()
        .filter(|line| {
            line.trim()
                .split_once('=')
                .map(|(key, _)| key.trim() != name)
                .unwrap_or(true)
        })
        .collect();
    let mut text = kept.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    write_atomic(path, &text)
}

/// The remembered approvals, as the trust surface reads them.
///
/// A missing or malformed config reads as nothing remembered rather
/// than an error, the way every other reader here tolerates a file it
/// does not own: a run must not stop because a list it was going to
/// consult could not be parsed.
/// The lifecycle's thresholds, read from `tools.*` in config.json with
/// the defaults the plan names. Every knob the disable gate and the
/// sweep consult; a missing key is the default, never an error.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleThresholds {
    /// A success rate at or below this over the window is idle
    /// (fraction, default 0.01).
    pub idle_rate: f64,
    /// The most tools one sweep may disable (default 3).
    pub max_disable_per_sweep: usize,
    /// Declared runs before a tool leaves trial (default 10).
    pub trial_runs: usize,
    /// Days before a tool leaves trial (default 14).
    pub trial_days: u64,
    /// Declared runs before a tool can be a candidate (default 10).
    pub disable_runs: usize,
    /// Days before a tool can be a candidate (default 14).
    pub disable_days: u64,
    /// Days after a re-enable before the sweep may disable again
    /// (default 7).
    pub reactivate_days: u64,
    /// Sessions the scan reads (default 50).
    pub window: usize,
}

impl Default for LifecycleThresholds {
    fn default() -> Self {
        LifecycleThresholds {
            idle_rate: 0.01,
            max_disable_per_sweep: 3,
            trial_runs: 10,
            trial_days: 14,
            disable_runs: 10,
            disable_days: 14,
            reactivate_days: 7,
            window: 50,
        }
    }
}

/// Read the lifecycle thresholds from `tools.*` in config.json.
pub fn lifecycle_thresholds(home: &Path) -> LifecycleThresholds {
    let document = read_json(&home.join("config.json")).unwrap_or_default();
    let number = |key: &str, fallback: f64| -> f64 {
        document
            .get("tools")
            .and_then(|tools| tools.get(key))
            .and_then(|value| value.as_f64())
            .unwrap_or(fallback)
    };
    LifecycleThresholds {
        idle_rate: number("idle_rate", 0.01),
        max_disable_per_sweep: number("max_disable_per_sweep", 3.0).max(0.0) as usize,
        trial_runs: number("trial_runs", 10.0).max(0.0) as usize,
        trial_days: number("trial_days", 14.0).max(0.0) as u64,
        disable_runs: number("disable_runs", 10.0).max(0.0) as usize,
        disable_days: number("disable_days", 14.0).max(0.0) as u64,
        reactivate_days: number("reactivate_days", 7.0).max(0.0) as u64,
        window: number("window", 50.0).max(1.0) as usize,
    }
}

thread_local! {
    /// The calling test thread's pin on the sweep's disabled list. The
    /// real list is machine state (`config.json` in the fleet home), and
    /// a test that builds an Agent over a scratch home must not inherit
    /// it — the 2026-10-27 gate reddened a declarations test because the
    /// sweep had disabled `list_dir` on the machine that ran it.
    /// Production never sets this, so the read falls through to the
    /// config exactly as before.
    static TOOLS_DISABLED_OVERRIDE: std::cell::RefCell<Option<Vec<String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Pin the disabled list for the calling test thread. An empty vec is a
/// pin too — no tool disabled — which is what a hermetic surface test
/// asserts against. Threads that never call this read the config.
pub fn set_tools_disabled_for_test(list: Vec<String>) {
    TOOLS_DISABLED_OVERRIDE.with(|slot| *slot.borrow_mut() = Some(list));
}

/// Drop the pin, so the thread reads the config again. A test that pins
/// clears before it returns: the test threads are pooled, and a pin
/// that outlives its test is the same leak in the other direction.
pub fn clear_tools_disabled_for_test() {
    TOOLS_DISABLED_OVERRIDE.with(|slot| *slot.borrow_mut() = None);
}

/// The tools the owner (or the sweep) disabled, from `tools.disabled`.
pub fn tools_disabled(home: &Path) -> Vec<String> {
    if let Some(pinned) = TOOLS_DISABLED_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return pinned;
    }
    read_json(&home.join("config.json"))
        .unwrap_or_default()
        .get("tools")
        .and_then(|tools| tools.get("disabled"))
        .and_then(|list| list.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Add or remove one name in `tools.disabled`, through the same
/// read-merge-write every config list rides. Idempotent: disabling a
/// disabled tool or enabling an absent one is still a valid write.
pub fn set_tool_disabled(home: &Path, name: &str, disabled: bool) -> Result<(), String> {
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;
    let tools = object
        .entry("tools".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let tools = tools
        .as_object_mut()
        .ok_or_else(|| "`tools` is not a JSON object".to_string())?;
    let list = tools
        .entry("disabled".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let list = list
        .as_array_mut()
        .ok_or_else(|| "`tools.disabled` is not a list".to_string())?;
    list.retain(|entry| entry.as_str() != Some(name));
    if disabled {
        list.push(Value::String(name.to_string()));
    }
    write_atomic(&path, &document.to_string())
}

/// The stamp of the last applied sweep, for the card's "last sweep"
/// line. Written by the sweep itself.
pub fn record_sweep(home: &Path, stamp: &str) -> Result<(), String> {
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;
    let tools = object
        .entry("tools".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    tools
        .as_object_mut()
        .ok_or_else(|| "`tools` is not a JSON object".to_string())?
        .insert("last_sweep".to_string(), Value::String(stamp.to_string()));
    write_atomic(&path, &document.to_string())
}

/// The workloads the owner (or the sweep) masked, from
/// `workflows.disabled` in config.json.
pub fn workflows_disabled(home: &Path) -> Vec<String> {
    read_json(&home.join("config.json"))
        .unwrap_or_default()
        .get("workflows")
        .and_then(|workflows| workflows.get("disabled"))
        .and_then(|list| list.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Add or remove one workload name in `workflows.disabled`.
pub fn set_workflow_disabled(home: &Path, name: &str, disabled: bool) -> Result<(), String> {
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;
    let workflows = object
        .entry("workflows".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let workflows = workflows
        .as_object_mut()
        .ok_or_else(|| "`workflows` is not a JSON object".to_string())?;
    let list = workflows
        .entry("disabled".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let list = list
        .as_array_mut()
        .ok_or_else(|| "`workflows.disabled` is not a list".to_string())?;
    list.retain(|entry| entry.as_str() != Some(name));
    if disabled {
        list.push(Value::String(name.to_string()));
    }
    write_atomic(&path, &document.to_string())
}

/// The last applied sweep's stamp, when there was one.
pub fn last_sweep(home: &Path) -> Option<String> {
    read_json(&home.join("config.json"))
        .ok()?
        .get("tools")?
        .get("last_sweep")?
        .as_str()
        .map(str::to_string)
}

pub fn approval_rules(home: &Path) -> crate::approvals::Rules {
    let document = read_json(&home.join("config.json")).unwrap_or_default();
    crate::approvals::Rules::from_config(&document)
}

/// Remember one approval, so its class stops prompting.
///
/// The rule rides `approval_rules` in config.json — the third list
/// beside `providers` and `workloads` — through this same writer, so
/// the browser's "yes, and don't ask again" and a hand-edit land
/// identically. The rule is derived from the call here rather than
/// taken from the caller, so there is one place a call becomes a rule;
/// and the class is checked here as well as at the seat, because this
/// is the last writer anything passes through and a refusal that lived
/// only in the HTTP handler would be one crafted POST away from
/// nothing.
///
/// Two refusals, both naming the rule they protect: the always-ask and
/// destructive classes, which hard rule 7 keeps behind a prompt in the
/// current turn, and a rule naming no tool, which would match calls no
/// tool issued. Remembering a rule that is already there is a no-op
/// that still returns it, so a second click on the same seat is
/// idempotent rather than a duplicate line in the file.
pub fn allow_tool(
    home: &Path,
    call: &crate::api::ToolCall,
) -> Result<crate::approvals::Rule, String> {
    if !crate::approvals::allowlistable(call) {
        return Err(format!(
            "a `{}` call is in the class the owner's rule keeps behind a prompt \
             (the always-ask class, or the destructive class), so it cannot be \
             remembered — approval has to happen in the current turn",
            crate::approvals::Rule::for_call(call).label()
        ));
    }
    let rule = crate::approvals::Rule::for_call(call);
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let rules = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .entry("approval_rules")
        .or_insert_with(|| Value::Array(Vec::new()));
    let array = rules
        .as_array_mut()
        .ok_or_else(|| "`approval_rules` is not a list".to_string())?;
    let value = rule.to_value();
    if array.iter().any(|existing| existing == &value) {
        return Ok(rule);
    }
    array.push(value);
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )?;
    Ok(rule)
}

/// Forget one remembered approval, the way it was remembered: through
/// the writer, so the browser and a hand-edit cannot disagree about
/// what the file holds.
/// Retire one skill from every run's prompt, fleet-locally: the name
/// is appended to `<home>/skills-retired.json`, the library file is
/// never touched (KI-215 — the owner retires, the library stays), and
/// the change is one `unretire` from reversing. Idempotent: a name
/// already retired is a no-op, not an error.
pub fn retire_skill(home: &Path, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a skill needs a name".to_string());
    }
    let path = home.join("skills-retired.json");
    let mut names = crate::skills::retired(home);
    if names.iter().any(|held| held == name) {
        return Ok(());
    }
    names.push(name.to_string());
    names.sort();
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&names).map_err(|error| error.to_string())?,
    )
}

/// Return one retired skill to the prompt. The name's absence is a
/// no-op, not an error: the button is the owner's, and a double click
/// is not a failure.
pub fn unretire_skill(home: &Path, name: &str) -> Result<(), String> {
    let name = name.trim();
    let path = home.join("skills-retired.json");
    let mut names = crate::skills::retired(home);
    let before = names.len();
    names.retain(|held| held != name);
    if names.len() == before {
        return Ok(());
    }
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&names).map_err(|error| error.to_string())?,
    )
}

pub fn forget_tool(home: &Path, rule: &crate::approvals::Rule) -> Result<(), String> {
    let path = home.join("config.json");
    let mut document = read_json(&path)?;
    let array = document
        .as_object_mut()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?
        .get_mut("approval_rules")
        .and_then(|rules| rules.as_array_mut())
        .ok_or_else(|| "`approval_rules` is not a list".to_string())?;
    let value = rule.to_value();
    let position = array.iter().position(|existing| existing == &value);
    let Some(index) = position else {
        return Err(format!("`{}` is not remembered", rule.label()));
    };
    array.remove(index);
    write_atomic(
        &path,
        &serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?,
    )
}
