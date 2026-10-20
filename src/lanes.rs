//! The lane registry: the five vendored service cores, where each one
//! launches from, and the live health probe behind `fleet lanes`.
//!
//! Launch details are replicated faithfully from the fork's
//! `@fleet/dsh-components` bundle: `python3 -u <home>/service/mcp_server.py`
//! with `<SVC>_PYTHONPATH=<home>/src` in the child env and the lane's home
//! as the working directory. Home resolution keeps the fork's order — a
//! per-lane environment home, then the bundled copy under `services/`,
//! then unavailable with the reason named — so a live checkout can shadow
//! the vendored copy without touching this file.
//!
//! The child's environment is a decision, not an accident of
//! `tokio::process::Command`: each lane declares the ambient names it
//! legitimately needs (KI-031), and `DEEPSEEK_API_KEY` is on none of the
//! lists. The lanes launch as `python3`, resolved through the child's
//! PATH, which `mcp::BASE_ENV` carries.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::json;

use crate::mcp::{McpError, McpServer, ServerSpec};

use palette::{ACCENT, BOLD, DIM, RESET, TEXT};

/// Environment variable overriding the whole services root.
pub const SERVICES_ROOT_ENV: &str = "FLEET_SERVICES_ROOT";

/// How many ancestors above the executable the services-root walk checks.
/// Twelve covers any real install depth, matching the fork's bound.
const ROOT_WALK_DEPTH: usize = 12;

/// The sibling cores the blended lane composes as path dependencies,
/// wired beside its own `<SVC>_PYTHONPATH` at launch: the judgment,
/// embeddings, tabular and media lanes, in the blend's own order. The
/// vendored copies sit beside the lane under the services root; the
/// cores are locators, never pip-installed into blend1's venv.
const BLEND1_SIBLINGS: &[&str] = &["sys1", "sem1", "sdm1", "gen1"];

/// One lane's static identity: its name, its role in the house pattern,
/// and the two environment variables that address it.
#[derive(Debug, Clone, Copy)]
pub struct Lane {
    pub name: &'static str,
    pub role: &'static str,
    /// The `<SVC>_PYTHONPATH` variable the lane's `mcp_server.py` reads.
    pub python_path_env: &'static str,
    /// The per-lane home override (`FLEET_LANE_<SVC>_HOME`).
    pub home_env: &'static str,
    /// The ambient environment names this lane legitimately needs, copied
    /// into the child only when the caller has them. The child's whole
    /// environment is `mcp::BASE_ENV` plus its own `<SVC>_PYTHONPATH` plus
    /// this list — never the caller's environment (KI-031).
    ///
    /// The rule: a credential the lane's providers read, and a locator for
    /// a machine-local resource the lane cannot discover itself (a venv, an
    /// endpoint, a store root, a log root, an env file). Each entry below
    /// names the source line that reads it.
    pub passthrough: &'static [&'static str],
    /// The tools this lane exposes that report PROVIDER-level
    /// availability, called by the probe beside the handshake. KI-030:
    /// the MCP-surface health (`initialize` + `tools/list`) says the
    /// lane answers, not that its providers do — a lane reports `ok`
    /// while its llama-server is down. sdm1 ships `sdm1_health` and
    /// sem1 a `sem1_probe` that report exactly this; blend1's `doctor`
    /// was the first.
    pub health_tools: &'static [&'static str],
}

/// The six lanes, in launch order.
pub const LANES: &[Lane] = &[
    Lane {
        name: "sys1",
        role: "judgment",
        python_path_env: "SYS1_PYTHONPATH",
        home_env: "FLEET_LANE_SYS1_HOME",
        // The hosted decision providers each read their own key from the
        // environment — `sys1/config.py`'s `<pid>_api_key_env` (decide and
        // glide → FASTINO, jev → TYPESAFE, drex → DREX, julia →
        // SUPERSONIC, liquid → LIQUID, plus clm/openjev/kev/tev1/laya), and
        // `api_key_for` resolves them by name. `SYS1_ENV_FILE` is where
        // `load_provider_env` reads the key file the default resolution
        // already finds; `SYS1_STATE_FILE` and `SYS1_LOG_DIR` are the
        // control-dashboard state and the JSONL log root.
        passthrough: &[
            "FASTINO_API_KEY",
            "TYPESAFE_API_KEY",
            "DREX_API_KEY",
            "SUPERSONIC_API_KEY",
            "LIQUID_API_KEY",
            "CLM_API_KEY",
            "OPENJEV_API_KEY",
            "KEV_API_KEY",
            "TOGETHER_API_KEY",
            "LAYA_API_KEY",
            "SYS1_ENV_FILE",
            "SYS1_STATE_FILE",
            "SYS1_LOG_DIR",
            // KI-039: sys1 reads its whole `SYS1_` namespace — any
            // `SYS1_<SECTION>__<KEY>` override, plus the `SYS1_LOG_DIR`
            // alias — so the allowlist declares the namespace and the
            // child's own config loader picks the names out.
            "SYS1_",
        ],
        health_tools: &[],
    },
    Lane {
        name: "sdm1",
        role: "tabular",
        python_path_env: "SDM1_PYTHONPATH",
        home_env: "FLEET_LANE_SDM1_HOME",
        // `tabpfn_hosted.py` reads TABPFN_API_KEY (the hosted lane's
        // credential — the one the review named), SDM1_TABPFN_BASE and
        // SDM1_TABPFN_TIMEOUT. `nvidia_sdm.py` runs its worker inside a
        // models venv it cannot discover on its own, addressed by
        // SDM1_MODELS_PYTHON or SDM1_MODELS_VENV. SDM1_LOG_DIR is the
        // telemetry root.
        passthrough: &[
            "TABPFN_API_KEY",
            "SDM1_TABPFN_BASE",
            "SDM1_TABPFN_TIMEOUT",
            "SDM1_MODELS_PYTHON",
            "SDM1_MODELS_VENV",
            "SDM1_TIMEOUT",
            "SDM1_LOG_DIR",
        ],
        health_tools: &["sdm1_health"],
    },
    Lane {
        name: "sem1",
        role: "embeddings",
        python_path_env: "SEM1_PYTHONPATH",
        home_env: "FLEET_LANE_SEM1_HOME",
        // `st_worker.py` runs sentence-transformers inside a venv it
        // resolves from SEM1_ST_VENV (the review's named entry) — the only
        // provider that needs anything outside the core. SEM1_VECTORS_DIR
        // is the vector-store root and SEM1_LOG_DIR the telemetry root;
        // the llama-server endpoint is a compile-time default plus a tool
        // argument, so it needs no ambient name.
        passthrough: &["SEM1_ST_VENV", "SEM1_VECTORS_DIR", "SEM1_LOG_DIR"],
        health_tools: &["sem1_probe"],
    },
    Lane {
        name: "gen1",
        role: "media",
        python_path_env: "GEN1_PYTHONPATH",
        home_env: "FLEET_LANE_GEN1_HOME",
        // `keys.py`'s KEY_ENV: qwen and wan read DASHSCOPE_API_KEY, stepfun
        // reads STEPFUN_API_KEY, kokoro (the offline floor) needs no key.
        // The kokoro runner resolves its model, voices and interpreter from
        // GEN1_KOKORO_MODEL / GEN1_KOKORO_VOICES / GEN1_KOKORO_VOICE /
        // GEN1_KOKORO_PYTHON — weights on this machine, not discoverable
        // defaults — and GEN1_TELEMETRY_FILE is its telemetry sink.
        passthrough: &[
            "DASHSCOPE_API_KEY",
            "STEPFUN_API_KEY",
            "QWEN_BASE_URL",
            "QWEN_TTS_WS_URL",
            "QWEN_TTS_MODEL",
            "QWEN_TTS_VOICE",
            "STEPFUN_BASE_URL",
            "STEPFUN_TTS_MODEL",
            "STEPFUN_TTS_VOICE",
            "STEPFUN_ASR_MODEL",
            "DASHSCOPE_BASE_URL",
            "IMAGE_MODEL",
            "GEN1_KOKORO_MODEL",
            "GEN1_KOKORO_VOICES",
            "GEN1_KOKORO_VOICE",
            "GEN1_KOKORO_PYTHON",
            "GEN1_TELEMETRY_FILE",
        ],
        health_tools: &[],
    },
    Lane {
        name: "browser",
        role: "render",
        python_path_env: "BROWSER_PYTHONPATH",
        home_env: "FLEET_LANE_BROWSER_HOME",
        // The one locator this lane cannot discover: Chrome's binary. The
        // usual suspects are probed when it is unset, and the refusal
        // names what was looked for — but a machine that keeps Chrome
        // somewhere unusual says so here rather than relying on the
        // probe. A lane child never inherits the caller's environment
        // (KI-031), so this name is the whole of what crosses.
        passthrough: &["BROWSER_CHROME"],
        health_tools: &[],
    },
    Lane {
        name: "blend1",
        role: "grounded",
        python_path_env: "BLEND1_PYTHONPATH",
        home_env: "FLEET_LANE_BLEND1_HOME",
        // The blended lane's own keys and locators — the union of its
        // legs': Liquid's LIQUID_API_KEY (one vendor, one key: the d1
        // router, the LFM2.5-24B generative lane, and the sys1-score
        // leg), Fastino's FASTINO_API_KEY (the GLiDE validation gate),
        // the critic chain's TOKEN_PLAN_API_KEY and OLLAMA_API_KEY, the
        // optional web leg's TINYFISH_API_KEY, the optional voice leg's
        // DASHSCOPE_API_KEY / STEPFUN_API_KEY, and the tabular leg's
        // TABPFN_API_KEY. The escalation leg (upstream ab4ac55) reads
        // BLEND1_ESCALATION_MODEL (the larger model id — arms the leg),
        // BLEND1_ESCALATION_BASE (its OpenAI-compatible base) and
        // BLEND1_ESCALATION_KEY_ENV (naming the key variable, default
        // BLEND1_ESCALATION_API_KEY — so that name rides too). The
        // critic chain's per-critic overrides are
        // BLEND1_CRITIC_<N>_BASE / _MODEL / _KEY_ENV (N = 1..2, in
        // chain order) — the core reads them directly, so without them
        // an owner's override of a critic is unreachable under fleet's
        // launch. BLEND1_LOG_DIR is the telemetry root, the locator
        // rule every sibling lane's log dir already carries. The
        // corpus tools read BLEND1_VECTORS_DIR (the store-root override,
        // the locator rule) and BLEND1_PROJECT (the project-resolution
        // input: param > env > cwd repo name > legacy). The sibling
        // cores resolve through the adapter's own BLEND1_<SVC>_PYTHONPATH
        // path wiring — locators beside the lane in services/, not
        // ambient machine state — so those names are not passthrough
        // entries.
        passthrough: &[
            "LIQUID_API_KEY",
            "FASTINO_API_KEY",
            "TINYFISH_API_KEY",
            "DASHSCOPE_API_KEY",
            "STEPFUN_API_KEY",
            "TOKEN_PLAN_API_KEY",
            "OLLAMA_API_KEY",
            "TABPFN_API_KEY",
            "BLEND1_ESCALATION_MODEL",
            "BLEND1_ESCALATION_BASE",
            "BLEND1_ESCALATION_KEY_ENV",
            "BLEND1_ESCALATION_API_KEY",
            "BLEND1_LOG_DIR",
            "BLEND1_CRITIC_1_BASE",
            "BLEND1_CRITIC_1_MODEL",
            "BLEND1_CRITIC_1_KEY_ENV",
            "BLEND1_CRITIC_2_BASE",
            "BLEND1_CRITIC_2_MODEL",
            "BLEND1_CRITIC_2_KEY_ENV",
            "BLEND1_VECTORS_DIR",
            "BLEND1_PROJECT",
        ],
        // blend1's own report, and the two sub-lane health tools the
        // blend's doctor already carries beside the five sub-lanes.
        health_tools: &["doctor"],
    },
];

/// The fleet-owned lane configuration file. The GUI writes it through
/// `settings.rs`; this module only reads it at launch.
pub const LANES_FILE: &str = "lanes.json";

/// The path the lane configuration lives at.
pub fn lanes_path(home: &Path) -> PathBuf {
    home.join(LANES_FILE)
}

/// The fleet home the configuration lives under, resolved the way every
/// other surface resolves it.
pub fn fleet_home_path() -> Option<PathBuf> {
    let env = std::env::var("FLEET_HOME").ok();
    let user = std::env::var("HOME").ok();
    crate::home::fleet_home(env.as_deref(), user.as_deref()).ok()
}

/// One lane's configured home, environment, and per-tool defaults, as
/// `lanes.json` carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaneConfig {
    pub home: Option<String>,
    pub env: std::collections::BTreeMap<String, String>,
    /// Tool name → argument name → default value.
    pub defaults: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
}

/// Read `<FLEET_HOME>/lanes.json` as a value. A missing file is an empty
/// object; a malformed one is a named error, because a launch that
/// silently ignored a broken config would hide the reason a lane is
/// serving the wrong model.
pub fn read_lanes(home: &Path) -> Result<serde_json::Value, String> {
    let path = home.join(LANES_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(serde_json::json!({}));
        }
        Err(error) => return Err(format!("{} is unreadable: {error}", path.display())),
    };
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} is not a JSON object", path.display()));
    }
    Ok(value)
}

/// One lane's configuration, when the file names it.
pub fn read_lane(home: &Path, name: &str) -> Result<Option<LaneConfig>, String> {
    let document = read_lanes(home)?;
    let Some(lane) = document.get("lanes").and_then(|lanes| lanes.get(name)) else {
        return Ok(None);
    };
    let home = lane
        .get("home")
        .and_then(serde_json::Value::as_str)
        .map(|home| home.trim().to_string())
        .filter(|home| !home.is_empty());
    let mut env = std::collections::BTreeMap::new();
    if let Some(map) = lane.get("env").and_then(serde_json::Value::as_object) {
        for (name, value) in map {
            if let Some(value) = value.as_str() {
                env.insert(name.clone(), value.to_string());
            }
        }
    }
    let mut defaults = std::collections::BTreeMap::new();
    if let Some(map) = lane.get("defaults").and_then(serde_json::Value::as_object) {
        for (tool, args) in map {
            let Some(args) = args.as_object() else {
                continue;
            };
            let mut values = std::collections::BTreeMap::new();
            for (argument, value) in args {
                if let Some(value) = value.as_str() {
                    values.insert(argument.clone(), value.to_string());
                } else if let Some(number) = value.as_i64() {
                    values.insert(argument.clone(), number.to_string());
                }
            }
            defaults.insert(tool.clone(), values);
        }
    }
    Ok(Some(LaneConfig {
        home,
        env,
        defaults,
    }))
}

/// Whether a name may cross into this lane's child. The rule is the
/// lane's own passthrough list, plus sys1's `SYS1_` namespace — the same
/// allowlist `mcp::child_env` applies, so a configured value cannot
/// widen what the child would accept.
pub fn lane_allows(lane: &Lane, name: &str) -> bool {
    if lane.name == "sys1" && name.starts_with("SYS1_") && name.len() > "SYS1_".len() {
        return true;
    }
    lane.passthrough.iter().any(|allowed| {
        if let Some(prefix) = allowed.strip_suffix('_') {
            name.starts_with(prefix) && name.len() > prefix.len()
        } else {
            *allowed == name
        }
    })
}

/// One lane's launch spec with the home's configuration applied:
/// `lanes.json` first, the dotenv/vault over it, and the process
/// environment winning outright. A name the lane does not declare never
/// crosses, and the spec's own entries (`<SVC>_PYTHONPATH`) still win
/// last.
pub fn configured_spec(lane: &Lane, home: &Path) -> Result<ServerSpec, String> {
    let Some(fleet_home) = fleet_home_path() else {
        return Ok(launch_spec(lane, home));
    };
    configured_spec_at(lane, home, &fleet_home)
}

/// The same injection over an explicit fleet home, so the precedence is
/// testable without mutating the process environment.
pub fn configured_spec_at(
    lane: &Lane,
    home: &Path,
    fleet_home: &Path,
) -> Result<ServerSpec, String> {
    let mut spec = launch_spec(lane, home);
    let config = read_lane(fleet_home, lane.name)?.unwrap_or_default();
    let passphrase = std::env::var("FLEET_VAULT_PASSPHRASE").ok();
    let dotenv = crate::home::read_dotenv_with_vault(fleet_home, passphrase.as_deref());
    let mut values: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    // Low to high: lane config, then the dotenv/vault, then skip the
    // process environment because it wins over everything below.
    for (name, value) in &config.env {
        if lane_allows(lane, name) {
            values.insert(name.clone(), value.clone());
        }
    }
    for (name, value) in &dotenv {
        if lane_allows(lane, name) {
            values.insert(name.clone(), value.clone());
        }
    }
    values.retain(|name, _| std::env::var(name).is_err());
    for (name, value) in values {
        if spec.env.iter().any(|(declared, _)| declared == &name) {
            continue;
        }
        spec.env.push((name, value));
    }
    Ok(spec)
}

/// One tool's configured defaults for a lane. The caller merges them
/// under the model's own arguments, so an explicit value always wins.
pub fn defaults_for(
    home: &Path,
    lane: &str,
    tool: &str,
) -> std::collections::BTreeMap<String, String> {
    read_lane(home, lane)
        .ok()
        .flatten()
        .and_then(|config| config.defaults.get(tool).cloned())
        .unwrap_or_default()
}

/// Merge a lane's configured defaults under the arguments the model
/// supplied. A key the model named always wins; a default only fills an
/// absent one, so the surface never silently overrides an explicit call.
pub fn with_defaults(
    arguments: &serde_json::Value,
    defaults: &std::collections::BTreeMap<String, String>,
) -> serde_json::Value {
    let serde_json::Value::Object(object) = arguments else {
        return arguments.clone();
    };
    if defaults.is_empty() {
        return arguments.clone();
    }
    let mut merged = object.clone();
    for (name, value) in defaults {
        merged
            .entry(name.clone())
            .or_insert_with(|| serde_json::Value::String(value.clone()));
    }
    serde_json::Value::Object(merged)
}

/// The arguments one lane call should carry: the configured defaults for
/// that tool with the model's own values laid over them.
pub fn lane_arguments(
    home: &Path,
    lane: &str,
    tool: &str,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let defaults = defaults_for(home, lane, tool);
    with_defaults(arguments, &defaults)
}

/// One pickable value for a lane form: the value, how it reads, the
/// provider it belongs to, and which field it feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneChoice {
    pub value: String,
    pub label: String,
    pub provider: String,
    pub kind: &'static str,
}

fn choice(value: &str, label: &str, provider: &str, kind: &'static str) -> LaneChoice {
    LaneChoice {
        value: value.to_string(),
        label: label.to_string(),
        provider: provider.to_string(),
        kind,
    }
}

/// The values each lane's own defaults publish. This is fleet's
/// declaration of the lane surfaces, not a live read: a value the lane
/// adds later is still accepted as a custom entry.
pub fn choices(lane: &str) -> Vec<LaneChoice> {
    match lane {
        "sys1" => {
            let mut out = vec![
                choice("decide", "decide", "decide", "provider"),
                choice(
                    "fastino/GLiNER2.5-Decide",
                    "GLiNER2.5-Decide",
                    "decide",
                    "model",
                ),
                choice(
                    "https://api.fastino.ai/v1/chat/completions",
                    "Fastino chat v1",
                    "decide",
                    "endpoint",
                ),
                choice("FASTINO_API_KEY", "FASTINO_API_KEY", "decide", "key_env"),
                choice("jev", "jev", "jev", "provider"),
                choice("jev-1.13.0", "jev-1.13.0", "jev", "model"),
                choice(
                    "https://api.typesafe.ai/v1/systemone",
                    "TypeSafe systemone",
                    "jev",
                    "endpoint",
                ),
                choice("TYPESAFE_API_KEY", "TYPESAFE_API_KEY", "jev", "key_env"),
                choice("drex", "drex", "drex", "provider"),
                choice("drex-v1.5", "drex-v1.5", "drex", "model"),
                choice(
                    "https://drex.nace.ai/v1/systemone",
                    "Nace systemone",
                    "drex",
                    "endpoint",
                ),
                choice("DREX_API_KEY", "DREX_API_KEY", "drex", "key_env"),
                choice("glide", "glide", "glide", "provider"),
                choice("fastino/glide", "GLiDE", "glide", "model"),
                choice(
                    "https://api.fastino.ai/v1/systemone",
                    "Fastino systemone",
                    "glide",
                    "endpoint",
                ),
                choice("FASTINO_API_KEY", "FASTINO_API_KEY", "glide", "key_env"),
                choice("julia", "julia", "julia", "provider"),
                choice("julia-1", "julia-1", "julia", "model"),
                choice(
                    "https://api.supersoniclabs.ai/v1/systemone",
                    "Supersonic systemone",
                    "julia",
                    "endpoint",
                ),
                choice(
                    "SUPERSONIC_API_KEY",
                    "SUPERSONIC_API_KEY",
                    "julia",
                    "key_env",
                ),
                choice("liquid", "liquid", "liquid", "provider"),
                choice("d1", "d1", "liquid", "model"),
                choice(
                    "https://api.liquid.ai/decisions/v1/systemone",
                    "Liquid decisions",
                    "liquid",
                    "endpoint",
                ),
                choice("LIQUID_API_KEY", "LIQUID_API_KEY", "liquid", "key_env"),
                choice("scaledown", "scaledown", "scaledown", "provider"),
                choice("classify-1", "classify-1", "scaledown", "model"),
                choice(
                    "https://api.scaledown.xyz/v1/scaledown",
                    "ScaleDown",
                    "scaledown",
                    "endpoint",
                ),
                choice(
                    "SCALEDOWN_API_KEY",
                    "SCALEDOWN_API_KEY",
                    "scaledown",
                    "key_env",
                ),
                choice("local", "local", "local", "provider"),
                choice(
                    "fastino/GLiNER2.5-Decide",
                    "GLiNER2.5-Decide local",
                    "local",
                    "model",
                ),
                choice("/private/tmp/gliner-decide", "local venv", "local", "path"),
                choice("clm", "clm", "clm", "provider"),
                choice("Contrastive-LM/CLM-v0.1-8B", "CLM-8B", "clm", "model"),
                choice(
                    "http://127.0.0.1:8451/v1/systemone",
                    "CLM loopback",
                    "clm",
                    "endpoint",
                ),
                choice("CLM_API_KEY", "CLM_API_KEY", "clm", "key_env"),
                choice("openjev", "openjev", "openjev", "provider"),
                choice("openjev-k5", "openjev-k5", "openjev", "model"),
                choice(
                    "http://127.0.0.1:8452/v1/systemone",
                    "openjev loopback",
                    "openjev",
                    "endpoint",
                ),
                choice("OPENJEV_API_KEY", "OPENJEV_API_KEY", "openjev", "key_env"),
                choice("kev", "kev", "kev", "provider"),
                choice("kev-4b", "kev-4b", "kev", "model"),
                choice(
                    "http://127.0.0.1:8453/v1/chat/completions",
                    "kev loopback",
                    "kev",
                    "endpoint",
                ),
                choice("KEV_API_KEY", "KEV_API_KEY", "kev", "key_env"),
                choice("tev1", "tev1", "tev1", "provider"),
                choice("TogetherTevalabs/tev1", "tev1", "tev1", "model"),
                choice(
                    "https://api.together.xyz/v1/chat/completions",
                    "Together chat",
                    "tev1",
                    "endpoint",
                ),
                choice("TOGETHER_API_KEY", "TOGETHER_API_KEY", "tev1", "key_env"),
                choice("laya", "laya", "laya", "provider"),
                choice("convai/laya", "laya", "laya", "model"),
                choice(
                    "http://127.0.0.1:8454/v1/chat/completions",
                    "laya loopback",
                    "laya",
                    "endpoint",
                ),
                choice("LAYA_API_KEY", "LAYA_API_KEY", "laya", "key_env"),
            ];
            for (name, label) in [
                ("decide", "decide"),
                ("jev", "jev"),
                ("local", "local"),
                ("modernbert", "modernbert"),
                ("both", "both"),
                ("all", "all"),
            ] {
                out.push(choice(name, label, "classify", "classify"));
            }
            out
        }
        "sem1" => vec![
            choice("llama-server", "llama-server", "llama-server", "provider"),
            choice(
                "embeddinggemma-2-BF16",
                "EmbeddingGemma-2 BF16",
                "llama-server",
                "model",
            ),
            choice(
                "http://127.0.0.1:8901",
                "llama-server loopback",
                "llama-server",
                "endpoint",
            ),
            choice("st-worker", "st-worker", "st-worker", "provider"),
            choice(
                "google/embeddinggemma-2",
                "EmbeddingGemma-2",
                "st-worker",
                "model",
            ),
        ],
        "sdm1" => vec![
            choice(
                "tabpfn-hosted",
                "tabpfn-hosted",
                "tabpfn-hosted",
                "provider",
            ),
            choice(
                "https://api.priorlabs.ai",
                "Prior Labs",
                "tabpfn-hosted",
                "endpoint",
            ),
            choice("nvidia-sdm", "nvidia-sdm", "nvidia-sdm", "provider"),
        ],
        "gen1" => vec![
            choice("qwen", "qwen", "qwen", "provider"),
            choice("qwen-audio-3.0-tts-plus", "qwen TTS plus", "qwen", "model"),
            choice("longanhuan_v3.6", "longanhuan_v3.6", "qwen", "voice"),
            choice(
                "https://token-plan.ap-southeast-1.maas.aliyuncs.com",
                "DashScope token plan",
                "qwen",
                "endpoint",
            ),
            choice("stepfun", "stepfun", "stepfun", "provider"),
            choice("stepaudio-2.5-tts", "stepaudio-2.5-tts", "stepfun", "model"),
            choice("stepaudio-2.5-asr", "stepaudio-2.5-asr", "stepfun", "model"),
            choice("lively-girl", "lively-girl", "stepfun", "voice"),
            choice("https://api.stepfun.ai", "StepFun", "stepfun", "endpoint"),
            choice("wan", "wan", "wan", "provider"),
            choice("wan2.7-image-pro", "wan2.7-image-pro", "wan", "model"),
            choice("kokoro", "kokoro", "kokoro", "provider"),
            choice("af_heart", "af_heart", "kokoro", "voice"),
        ],
        _ => Vec::new(),
    }
}

/// The machine-local paths the pickers offer beside the resolved lane
/// homes.
pub fn known_paths() -> Vec<(String, String)> {
    let Some(user) = std::env::var("HOME").ok() else {
        return Vec::new();
    };
    let user = PathBuf::from(user);
    [
        (
            "/private/tmp/gliner-decide".to_string(),
            "sys1 local GLiNER venv".to_string(),
        ),
        (
            user.join(".cache/hyperframes/tts/models/kokoro-v1.0.onnx")
                .display()
                .to_string(),
            "kokoro model".to_string(),
        ),
        (
            user.join(".cache/hyperframes/tts/voices/voices-v1.0.bin")
                .display()
                .to_string(),
            "kokoro voices".to_string(),
        ),
        (
            user.join(".cache/gen1/tts").display().to_string(),
            "gen1 TTS cache".to_string(),
        ),
    ]
    .into_iter()
    .collect()
}

/// Where a resolved lane home came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeSource {
    /// The per-lane environment override named it.
    Env(String),
    /// The fleet home's `lanes.json` named it.
    Config,
    /// The bundled `services/` tree answered.
    Bundled,
}

impl std::fmt::Display for HomeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HomeSource::Env(name) => write!(f, "{name}"),
            HomeSource::Config => write!(f, "lanes.json"),
            HomeSource::Bundled => write!(f, "bundled"),
        }
    }
}

/// A lane's resolved home, or the reason it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneHome {
    Resolved { home: PathBuf, source: HomeSource },
    Unavailable { reason: String },
}

/// What one lane's probe found. The headless command and the TUI modal
/// render this same value — one source of truth for lane health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneHealth {
    pub name: String,
    pub role: String,
    /// The resolved copy, when there was one.
    pub home: Option<PathBuf>,
    /// How the home resolved, when it did.
    pub source: Option<HomeSource>,
    pub state: LaneState,
    /// The server's self-reported version, when initialize answered.
    pub server_version: Option<String>,
    /// The tool names `tools/list` returned, when it answered.
    pub tools: Vec<String>,
    /// The per-lane availability the lane's own `doctor` tool reported,
    /// for a lane that exposes one (blend1): each of its lanes — route,
    /// semantic, tabular, generate, validate — available or not, with
    /// its named reason, and the web leg's own entry beside them. Empty
    /// for the other four lanes, which expose no `doctor` tool.
    pub doctor: Vec<LaneAvailability>,
    /// The launch command when the lane is ok; the failure reason when it
    /// is not.
    pub detail: String,
    /// How long the whole probe took.
    pub elapsed_ms: u128,
}

/// One lane's availability as a lane's own `doctor` tool reported it:
/// the sub-lane's name, whether it can answer, its named reason, and
/// the provider that backs it. The reason is named either way — an
/// available lane says what backs it, an unavailable one says what is
/// missing — so the card never renders a bare dot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaneAvailability {
    pub lane: String,
    pub available: bool,
    pub reason: String,
    pub provider: String,
}

/// Where one lane stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneState {
    /// Initialized and listed its tools.
    Ok,
    /// Could not launch at all: no copy, no `python3`.
    Unavailable,
    /// Launched, but the handshake or the listing failed.
    Error,
}

impl LaneState {
    /// The dot each state renders with, in the TUI's palette.
    pub fn dot(self) -> &'static str {
        match self {
            LaneState::Ok => "✓",
            LaneState::Unavailable => "○",
            LaneState::Error => "✕",
        }
    }

    /// The word each state renders with.
    pub fn label(self) -> &'static str {
        match self {
            LaneState::Ok => "ok",
            LaneState::Unavailable => "unavailable",
            LaneState::Error => "error",
        }
    }
}

/// Why the services root could not be resolved. A typed value, so the
/// caller names the reason instead of collapsing two different failures
/// into one "not found".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootError {
    /// The `FLEET_SERVICES_ROOT` override was set to a relative path,
    /// which would resolve against the caller's working directory. The
    /// offending value rides along for the message.
    Relative(String),
    /// No override, and neither the executable walk nor the compile-time
    /// manifest directory found a services tree.
    NotFound,
}

impl std::fmt::Display for RootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RootError::Relative(value) => write!(
                f,
                "{SERVICES_ROOT_ENV}={value} is a relative path; an absolute one is required, \
                 because a relative one resolves against the caller's working directory"
            ),
            RootError::NotFound => {
                write!(f, "no services root found (set {SERVICES_ROOT_ENV})")
            }
        }
    }
}

impl std::error::Error for RootError {}

/// The reason a configured home is unusable, when it is. A relative path
/// passes every layout check — the `service/mcp_server.py` and `src/`
/// are right there — and still launches from a different place depending
/// on where the caller stood, so the check is on absoluteness, not on
/// layout.
fn relative_home_reason(env: &str, home: &Path) -> Option<String> {
    (!home.is_absolute()).then(|| {
        format!(
            "{env}={} is a relative path; an absolute one is required, \
             because a relative one resolves against the caller's working directory",
            home.display()
        )
    })
}

/// Resolve one lane's home: the environment override first, then the
/// bundled copy, then a named reason.
pub fn resolve_home(lane: &Lane) -> LaneHome {
    if let Some(home) = std::env::var_os(lane.home_env) {
        let home = PathBuf::from(home);
        if let Some(reason) = relative_home_reason(lane.home_env, &home) {
            return LaneHome::Unavailable { reason };
        }
        return if looks_like_service_home(&home) {
            LaneHome::Resolved {
                home,
                source: HomeSource::Env(lane.home_env.to_string()),
            }
        } else {
            LaneHome::Unavailable {
                reason: format!(
                    "{}={} has no service/mcp_server.py and src/",
                    lane.home_env,
                    home.display()
                ),
            }
        };
    }
    // The fleet home's own configuration, after the process override and
    // before the bundled copy. A configured home that cannot launch is a
    // named refusal rather than a silent fall-through: the owner asked
    // for that copy.
    if let Some(fleet_home) = fleet_home_path() {
        match read_lane(&fleet_home, lane.name) {
            Ok(Some(config)) => {
                if let Some(configured) = config.home {
                    let configured = PathBuf::from(configured);
                    if !configured.is_absolute() {
                        return LaneHome::Unavailable {
                            reason: format!(
                                "lanes.json names a relative home for `{}`: {}; an absolute path \
                                 is required",
                                lane.name,
                                configured.display()
                            ),
                        };
                    }
                    return if looks_like_service_home(&configured) {
                        LaneHome::Resolved {
                            home: configured,
                            source: HomeSource::Config,
                        }
                    } else {
                        LaneHome::Unavailable {
                            reason: format!(
                                "lanes.json's home for `{}` ({}) has no service/mcp_server.py and src/",
                                lane.name,
                                configured.display()
                            ),
                        }
                    };
                }
            }
            Ok(None) => {}
            Err(reason) => {
                return LaneHome::Unavailable {
                    reason: format!("lanes.json: {reason}"),
                }
            }
        }
    }
    let root = match services_root() {
        Ok(root) => root,
        Err(RootError::Relative(value)) => {
            return LaneHome::Unavailable {
                reason: RootError::Relative(value).to_string(),
            }
        }
        Err(RootError::NotFound) => {
            return LaneHome::Unavailable {
                reason: format!(
                    "no services root found (set {} or {})",
                    SERVICES_ROOT_ENV, lane.home_env
                ),
            }
        }
    };
    let home = root.join(lane.name);
    if looks_like_service_home(&home) {
        LaneHome::Resolved {
            home,
            source: HomeSource::Bundled,
        }
    } else {
        LaneHome::Unavailable {
            reason: format!("the bundled copy at {} is incomplete", home.display()),
        }
    }
}

/// The layout a service home must show to launch.
fn looks_like_service_home(home: &Path) -> bool {
    home.join("service").join("mcp_server.py").is_file() && home.join("src").is_dir()
}

/// The bundled services root: the environment override, else a walk up
/// from the running executable, else the compile-time manifest directory
/// (this is a solo repo whose builds happen beside its own checkout).
/// A set-but-invalid override wins over the walk, so a mistyped path
/// fails with a named reason instead of silently resolving elsewhere.
pub fn services_root() -> Result<PathBuf, RootError> {
    if let Some(root) = std::env::var_os(SERVICES_ROOT_ENV) {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err(RootError::Relative(root.to_string_lossy().into_owned()));
        }
        return Ok(root);
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..ROOT_WALK_DEPTH {
            let Some(candidate) = dir else { break };
            if candidate.join("services").is_dir() && candidate.join("Cargo.toml").is_file() {
                return Ok(candidate.join("services"));
            }
            dir = candidate.parent().map(Path::to_path_buf);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("services");
    if manifest.is_dir() {
        Ok(manifest)
    } else {
        Err(RootError::NotFound)
    }
}

/// The launch spec for one lane, or the reason it cannot launch.
pub fn launch_spec(lane: &Lane, home: &Path) -> ServerSpec {
    let mut env = vec![(
        lane.python_path_env.to_string(),
        home.join("src").to_string_lossy().into_owned(),
    )];
    // The blended lane composes its siblings as path dependencies: the
    // vendored cores beside it under the services root, wired through
    // the house `<SVC>_PYTHONPATH` mechanism its adapter reads
    // (`BLEND1_SYS1_PYTHONPATH` / `BLEND1_SEM1_PYTHONPATH` /
    // `BLEND1_SDM1_PYTHONPATH` / `BLEND1_GEN1_PYTHONPATH` beside
    // `BLEND1_PYTHONPATH`) — the same mechanism sem1's adapter already
    // reads, extended to the blend's siblings. Every other lane's spec
    // is exactly what it was: the one path var and nothing more.
    env.extend(sibling_paths(lane, home));
    ServerSpec {
        program: "python3".to_string(),
        args: vec![
            "-u".to_string(),
            home.join("service")
                .join("mcp_server.py")
                .to_string_lossy()
                .into_owned(),
        ],
        env,
        // The lane's declared ambient names, and nothing else: the child
        // starts from `mcp::BASE_ENV` plus this plus the entry above.
        passthrough: lane
            .passthrough
            .iter()
            .map(|name| name.to_string())
            .collect(),
        cwd: Some(home.to_path_buf()),
    }
}

/// The sibling cores the blended lane path-wires beside its own —
/// `BLEND1_<SVC>_PYTHONPATH` → `<services root>/<svc>/src` — resolved
/// from the lane's own home, so a `FLEET_LANE_BLEND1_HOME` override
/// outside the bundled tree wires the siblings beside that copy. An
/// empty list for every other lane: they compose no siblings.
fn sibling_paths(lane: &Lane, home: &Path) -> Vec<(String, String)> {
    if lane.name != "blend1" {
        return Vec::new();
    }
    let Some(root) = home.parent() else {
        return Vec::new();
    };
    BLEND1_SIBLINGS
        .iter()
        .map(|sibling| {
            (
                format!("BLEND1_{}_PYTHONPATH", sibling.to_uppercase()),
                root.join(sibling)
                    .join("src")
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .collect()
}

/// The launch command as one readable line, for the health detail.
pub fn launch_command(lane: &Lane, home: &Path) -> String {
    format!(
        "{}={}/src python3 -u {}/service/mcp_server.py",
        lane.python_path_env,
        home.display(),
        home.display()
    )
}

/// The launch spec for the lane named `name`, or the reason it cannot
/// launch: a name the registry does not carry, or a home that did not
/// resolve. The agent's tool loop resolves a lane through this the first
/// time one of its tools is called, so a lane that cannot launch fails
/// that call with its named reason instead of silently never answering.
pub fn spec_for(name: &str) -> Result<ServerSpec, String> {
    let Some(lane) = LANES.iter().find(|lane| lane.name == name) else {
        return Err(format!("no lane named `{name}`"));
    };
    match resolve_home(lane) {
        LaneHome::Resolved { home, .. } => configured_spec(lane, &home),
        LaneHome::Unavailable { reason } => Err(reason),
    }
}

/// Probe every lane in order, tearing each child down before the next.
/// Sequential on purpose: five small children under a loaded machine is
/// where parallel spawns flake (KI-027), and the order stays the
/// registry's.
pub async fn health_all() -> Vec<LaneHealth> {
    let mut health = Vec::with_capacity(LANES.len());
    for lane in LANES {
        health.push(probe_lane(lane).await);
    }
    health
}

/// Every lane reported unavailable with `reason` — the shape the TUI's
/// prober thread sends when it cannot even start its runtime.
pub fn unavailable_all(reason: &str) -> Vec<LaneHealth> {
    LANES
        .iter()
        .map(|lane| LaneHealth {
            name: lane.name.to_string(),
            role: lane.role.to_string(),
            home: None,
            source: None,
            state: LaneState::Unavailable,
            server_version: None,
            tools: Vec::new(),
            doctor: Vec::new(),
            detail: reason.to_string(),
            elapsed_ms: 0,
        })
        .collect()
}

/// Probe one lane: spawn, initialize, list, tear down — in that order,
/// with the child never outliving the probe.
pub async fn probe_lane(lane: &Lane) -> LaneHealth {
    let started = Instant::now();
    let home = match resolve_home(lane) {
        LaneHome::Resolved { home, source } => Some((home, source)),
        LaneHome::Unavailable { reason } => {
            return LaneHealth {
                name: lane.name.to_string(),
                role: lane.role.to_string(),
                home: None,
                source: None,
                state: LaneState::Unavailable,
                server_version: None,
                tools: Vec::new(),
                doctor: Vec::new(),
                detail: reason,
                elapsed_ms: started.elapsed().as_millis(),
            };
        }
    };
    let (home, source) = home.expect("a resolved home");
    let spec = match configured_spec(lane, &home) {
        Ok(spec) => spec,
        Err(reason) => {
            return LaneHealth {
                name: lane.name.to_string(),
                role: lane.role.to_string(),
                home: Some(home),
                source: Some(source),
                state: LaneState::Unavailable,
                server_version: None,
                tools: Vec::new(),
                doctor: Vec::new(),
                detail: reason,
                elapsed_ms: started.elapsed().as_millis(),
            };
        }
    };
    let launch = launch_command(lane, &home);
    let mut server = match McpServer::spawn(&spec) {
        Ok(server) => server,
        Err(error) => {
            return LaneHealth {
                name: lane.name.to_string(),
                role: lane.role.to_string(),
                home: Some(home),
                source: Some(source),
                state: LaneState::Unavailable,
                server_version: None,
                tools: Vec::new(),
                doctor: Vec::new(),
                detail: error.to_string(),
                elapsed_ms: started.elapsed().as_millis(),
            };
        }
    };
    // Every exit path below tears the child down first, so a failed probe
    // never leaks a Python process.
    let outcome = probe_handshake(&mut server).await;
    // A lane that exposes a `doctor` tool gets its report called before
    // teardown: the per-lane availability its card renders. A call that
    // fails — or a lane that exposes no such tool — is an empty report,
    // never a failed probe: the handshake already answered, and a
    // doctor that cannot answer is not a lane that broke.
    // KI-030: every health tool the lane exposes gets called beside the
    // handshake, and each report's per-lane availability rides the
    // health's `doctor` list. A call that fails — or a lane that
    // exposes no such tool — is an empty report, never a failed probe:
    // the handshake already answered, and a health tool that cannot
    // answer is not a lane that broke.
    let mut doctor: Vec<LaneAvailability> = Vec::new();
    if let Ok((_, tools)) = &outcome {
        for tool_name in lane.health_tools {
            if !tools.iter().any(|tool| tool.name == *tool_name) {
                continue;
            }
            let Ok(call) = server.call_tool(tool_name, json!({})).await else {
                continue;
            };
            let text = call.text();
            if *tool_name == DOCTOR_TOOL {
                let report = parse_doctor(&text);
                doctor.extend(report.lanes.into_iter().chain(report.web));
            } else {
                doctor.extend(parse_health_map(&text));
            }
        }
    }
    let _ = server.close().await;
    match outcome {
        Ok((initialize, tools)) => LaneHealth {
            name: lane.name.to_string(),
            role: lane.role.to_string(),
            home: Some(home),
            source: Some(source),
            state: LaneState::Ok,
            server_version: Some(initialize.server_version),
            tools: tools.iter().map(|tool| tool.name.clone()).collect(),
            doctor,
            detail: launch,
            elapsed_ms: started.elapsed().as_millis(),
        },
        Err(error) => LaneHealth {
            name: lane.name.to_string(),
            role: lane.role.to_string(),
            home: Some(home),
            source: Some(source),
            state: LaneState::Error,
            server_version: None,
            tools: Vec::new(),
            doctor: Vec::new(),
            detail: error.to_string(),
            elapsed_ms: started.elapsed().as_millis(),
        },
    }
}

/// The tool name a lane's own per-lane availability report answers on.
/// A lane that lists it in `tools/list` gets it called by the probe.
const DOCTOR_TOOL: &str = "doctor";

/// A provider-map health report — sdm1's `sdm1_health` and sem1's
/// `sem1_probe` shape: `{provider_id: {name, available, reason, …}}`.
/// KI-030: these are the reports that say whether a lane's PROVIDERS
/// answer, which the MCP-surface handshake cannot. Each entry rides the
/// health's `doctor` list, so the CLI's card and the TUI's lanes pane
/// render them beside blend1's own sub-lane report with no change.
fn parse_health_map(text: &str) -> Vec<LaneAvailability> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    let mut out: Vec<LaneAvailability> = map
        .iter()
        .filter_map(|(provider, entry)| {
            let available = entry.get("available").and_then(|a| a.as_bool())?;
            Some(LaneAvailability {
                lane: entry
                    .get("name")
                    .and_then(|name| name.as_str())
                    .unwrap_or(provider)
                    .to_string(),
                available,
                reason: entry
                    .get("reason")
                    .and_then(|reason| reason.as_str())
                    .unwrap_or_default()
                    .to_string(),
                provider: provider.clone(),
            })
        })
        .collect();
    out.sort_by(|left, right| left.provider.cmp(&right.provider));
    out
}

/// The sub-lanes a `doctor` report renders, in the blend's own order:
/// the five lanes an `ask()` composes its answer from. The report also
/// carries the escalation and sys1-score legs (upstream ab4ac55) beside
/// the critic chain, the voice leg, and the web leg; the card renders
/// these five, and the web leg as its own line — the web leg is not a
/// blend sub-lane, it is the web leg blend1 serves.
const DOCTOR_LANES: &[&str] = &["route", "semantic", "tabular", "generate", "validate"];

/// The web leg's key in a `doctor` report. It rides the same object
/// as the five sub-lanes but is not one of them, so the parse keeps
/// it beside them and every consumer that draws the sub-lane column
/// — the CLI's card and the TUI's lanes pane alike — filters it out
/// and renders it as its own labeled line.
pub const WEB_LEG: &str = "web";

/// What a lane's `doctor` report parses into: the five sub-lanes the
/// card renders, in the blend's own order, and the web leg's
/// availability beside them — the key the report already carries
/// that the parse used to drop as an unrendered extra.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DoctorReport {
    lanes: Vec<LaneAvailability>,
    web: Option<LaneAvailability>,
}

/// Read a `doctor` tool's answer into the per-lane availability the
/// card renders. The answer's text is a JSON object keyed by lane
/// name, each value `{available, reason, provider}` — a lane missing
/// from the object, or a value without an `available` boolean, is
/// skipped rather than guessed at. The web leg rides the same object
/// under its own key and is kept beside the five. A report that
/// cannot be read at all is empty: the card renders without it and
/// the probe stays Ok, because a doctor that cannot answer is not a
/// broken handshake.
fn parse_doctor(text: &str) -> DoctorReport {
    let Ok(report) = serde_json::from_str::<serde_json::Value>(text) else {
        return DoctorReport::default();
    };
    let Some(report) = report.as_object() else {
        return DoctorReport::default();
    };
    let lanes = DOCTOR_LANES
        .iter()
        .filter_map(|lane| parse_availability(lane, report.get(*lane)?))
        .collect();
    let web = report
        .get(WEB_LEG)
        .and_then(|entry| parse_availability(WEB_LEG, entry));
    DoctorReport { lanes, web }
}

/// One availability entry from a `doctor` report: the named lane,
/// whether it can answer, its named reason, and the provider that
/// backs it. None when the entry is absent, is not an object, or
/// carries no boolean `available` — skipped rather than guessed at.
fn parse_availability(lane: &str, entry: &serde_json::Value) -> Option<LaneAvailability> {
    let entry = entry.as_object()?;
    Some(LaneAvailability {
        lane: lane.to_string(),
        available: entry.get("available")?.as_bool()?,
        reason: entry
            .get("reason")
            .and_then(|reason| reason.as_str())
            .unwrap_or_default()
            .to_string(),
        provider: entry
            .get("provider")
            .and_then(|provider| provider.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// The handshake half of a probe, kept separate so the caller owns
/// teardown.
async fn probe_handshake(
    server: &mut McpServer,
) -> Result<(crate::mcp::InitializeResult, Vec<crate::mcp::ToolInfo>), McpError> {
    let initialize = server.initialize().await?;
    let tools = server.list_tools().await?;
    Ok((initialize, tools))
}

/// The ANSI palette, mirroring the TUI's `ui.rs` palette module so the
/// headless command and the TUI read as one surface.
mod palette {
    pub const ACCENT: &str = "\u{1b}[36m"; // cyan
    pub const OK: &str = "\u{1b}[32m"; // green
    pub const ERROR: &str = "\u{1b}[31m"; // red
    pub const DIM: &str = "\u{1b}[90m"; // dark gray
    pub const TEXT: &str = "\u{1b}[37m"; // gray
    pub const BOLD: &str = "\u{1b}[1m";
    pub const RESET: &str = "\u{1b}[0m";
}

/// The card width, in characters. A fixed width keeps the cards aligned
/// without a terminal-size dependency.
const CARD_WIDTH: usize = 72;

/// Render the health report as cards: one bordered card per lane, a status
/// dot, the role and tool count, the tool names, and the launch command —
/// never a plain-text table. A lane that exposes a `doctor` tool (blend1)
/// also renders its per-lane availability, one dot-and-reason line per
/// sub-lane, inside its card.
pub fn render(health: &[LaneHealth]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet lanes{RESET} {DIM}— the {} house lanes, probed live{RESET}\n\n",
        health.len()
    ));
    for lane in health {
        out.push_str(&render_card(lane));
    }
    let ok = health
        .iter()
        .filter(|lane| lane.state == LaneState::Ok)
        .count();
    out.push_str(&format!(
        "\n{DIM}{} lanes · {ok} ok · {} not ok{RESET}\n",
        health.len(),
        health.len() - ok
    ));
    out
}

/// One lane's card.
fn render_card(lane: &LaneHealth) -> String {
    let dot_color = match lane.state {
        LaneState::Ok => palette::OK,
        LaneState::Unavailable => palette::DIM,
        LaneState::Error => palette::ERROR,
    };
    let header_right = format!("{} {}", lane.state.dot(), lane.state.label());
    let header = format!(
        "{DIM}┌─{RESET} {BOLD}{ACCENT}{name}{RESET} {DIM}{dashes} {dot_color}{right}{RESET} {DIM}┐{RESET}",
        name = lane.name,
        dashes = dashes_for(&format!(" {} ", lane.name), &header_right),
        right = header_right,
    );
    let facts = match (&lane.server_version, lane.tools.len()) {
        (Some(version), count) => {
            format!(
                "{} · v{} · {} tool{}",
                lane.role,
                version,
                count,
                plural(count)
            )
        }
        (None, _) => lane.role.to_string(),
    };
    let mut lines = vec![format!("{DIM}│{RESET} {TEXT}{facts}{RESET}")];
    // The detail (a launch command or a failure reason) wraps hard inside
    // the card: the card keeps its width, and nothing is truncated.
    for chunk in wrap_detail(&lane.detail, CARD_WIDTH - 4) {
        lines.push(format!("{DIM}│{RESET} {DIM}{chunk}{RESET}"));
    }
    if !lane.tools.is_empty() {
        lines.push(format!(
            "{DIM}│{RESET} {TEXT}{tools}{RESET}",
            tools = lane.tools.join(" · ")
        ));
    }
    // The lane's own `doctor` report, when it exposed one: one line per
    // sub-lane — a status dot and its named reason — at the house visual
    // bar, never a plain-text table. A wrapped continuation indents under
    // the body, so a long reason reads as one line broken, not two. The
    // web leg is not one of these rows; it renders its own line below.
    for availability in lane.doctor.iter().filter(|entry| entry.lane != WEB_LEG) {
        let (dot, color) = if availability.available {
            ("\u{2713}", palette::OK)
        } else {
            ("\u{25cb}", palette::DIM)
        };
        let named = if availability.reason.is_empty() {
            availability.provider.as_str()
        } else {
            availability.reason.as_str()
        };
        let body = format!("{dot} {} — {named}", availability.lane);
        for (index, chunk) in wrap_detail(&body, CARD_WIDTH - 4).iter().enumerate() {
            let indent = if index == 0 { "" } else { "  " };
            lines.push(format!("{DIM}│{RESET} {color}{indent}{chunk}{RESET}"));
        }
    }
    // The web leg, its own line: the label leads in the accent color
    // because the web leg is not one of the five blend sub-lanes — it
    // is the web leg blend1 serves — and the card marks the difference
    // at a glance. The reason is named either way, the same as the
    // sub-lanes above: available says what backs it, unavailable says
    // what is missing.
    if let Some(web) = lane.doctor.iter().find(|entry| entry.lane == WEB_LEG) {
        let (dot, color) = if web.available {
            ("\u{2713}", palette::OK)
        } else {
            ("\u{25cb}", palette::DIM)
        };
        let named = if web.reason.is_empty() {
            web.provider.as_str()
        } else {
            web.reason.as_str()
        };
        let label = format!("{ACCENT}web leg{RESET}");
        let body = format!("{dot} — {named}");
        for (index, chunk) in wrap_detail(&body, CARD_WIDTH - 12).iter().enumerate() {
            let indent = if index == 0 { "" } else { "  " };
            lines.push(format!(
                "{DIM}│{RESET} {label} {color}{indent}{chunk}{RESET}"
            ));
        }
    }
    if let Some(source) = &lane.source {
        lines.push(format!(
            "{DIM}│{RESET} {DIM}home: {source} · {} ms{RESET}",
            lane.elapsed_ms
        ));
    }
    let bottom = format!(
        "{DIM}└{bottom}┘{RESET}",
        bottom = "─".repeat(CARD_WIDTH - 2)
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

/// The dash run that pads a card header so its right edge aligns. Counts
/// characters, which is exact for the ASCII lane names and labels used
/// here.
fn dashes_for(left: &str, right: &str) -> String {
    let used = left.chars().count() + right.chars().count() + 2;
    "─".repeat(CARD_WIDTH.saturating_sub(used + 4).max(1))
}

/// Hard-wrap `text` into `width`-character chunks. The details here are
/// paths and command lines with no natural break points, so the break is
/// a split, never a truncation.
/// KI-055: one word-aware wrapper for the card family. `skills` and
/// `cron` already broke prose at word boundaries; this hard-split
/// mid-character instead, so the same detail line read two ways on two
/// cards. The shared wrapper keeps words whole and hard-splits only the
/// word that cannot fit a line of its own — which is the one case a
/// word-aware break cannot serve.
fn wrap_detail(text: &str, width: usize) -> Vec<String> {
    crate::skills::wrap_detail(text, width)
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The env-mutating tests below serialize on this. `cargo test` runs a
    /// module's tests on threads, and a `set_var` in one must not be
    /// visible to another mid-flight; the readers take it too.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Set `name` for the duration of the guard, restoring whatever was
    /// there before — including its absence — on the way out.
    struct EnvVar {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVar {
        fn set(name: &str, value: &str) -> EnvVar {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
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

    fn lane(name: &'static str) -> Lane {
        *LANES
            .iter()
            .find(|lane| lane.name == name)
            .expect("a registry lane")
    }

    #[test]
    fn the_registry_names_the_six_lanes_in_launch_order() {
        let names: Vec<&str> = LANES.iter().map(|lane| lane.name).collect();
        assert_eq!(
            names,
            vec!["sys1", "sdm1", "sem1", "gen1", "browser", "blend1"]
        );
        // The browser lane's own names, and the one locator it declares.
        assert_eq!(lane("browser").python_path_env, "BROWSER_PYTHONPATH");
        assert_eq!(lane("browser").home_env, "FLEET_LANE_BROWSER_HOME");
        assert_eq!(lane("browser").role, "render");
        assert_eq!(lane("browser").passthrough, &["BROWSER_CHROME"]);
        // Each lane's env names follow the fork's convention.
        assert_eq!(lane("sem1").python_path_env, "SEM1_PYTHONPATH");
        assert_eq!(lane("sem1").home_env, "FLEET_LANE_SEM1_HOME");
        // blend1 is the fifth: the blended lane, its own path var and
        // per-lane home override beside it.
        assert_eq!(lane("blend1").python_path_env, "BLEND1_PYTHONPATH");
        assert_eq!(lane("blend1").home_env, "FLEET_LANE_BLEND1_HOME");
    }

    /// KI-030: every lane that exposes a provider-level health tool names
    /// it, so the probe calls it beside the handshake. The lanes with no
    /// such tool name none — an empty list is not an omission the probe
    /// has to guess at.
    #[test]
    fn every_lane_names_its_provider_health_tools() {
        assert_eq!(lane("blend1").health_tools, &["doctor"]);
        assert_eq!(lane("sdm1").health_tools, &["sdm1_health"]);
        assert_eq!(lane("sem1").health_tools, &["sem1_probe"]);
        for name in ["sys1", "gen1", "browser"] {
            assert!(
                lane(name).health_tools.is_empty(),
                "{name} exposes no provider health tool"
            );
        }
    }

    /// A provider-map health report — sdm1's and sem1's shape — parses
    /// into per-lane availability, sorted by provider, and a payload
    /// that is not the shape reads as nothing rather than as a lane that
    /// broke.
    #[test]
    fn a_provider_map_health_report_parses_into_availability() {
        let report = r#"{
            "tabpfn_v2": { "name": "TabPFN v2", "available": true, "reason": "ok" },
            "nvidia_sdm": { "name": "NVIDIA SDM", "available": false,
                            "reason": "not configured — set TABPFN_API_KEY" }
        }"#;
        let parsed = parse_health_map(report);
        assert_eq!(parsed.len(), 2, "{parsed:?}");
        assert_eq!(parsed[0].provider, "nvidia_sdm", "sorted by provider");
        assert!(!parsed[0].available);
        assert!(parsed[0].reason.contains("TABPFN_API_KEY"), "{parsed:?}");
        assert!(parsed[1].available);
        assert_eq!(parsed[1].lane, "TabPFN v2");
        assert!(parse_health_map("not json").is_empty());
        assert!(parse_health_map("[1, 2]").is_empty());
    }

    #[test]
    fn the_registrys_env_names_follow_the_forks_convention() {
        assert_eq!(lane("sem1").python_path_env, "SEM1_PYTHONPATH");
        assert_eq!(lane("sem1").home_env, "FLEET_LANE_SEM1_HOME");
        // blend1 is the fifth: the blended lane, its own path var and
        // per-lane home override beside it.
        assert_eq!(lane("blend1").python_path_env, "BLEND1_PYTHONPATH");
        assert_eq!(lane("blend1").home_env, "FLEET_LANE_BLEND1_HOME");
        assert_eq!(lane("blend1").role, "grounded");
        // Its keys ride through; Fleet's own never does (the allowlist
        // test below walks every lane, blend1 included).
        assert!(lane("blend1").passthrough.contains(&"LIQUID_API_KEY"));
        assert!(lane("blend1").passthrough.contains(&"TABPFN_API_KEY"));
        // The escalation leg's wiring (upstream ab4ac55): the target's
        // model, base and key-env names, and the default key variable
        // the key-env name falls back to.
        for escalation in [
            "BLEND1_ESCALATION_MODEL",
            "BLEND1_ESCALATION_BASE",
            "BLEND1_ESCALATION_KEY_ENV",
            "BLEND1_ESCALATION_API_KEY",
        ] {
            assert!(
                lane("blend1").passthrough.contains(&escalation),
                "{escalation} rides through"
            );
        }
        // The corpus tools' locators beside them.
        assert!(lane("blend1").passthrough.contains(&"BLEND1_VECTORS_DIR"));
        assert!(lane("blend1").passthrough.contains(&"BLEND1_PROJECT"));
        // The critic chain's per-critic overrides and the telemetry
        // root: the core reads BLEND1_CRITIC_<N>_{BASE,MODEL,KEY_ENV}
        // and BLEND1_LOG_DIR directly, so without these names an
        // owner's override (or a relocated log root) is unreachable
        // under fleet's launch — every sibling lane already carries
        // its own log dir.
        for n in [1, 2] {
            for suffix in ["BASE", "MODEL", "KEY_ENV"] {
                let name = format!("BLEND1_CRITIC_{n}_{suffix}");
                assert!(
                    lane("blend1").passthrough.contains(&name.as_str()),
                    "{name} rides through"
                );
            }
        }
        assert!(lane("blend1").passthrough.contains(&"BLEND1_LOG_DIR"));
    }

    #[test]
    fn no_lane_passes_the_deepseek_key_through() {
        // KI-031: the allowlist is a decision. No lane has any use for
        // Fleet's own credential, so it is on none of the lists.
        for lane in LANES {
            assert!(
                !lane.passthrough.contains(&"DEEPSEEK_API_KEY"),
                "{} must not declare DEEPSEEK_API_KEY",
                lane.name
            );
            assert!(
                !lane.passthrough.contains(&"DEEPSEEK_BASE_URL"),
                "{} must not declare DEEPSEEK_BASE_URL",
                lane.name
            );
        }
        // And every declared name is one the lane's own source reads —
        // no accidental `*` and no unrelated credential.
        for lane in LANES {
            assert!(
                !lane.passthrough.is_empty(),
                "{} declares nothing",
                lane.name
            );
            assert!(
                lane.passthrough
                    .iter()
                    .all(|name| name.starts_with(|c: char| c.is_ascii_uppercase())),
                "{} declares a non-constant-shaped name",
                lane.name
            );
        }
    }

    #[test]
    fn the_launch_spec_replicates_the_forks_launcher() {
        let home = PathBuf::from("/opt/services/sem1");
        let spec = launch_spec(&lane("sem1"), &home);
        assert_eq!(spec.program, "python3");
        assert_eq!(
            spec.args,
            vec![
                "-u".to_string(),
                "/opt/services/sem1/service/mcp_server.py".to_string()
            ]
        );
        assert_eq!(
            spec.env,
            vec![(
                "SEM1_PYTHONPATH".to_string(),
                "/opt/services/sem1/src".to_string()
            )]
        );
        // The child's environment beyond the base is exactly the lane's
        // allowlist — no inherited ambient names.
        assert_eq!(
            spec.passthrough,
            lane("sem1")
                .passthrough
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(spec.cwd, Some(home.clone()));
        assert_eq!(
            launch_command(&lane("sem1"), &home),
            "SEM1_PYTHONPATH=/opt/services/sem1/src python3 -u /opt/services/sem1/service/mcp_server.py"
        );
    }

    #[test]
    fn blend1s_launch_spec_wires_the_sibling_cores_beside_its_own() {
        let home = PathBuf::from("/opt/services/blend1");
        let spec = launch_spec(&lane("blend1"), &home);
        assert_eq!(spec.program, "python3");
        assert_eq!(
            spec.args,
            vec![
                "-u".to_string(),
                "/opt/services/blend1/service/mcp_server.py".to_string()
            ]
        );
        // Its own core first, then the three siblings the ask() composes
        // (plus the voice lane's), each resolved beside the lane under
        // the services root — the house path mechanism, not a pip
        // install into blend1's venv.
        assert_eq!(
            spec.env,
            vec![
                (
                    "BLEND1_PYTHONPATH".to_string(),
                    "/opt/services/blend1/src".to_string()
                ),
                (
                    "BLEND1_SYS1_PYTHONPATH".to_string(),
                    "/opt/services/sys1/src".to_string()
                ),
                (
                    "BLEND1_SEM1_PYTHONPATH".to_string(),
                    "/opt/services/sem1/src".to_string()
                ),
                (
                    "BLEND1_SDM1_PYTHONPATH".to_string(),
                    "/opt/services/sdm1/src".to_string()
                ),
                (
                    "BLEND1_GEN1_PYTHONPATH".to_string(),
                    "/opt/services/gen1/src".to_string()
                ),
            ]
        );
        // The blended lane's own keys ride through; Fleet's own never
        // does — the allowlist test above walks every lane, blend1
        // included, and this names the eight beside it.
        assert_eq!(
            spec.passthrough,
            lane("blend1")
                .passthrough
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(spec.cwd, Some(home.clone()));
        // Every other lane's spec stays exactly one path var: the
        // sibling wiring is the blend's alone.
        for other in LANES.iter().filter(|other| other.name != "blend1") {
            let spec = launch_spec(other, Path::new("/opt/services/other"));
            assert_eq!(spec.env.len(), 1, "{} carries one path var", other.name);
        }
    }

    #[test]
    fn a_relative_home_is_refused_with_a_named_reason() {
        // A relative home passes every layout check and still launches
        // from wherever the caller stood, so the check is on absoluteness.
        let reason = relative_home_reason("FLEET_LANE_SEM1_HOME", Path::new("services/sem1"))
            .expect("a relative path must be refused");
        assert!(reason.contains("FLEET_LANE_SEM1_HOME"), "{reason}");
        assert!(reason.contains("services/sem1"), "{reason}");
        assert!(reason.contains("relative"), "{reason}");
        // An absolute one is fine.
        assert_eq!(
            relative_home_reason("FLEET_LANE_SEM1_HOME", Path::new("/opt/sem1")),
            None
        );
    }

    #[test]
    fn a_relative_per_lane_home_reports_the_reason_not_a_layout_failure() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _sem1 = EnvVar::set("FLEET_LANE_SEM1_HOME", "services/sem1");
        match resolve_home(&lane("sem1")) {
            LaneHome::Unavailable { reason } => {
                assert!(reason.contains("relative"), "{reason}");
                assert!(reason.contains("FLEET_LANE_SEM1_HOME"), "{reason}");
            }
            resolved => panic!("a relative home must not resolve: {resolved:?}"),
        }
    }

    #[test]
    fn a_relative_services_root_is_a_typed_error_not_a_silent_fallback() {
        let _guard = ENV_LOCK.lock();
        let _root = EnvVar::set(SERVICES_ROOT_ENV, "services");
        assert_eq!(
            services_root(),
            Err(RootError::Relative("services".to_string()))
        );
        // The reason reaches the lane row rather than collapsing into
        // "not found".
        let reason = RootError::Relative("services".to_string()).to_string();
        assert!(reason.contains("FLEET_SERVICES_ROOT=services"), "{reason}");
        assert!(reason.contains("relative"), "{reason}");
        assert!(RootError::NotFound
            .to_string()
            .contains("FLEET_SERVICES_ROOT"));
    }

    #[test]
    fn a_named_lane_resolves_its_spec_and_an_unknown_one_names_itself() {
        // Resolution reads the ambient `FLEET_SERVICES_ROOT`, so this test
        // serializes against the env-mutating ones below the way they
        // serialize against each other.
        let _guard = ENV_LOCK.lock();
        // The agent loop resolves a lane by name on its first call, so an
        // unknown name must be a reason, not a panic and not silence.
        let error = spec_for("nope").expect_err("an unknown lane has no spec");
        assert!(error.contains("no lane named `nope`"), "{error}");
        // Every registry lane either resolves or names why it cannot; the
        // vendored copies resolve.
        for lane in LANES {
            let outcome = spec_for(lane.name);
            if lane.name == "sem1" {
                let spec = outcome.expect("the vendored sem1 resolves");
                assert_eq!(spec.program, "python3");
                assert!(
                    spec.args[1].ends_with("service/mcp_server.py"),
                    "{:?}",
                    spec.args
                );
            } else {
                assert!(
                    outcome.is_ok(),
                    "{} must resolve from the vendored copy",
                    lane.name
                );
            }
        }
    }

    #[test]
    fn a_lane_without_a_home_reports_a_named_reason() {
        let _guard = ENV_LOCK.lock();
        // No env override is set for this lane in the test process, so the
        // resolution falls through to the bundled root or names its absence.
        let home = resolve_home(&lane("gen1"));
        match home {
            LaneHome::Resolved { home, .. } => {
                assert!(looks_like_service_home(&home));
            }
            LaneHome::Unavailable { reason } => {
                assert!(
                    reason.contains("FLEET_LANE_GEN1_HOME") || reason.contains("services root")
                );
            }
        }
    }

    #[test]
    fn the_services_root_walk_finds_a_repo_root_from_a_nested_exe() {
        let _guard = ENV_LOCK.lock();
        // The walk starts at the executable: in tests that is
        // target/debug/deps/..., and the repo root carries both services/
        // and Cargo.toml. This asserts the discovery mechanism, not the
        // machine's install.
        if let Ok(root) = services_root() {
            assert!(root.join("sem1").join("VENDOR.json").is_file(), "{root:?}");
        }
    }

    #[test]
    fn a_home_needs_the_mcp_server_and_the_src_tree() {
        let scratch = std::env::temp_dir().join(format!("fleet-lanes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("service")).unwrap();
        std::fs::create_dir_all(scratch.join("src")).unwrap();
        // Incomplete: no mcp_server.py yet.
        assert!(!looks_like_service_home(&scratch));
        std::fs::write(scratch.join("service").join("mcp_server.py"), b"# stub").unwrap();
        assert!(looks_like_service_home(&scratch));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_headless_render_is_cards_with_dots_never_a_table() {
        let health = vec![
            LaneHealth {
                name: "sem1".to_string(),
                role: "embeddings".to_string(),
                home: Some(PathBuf::from("/repo/services/sem1")),
                source: Some(HomeSource::Bundled),
                state: LaneState::Ok,
                server_version: Some("0.1.0".to_string()),
                tools: vec!["sem1_embed".to_string(), "sem1_probe".to_string()],
                doctor: Vec::new(),
                detail: "SEM1_PYTHONPATH=/repo/services/sem1/src python3 -u /repo/services/sem1/service/mcp_server.py".to_string(),
                elapsed_ms: 412,
            },
            LaneHealth {
                name: "gen1".to_string(),
                role: "media".to_string(),
                home: None,
                source: None,
                state: LaneState::Unavailable,
                server_version: None,
                tools: Vec::new(),
                doctor: Vec::new(),
                detail: "no services root found (set FLEET_SERVICES_ROOT or FLEET_LANE_GEN1_HOME)".to_string(),
                elapsed_ms: 0,
            },
        ];
        let rendered = render(&health);
        // Cards: box-drawing borders, one per lane.
        assert_eq!(rendered.matches('┌').count(), 2, "{rendered}");
        assert_eq!(rendered.matches('└').count(), 2, "{rendered}");
        // Status dots, colored by state.
        assert!(
            rendered.contains(&format!("{}✓ ok{RESET}", palette::OK)),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!("{}○ unavailable{RESET}", palette::DIM)),
            "{rendered}"
        );
        // The facts a health command owes: role, version, tool count, names.
        assert!(
            rendered.contains("embeddings · v0.1.0 · 2 tools"),
            "{rendered}"
        );
        assert!(rendered.contains("sem1_embed · sem1_probe"), "{rendered}");
        assert!(rendered.contains("2 lanes · 1 ok · 1 not ok"), "{rendered}");
        // No plain-text table anywhere.
        assert!(!rendered.contains("| name |"), "{rendered}");
    }

    #[test]
    fn an_error_state_renders_the_red_dot_and_the_reason() {
        let health = vec![LaneHealth {
            name: "sys1".to_string(),
            role: "judgment".to_string(),
            home: Some(PathBuf::from("/repo/services/sys1")),
            source: Some(HomeSource::Bundled),
            state: LaneState::Error,
            server_version: None,
            tools: Vec::new(),
            doctor: Vec::new(),
            detail: "`tools/list` timed out after 10s".to_string(),
            elapsed_ms: 10_000,
        }];
        let rendered = render(&health);
        assert!(
            rendered.contains(&format!("{}✕ error{RESET}", palette::ERROR)),
            "{rendered}"
        );
        assert!(rendered.contains("timed out"), "{rendered}");
    }

    #[test]
    fn parse_doctor_keeps_the_five_lanes_and_the_web_key_with_its_reason() {
        // A doctor answer as the adapter returns it: the five lanes an
        // ask() composes from, the web leg the report carries beside
        // them, and the other keys (the critic chain, the voice leg)
        // that still ride along unrendered.
        let report = parse_doctor(
            r#"{
                "route": {"available": true, "reason": null, "provider": "liquid (d1) via sys1"},
                "semantic": {"available": false, "reason": "endpoint unreachable", "provider": "llama-server (EmbeddingGemma-2)"},
                "tabular": {"available": false, "reason": "not configured — set TABPFN_API_KEY", "provider": "tabpfn-hosted (Prior Labs)"},
                "generate": {"available": false, "reason": "missing LIQUID_API_KEY", "provider": "lfm2.5-24b-a2b via api.liquid.ai"},
                "validate": {"available": false, "reason": "missing FASTINO_API_KEY", "provider": "glide → d1"},
                "critics": [{"critic": "qwen3.8-flash", "configured": false}],
                "voice": {"available": false, "reason": "missing DASHSCOPE_API_KEY"},
                "web": {"available": false, "reason": "missing TINYFISH_API_KEY", "provider": "tinyfish (search + fetch)"}
            }"#,
        );
        // The five lanes, in the blend's own order — unchanged.
        let lanes: Vec<&str> = report
            .lanes
            .iter()
            .map(|entry| entry.lane.as_str())
            .collect();
        assert_eq!(
            lanes,
            vec!["route", "semantic", "tabular", "generate", "validate"]
        );
        assert!(report.lanes[0].available);
        // A null reason reads as the empty string, and the card falls
        // back to the provider for an available lane.
        assert!(report.lanes[0].reason.is_empty());
        assert_eq!(report.lanes[0].provider, "liquid (d1) via sys1");
        for entry in &report.lanes[1..] {
            assert!(!entry.available, "{} must be unavailable", entry.lane);
            assert!(!entry.reason.is_empty(), "{} names its reason", entry.lane);
        }
        // The web leg: the key the report carries, kept beside the
        // five with its named reason instead of dropped as an
        // unrendered extra.
        let web = report.web.expect("the report carries the web key");
        assert_eq!(web.lane, "web");
        assert!(!web.available);
        assert_eq!(web.reason, "missing TINYFISH_API_KEY");
        assert_eq!(web.provider, "tinyfish (search + fetch)");
        // A report with no web key parses none, and the five still
        // read as they always did.
        let without = parse_doctor(
            r#"{"route": {"available": true, "reason": null, "provider": "liquid (d1) via sys1"}}"#,
        );
        assert!(without.web.is_none(), "{:?}", without.web);
        assert_eq!(without.lanes.len(), 1);
        // A report that cannot be read is empty, never a panic: the card
        // renders without it and the probe stays Ok.
        let unreadable = parse_doctor("not json");
        assert!(unreadable.lanes.is_empty());
        assert!(unreadable.web.is_none());
        assert!(parse_doctor("[]").lanes.is_empty());
        // A lane missing from the object, or one whose `available` is
        // not a boolean, is skipped rather than guessed at.
        let partial = parse_doctor(r#"{"route": {"available": "yes"}, "generate": {}}"#);
        assert!(partial.lanes.is_empty(), "{partial:?}");
        assert!(partial.web.is_none());
    }

    #[test]
    fn a_doctor_report_renders_status_dots_and_named_reasons_never_a_table() {
        let health = vec![LaneHealth {
            name: "blend1".to_string(),
            role: "grounded".to_string(),
            home: Some(PathBuf::from("/repo/services/blend1")),
            source: Some(HomeSource::Bundled),
            state: LaneState::Ok,
            server_version: Some("0.1.0".to_string()),
            tools: vec!["ask".to_string(), "doctor".to_string()],
            doctor: vec![
                LaneAvailability {
                    lane: "route".to_string(),
                    available: true,
                    reason: String::new(),
                    provider: "liquid (d1) via sys1".to_string(),
                },
                LaneAvailability {
                    lane: "semantic".to_string(),
                    available: false,
                    reason: "endpoint unreachable".to_string(),
                    provider: "llama-server (EmbeddingGemma-2)".to_string(),
                },
                LaneAvailability {
                    lane: "tabular".to_string(),
                    available: false,
                    reason: "not configured — set TABPFN_API_KEY".to_string(),
                    provider: "tabpfn-hosted (Prior Labs)".to_string(),
                },
                LaneAvailability {
                    lane: "generate".to_string(),
                    available: false,
                    reason: "missing LIQUID_API_KEY (set it in ~/.config/blend1/env)".to_string(),
                    provider: "lfm2.5-24b-a2b via api.liquid.ai".to_string(),
                },
                LaneAvailability {
                    lane: "validate".to_string(),
                    available: false,
                    reason: "missing FASTINO_API_KEY — the gate would ship ungated".to_string(),
                    provider: "glide → d1 (oversized states)".to_string(),
                },
                LaneAvailability {
                    lane: "web".to_string(),
                    available: false,
                    reason: "missing TINYFISH_API_KEY (set it in ~/.config/blend1/env)".to_string(),
                    provider: "tinyfish (search + fetch)".to_string(),
                },
            ],
            detail: "BLEND1_PYTHONPATH=/repo/services/blend1/src python3 -u /repo/services/blend1/service/mcp_server.py".to_string(),
            elapsed_ms: 812,
        }];
        let rendered = render(&health);
        // Status dots, colored by availability: green for the lane that
        // can answer, dim for the ones naming what they are missing.
        assert!(
            rendered.contains(&format!(
                "{}✓ route — liquid (d1) via sys1{RESET}",
                palette::OK
            )),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!(
                "{}○ semantic — endpoint unreachable{RESET}",
                palette::DIM
            )),
            "{rendered}"
        );
        // Every unavailable lane names its reason.
        for reason in [
            "endpoint unreachable",
            "not configured — set TABPFN_API_KEY",
            "missing LIQUID_API_KEY",
            "missing FASTINO_API_KEY",
        ] {
            assert!(rendered.contains(reason), "{reason} is named: {rendered}");
        }
        // The five lanes render in the blend's own order.
        let route_at = rendered.find("route").expect("the route lane renders");
        let validate_at = rendered
            .find("validate")
            .expect("the validate lane renders");
        assert!(route_at < validate_at, "{rendered}");
        // The web leg: its own line on blend1's card, the accented
        // label marking it as not one of the five sub-lane rows — it
        // is the web leg blend1 serves — with its reason named.
        assert!(
            rendered.contains(&format!(
                "{}web leg{RESET} {}○ — missing TINYFISH_API_KEY",
                palette::ACCENT,
                palette::DIM
            )),
            "{rendered}"
        );
        // It never renders in the five's row shape: no bare web row
        // among the sub-lanes.
        assert!(
            !rendered.contains(&format!("{}○ web — ", palette::DIM)),
            "{rendered}"
        );
        // A card, never a plain-text table.
        assert_eq!(rendered.matches('┌').count(), 1, "{rendered}");
        assert!(!rendered.contains("| lane |"), "{rendered}");
    }

    #[test]
    fn a_report_without_a_web_key_renders_no_web_line() {
        // A blend1 card whose doctor report carries the five sub-lanes
        // and no web key: the five render, and no web line appears —
        // the card never invents a leg the report did not name.
        let health = vec![LaneHealth {
            name: "blend1".to_string(),
            role: "grounded".to_string(),
            home: Some(PathBuf::from("/repo/services/blend1")),
            source: Some(HomeSource::Bundled),
            state: LaneState::Ok,
            server_version: Some("0.1.0".to_string()),
            tools: vec!["ask".to_string(), "doctor".to_string()],
            doctor: vec![LaneAvailability {
                lane: "route".to_string(),
                available: true,
                reason: String::new(),
                provider: "liquid (d1) via sys1".to_string(),
            }],
            detail: "BLEND1_PYTHONPATH=/repo/services/blend1/src python3 -u /repo/services/blend1/service/mcp_server.py".to_string(),
            elapsed_ms: 812,
        }];
        let rendered = render(&health);
        assert!(rendered.contains("route"), "{rendered}");
        assert!(!rendered.contains("web leg"), "{rendered}");
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-lanes-config-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The configured values reach the child for names the lane declares,
    /// and a name it does not declare never crosses.
    #[test]
    fn lanes_json_values_reach_the_spec_and_only_declared_names_cross() {
        let home = scratch("reach");
        std::fs::write(
            home.join(LANES_FILE),
            r#"{"lanes":{"sem1":{"env":{"SEM1_ST_VENV":"/opt/venv-st","SEM1_EVIL":"no"}}}}"#,
        )
        .unwrap();
        let spec = configured_spec_at(&lane("sem1"), Path::new("/repo/services/sem1"), &home)
            .expect("the spec");
        assert!(
            spec.env
                .iter()
                .any(|(name, value)| name == "SEM1_ST_VENV" && value == "/opt/venv-st"),
            "{:?}",
            spec.env
        );
        assert!(
            !spec.env.iter().any(|(name, _)| name == "SEM1_EVIL"),
            "{:?}",
            spec.env
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The dotenv sits over lanes.json, and a name the process already
    /// carries is never rewritten by either.
    #[test]
    fn the_dotenv_wins_over_the_lane_config() {
        let home = scratch("precedence");
        std::fs::write(
            home.join(LANES_FILE),
            r#"{"lanes":{"sem1":{"env":{"SEM1_ST_VENV":"/from/config"}}}}"#,
        )
        .unwrap();
        std::fs::write(home.join(".env"), "SEM1_ST_VENV=/from/dotenv\n").unwrap();
        let spec = configured_spec_at(&lane("sem1"), Path::new("/repo/services/sem1"), &home)
            .expect("the spec");
        let value = spec
            .env
            .iter()
            .find(|(name, _)| name == "SEM1_ST_VENV")
            .map(|(_, value)| value.as_str());
        assert_eq!(value, Some("/from/dotenv"));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// Per-call defaults fill only what the model left out.
    #[test]
    fn a_default_fills_only_an_absent_argument() {
        let mut defaults = std::collections::BTreeMap::new();
        defaults.insert("provider".to_string(), "llama-server".to_string());
        defaults.insert("model".to_string(), "embeddinggemma-2-BF16".to_string());
        let merged = with_defaults(&serde_json::json!({"model": "explicit-model"}), &defaults);
        assert_eq!(merged["provider"], "llama-server");
        assert_eq!(merged["model"], "explicit-model");
        assert_eq!(merged["provider"], "llama-server");
    }

    /// The lane's configured default is what a call reads when it has no
    /// explicit value.
    #[test]
    fn lane_arguments_read_the_configured_defaults() {
        let home = scratch("defaults");
        std::fs::write(
            home.join(LANES_FILE),
            r#"{"lanes":{"sem1":{"defaults":{"sem1_embed":{"provider":"st-worker","model":"google/embeddinggemma-2"}}}}}"#,
        )
        .unwrap();
        let merged = lane_arguments(
            &home,
            "sem1",
            "sem1_embed",
            &serde_json::json!({"provider": "llama-server"}),
        );
        assert_eq!(merged["provider"], "llama-server");
        assert_eq!(merged["model"], "google/embeddinggemma-2");
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The lane catalog carries the declared defaults the pickers offer.
    #[test]
    fn the_lane_catalog_carries_the_declared_defaults() {
        let sem1 = choices("sem1");
        assert!(
            sem1.iter()
                .any(|choice| choice.value == "llama-server" && choice.kind == "provider"),
            "{sem1:?}"
        );
        assert!(
            sem1.iter()
                .any(|choice| choice.value == "embeddinggemma-2-BF16" && choice.kind == "model"),
            "{sem1:?}"
        );
        let sys1 = choices("sys1");
        assert!(
            sys1.iter()
                .any(|choice| choice.value == "glide" && choice.kind == "provider"),
            "{sys1:?}"
        );
        let gen1 = choices("gen1");
        assert!(
            gen1.iter()
                .any(|choice| choice.value == "qwen-audio-3.0-tts-plus" && choice.kind == "model"),
            "{gen1:?}"
        );
    }

    #[test]
    fn a_long_detail_wraps_inside_the_card_without_truncating() {
        // Every character survives the wrap, in order.
        let text = "SEM1_PYTHONPATH=/very/long/path/to/services/sem1/src python3 -u /very/long/path/to/services/sem1/service/mcp_server.py";
        let chunks = wrap_detail(text, 40);
        assert!(chunks.len() > 1, "{chunks:?}");
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= 40));
        assert_eq!(&chunks.concat(), text);
        // An empty detail still renders one (empty) line.
        assert_eq!(wrap_detail("", 40), vec![""]);
    }
}
