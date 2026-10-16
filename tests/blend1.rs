//! blend1 — the blended lane's acceptance, driven through fleet's own
//! MCP client against the real vendored adapter, plus the registry and
//! install tables that declare it.
//!
//! The lane's core is stdlib-only and its three siblings are path
//! dependencies, so the handshake tests spawn
//! `services/blend1/service/mcp_server.py` through the production
//! launch spec — `BLEND1_PYTHONPATH` beside the sibling
//! `BLEND1_<SVC>_PYTHONPATH` wiring — exactly as `fleet lanes` and the
//! agent's tool loop do. The real-lane tests skip with a loud note when
//! `python3` is absent, the way `tests/integration.rs`'s sem1 lane
//! tests do, so a Python-less machine stays green without silently
//! passing.
//!
//! The lanes' availability the `doctor` report carries is the
//! machine's — keys present in `~/.config/blend1/env` or not, sibling
//! deps installed or not — so the assertion is on the shape: every lane
//! available, or unavailable naming its reason. Never a hang (the
//! client's budgets are the guard) and never a panic.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use fleet::install::{self, LANES};
use fleet::lanes::{self, Lane, LaneHome, LaneState};
use fleet::mcp::{McpServer, ServerSpec};

/// The blended lane's own keys and locators — the union of its legs' —
/// which the spawned lane's environment carries and Fleet's own never
/// does. The first eight are the legs' credentials (Liquid one key for
/// the d1 router, the 24B generative lane and the sys1-score leg;
/// Fastino's GLiDE gate; the critic chain; the optional web and voice
/// legs; the tabular leg); the rest are the escalation leg's wiring
/// (upstream ab4ac55), the telemetry root, the critic chain's
/// per-critic overrides, and the corpus tools' locators.
const BLEND1_KEYS: &[&str] = &[
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
];

/// The escalation leg's wiring, named separately so the assertion that
/// it rides through reads on its own line: the target's model id (which
/// arms the leg), its OpenAI-compatible base, the name of the variable
/// holding its key, and that default key variable itself.
const BLEND1_ESCALATION: &[&str] = &[
    "BLEND1_ESCALATION_MODEL",
    "BLEND1_ESCALATION_BASE",
    "BLEND1_ESCALATION_KEY_ENV",
    "BLEND1_ESCALATION_API_KEY",
];

/// The sibling cores the blended lane composes, path-wired beside its
/// own at launch.
const BLEND1_SIBLINGS: &[&str] = &["sys1", "sem1", "sdm1", "gen1"];

/// The critic chain's per-critic overrides and the telemetry root, named
/// separately so the assertion that they ride through reads on its own
/// line: the core reads `BLEND1_CRITIC_<N>_{BASE,MODEL,KEY_ENV}` (N =
/// 1..2, in chain order) and `BLEND1_LOG_DIR` directly, so without these
/// names an owner's critic override — or a relocated telemetry root — is
/// unreachable under fleet's launch, even though every sibling lane
/// already carries its own log dir.
const BLEND1_CRITICS_AND_LOG: &[&str] = &[
    "BLEND1_LOG_DIR",
    "BLEND1_CRITIC_1_BASE",
    "BLEND1_CRITIC_1_MODEL",
    "BLEND1_CRITIC_1_KEY_ENV",
    "BLEND1_CRITIC_2_BASE",
    "BLEND1_CRITIC_2_MODEL",
    "BLEND1_CRITIC_2_KEY_ENV",
];

/// The sub-lanes a `doctor` report carries for the card: the five an
/// `ask()` composes its answer from.
const DOCTOR_LANES: &[&str] = &["route", "semantic", "tabular", "generate", "validate"];

/// Is `python3` on PATH? The real-lane tests skip with a loud note when
/// it is not, so a Python-less machine stays green without silently
/// passing.
fn python3_available() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// One registry lane by name.
fn lane(name: &str) -> Lane {
    *lanes::LANES
        .iter()
        .find(|lane| lane.name == name)
        .expect("a registry lane")
}

/// blend1's resolved home; the vendored copy must always answer — the
/// lane layout check (`service/mcp_server.py` + `src/`) is what makes
/// it a home at all.
fn blend1_home() -> PathBuf {
    match lanes::resolve_home(&lane("blend1")) {
        LaneHome::Resolved { home, .. } => home,
        LaneHome::Unavailable { reason } => {
            panic!("blend1 must resolve from the vendored copy: {reason}")
        }
    }
}

/// blend1's production launch spec: `python3 -u <home>/service/mcp_server.py`
/// with the path wiring and the key allowlist the registry declares.
fn blend1_spec() -> ServerSpec {
    lanes::launch_spec(&lane("blend1"), &blend1_home())
}

/// The real vendored blend1 adapter, driven end to end through fleet's
/// own client: initialize, `tools/list` — the two tools the lane
/// contract names, with their argument schemas — and clean teardown.
#[tokio::test]
async fn the_blend1_adapter_handshakes_through_fleets_own_client() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the blend1 handshake");
        return;
    }
    let home = blend1_home();
    let spec = blend1_spec();
    // The spec carries the house path wiring: blend1's own core beside
    // the four siblings the ask() composes, each resolved beside the
    // lane under the services root.
    let wired = |name: &str| {
        spec.env
            .iter()
            .find(|(var, _)| var == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("{name} is wired: {:?}", spec.env))
    };
    assert_eq!(
        wired("BLEND1_PYTHONPATH"),
        home.join("src").to_string_lossy().into_owned()
    );
    let root = home
        .parent()
        .expect("the lane sits under a services root")
        .to_path_buf();
    for sibling in BLEND1_SIBLINGS {
        let expected = root
            .join(sibling)
            .join("src")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            wired(&format!("BLEND1_{}_PYTHONPATH", sibling.to_uppercase())),
            expected
        );
    }

    let server = McpServer::spawn(&spec).expect("spawn the blend1 lane");

    let initialize = server.initialize().await.expect("initialize");
    assert_eq!(initialize.protocol_version, "2024-11-05");
    assert_eq!(initialize.server_name, "blend1");
    assert_eq!(initialize.server_version, "0.1.0");

    let tools = server.list_tools().await.expect("tools/list");
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();
    // The record: the actual tools/list the handshake answered with.
    let listed = serde_json::json!({
        "tools": tools
            .iter()
            .map(|tool| serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            }))
            .collect::<Vec<_>>()
    });
    eprintln!(
        "blend1 tools/list: {}",
        serde_json::to_string_pretty(&listed).expect("the list serializes")
    );
    // The two tools the lane contract names.
    assert!(names.contains(&"ask"), "ask is declared: {names:?}");
    assert!(names.contains(&"doctor"), "doctor is declared: {names:?}");
    // The corpus tools ride beside them (the vendored adapter's
    // search/refresh/seed surface).
    for corpus_tool in ["search", "refresh", "seed"] {
        assert!(
            names.contains(&corpus_tool),
            "{corpus_tool} is declared: {names:?}"
        );
    }

    // ask's schema: `prompt` required, the tabular and retrieval
    // arguments optional.
    let ask = tools.iter().find(|tool| tool.name == "ask").expect("ask");
    assert_eq!(ask.input_schema["type"], serde_json::json!("object"));
    assert_eq!(ask.input_schema["required"], serde_json::json!(["prompt"]));
    let properties = ask.input_schema["properties"]
        .as_object()
        .expect("ask's properties");
    for argument in ["prompt", "table", "target", "task_type", "retrieve"] {
        assert!(
            properties.contains_key(argument),
            "ask takes `{argument}`: {properties:?}"
        );
    }
    assert_eq!(properties["prompt"]["type"], serde_json::json!("string"));
    assert_eq!(properties["table"]["type"], serde_json::json!("array"));
    assert_eq!(properties["target"]["type"], serde_json::json!("string"));
    assert_eq!(properties["retrieve"]["type"], serde_json::json!("boolean"));
    assert_eq!(
        properties["task_type"]["enum"],
        serde_json::json!(["classification", "regression"])
    );

    // doctor's schema: no arguments at all.
    let doctor = tools
        .iter()
        .find(|tool| tool.name == "doctor")
        .expect("doctor");
    assert_eq!(doctor.input_schema["type"], serde_json::json!("object"));
    assert_eq!(
        doctor.input_schema["properties"],
        serde_json::json!({}),
        "doctor takes no arguments"
    );

    // A tools/call the server refuses comes back as a typed JSON-RPC
    // error, not a hang and not a panic — the ask tool's own contract,
    // the sem1 lane test's shape.
    let error = server
        .call_tool("ask", serde_json::json!({}))
        .await
        .expect_err("an ask with no prompt must be refused");
    match error {
        fleet::mcp::McpError::Rpc { code, message } => {
            assert_eq!(code, -32602);
            assert!(message.contains("prompt"), "{message}");
        }
        other => panic!("expected an Rpc error, got {other:?}"),
    }

    // Teardown: stdin closes, the child exits, nothing lingers.
    server.close().await.expect("clean teardown");
}

/// The adapter's validation branches, driven through fleet's own client
/// over a real handshake: every malformed argument is a **named** refusal
/// — a `-32602` naming the offending field, or a `-32601` for a tool
/// that does not exist — never a traceback and never a hang. These fire
/// before the core is touched, so they hold on any machine, with or
/// without the sibling lanes' providers.
#[tokio::test]
async fn every_malformed_argument_is_a_named_refusal_never_a_traceback() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the blend1 validation refusals");
        return;
    }
    let server = McpServer::spawn(&blend1_spec()).expect("spawn the blend1 lane");
    server.initialize().await.expect("initialize");
    server.list_tools().await.expect("tools/list");

    // (tool, arguments, the JSON-RPC code, the words the refusal names).
    let refusals: &[(&str, serde_json::Value, i64, &str)] = &[
        // A table with no target: the column TabPFN predicts is required.
        (
            "ask",
            serde_json::json!({"prompt": "will customer 1042 churn?", "table": [{"tenure_months": 4}]}),
            -32602,
            "target",
        ),
        // A task type outside the enum the schema declares.
        (
            "ask",
            serde_json::json!({"prompt": "hi", "task_type": "clustering"}),
            -32602,
            "task_type",
        ),
        // A k that is not a positive integer.
        (
            "search",
            serde_json::json!({"query": "churn playbook", "k": 0}),
            -32602,
            "positive integer",
        ),
        // A seed item with no text.
        (
            "seed",
            serde_json::json!({"items": [{"key": "churn-playbook"}]}),
            -32602,
            "text",
        ),
        // A sources entry that is blank.
        (
            "refresh",
            serde_json::json!({"sources": ["   "]}),
            -32602,
            "sources",
        ),
        // A tool the adapter does not declare.
        ("nope", serde_json::json!({}), -32601, "Unknown tool"),
    ];
    for (tool, arguments, code, names) in refusals {
        let error = server
            .call_tool(tool, arguments.clone())
            .await
            .expect_err("a malformed argument must be refused, not answered");
        match error {
            fleet::mcp::McpError::Rpc { code: got, message } => {
                assert_eq!(
                    got, *code,
                    "{tool} with {arguments} names its code: {message}"
                );
                assert!(message.contains(names), "{tool} names {names}: {message}");
            }
            other => panic!("expected an Rpc error for {tool} with {arguments}, got {other:?}"),
        }
    }

    server.close().await.expect("clean teardown");
}

/// A `doctor` call answers the per-lane availability with named
/// reasons — the siblings' deps and keys may be uninstalled or unset,
/// and unavailable-with-reason is the correct answer. The budget is
/// the guard: a hang fails it, and a panic never happens.
#[tokio::test]
async fn a_doctor_call_names_every_lanes_availability_never_a_hang() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the blend1 doctor call");
        return;
    }
    let server = McpServer::spawn(&blend1_spec()).expect("spawn the blend1 lane");
    server.initialize().await.expect("initialize");
    server.list_tools().await.expect("tools/list");

    let call = server
        .call_tool("doctor", serde_json::json!({}))
        .await
        .expect("doctor answers — a hang or a panic fails here");
    assert!(
        !call.is_error,
        "the doctor report is not an error: {}",
        call.text()
    );
    let report: serde_json::Value =
        serde_json::from_str(&call.text()).expect("the doctor report is JSON");
    let report = report.as_object().expect("the report is an object");
    for lane_name in DOCTOR_LANES {
        let entry = report
            .get(*lane_name)
            .unwrap_or_else(|| panic!("{lane_name} is in the report: {report:?}"));
        assert!(entry.is_object(), "{lane_name} is a row: {entry}");
        assert!(
            entry["available"].is_boolean(),
            "{lane_name} names its availability: {entry}"
        );
        // The contract: a lane that cannot answer names why — a missing
        // key names its variable, never its value.
        if entry["available"] == serde_json::json!(false) {
            let reason = entry["reason"].as_str().unwrap_or_default();
            assert!(
                !reason.trim().is_empty(),
                "{lane_name} names its reason: {entry}"
            );
        }
    }
    // The re-vendored core's two newer rows ride beside the five (the
    // escalation leg, upstream ab4ac55, and the sys1-score leg): each
    // an availability with its reason — the escalation target a named
    // skip until its model and base are configured.
    for row in ["escalation", "score"] {
        let entry = report
            .get(row)
            .unwrap_or_else(|| panic!("{row} is in the report: {report:?}"));
        assert!(
            entry["available"].is_boolean(),
            "{row} names its availability: {entry}"
        );
        if entry["available"] == serde_json::json!(false) {
            let reason = entry["reason"].as_str().unwrap_or_default();
            assert!(!reason.trim().is_empty(), "{row} names its reason: {entry}");
        }
    }

    server.close().await.expect("clean teardown");
}

/// The registry's probe wrapper over the real lane: the typed health
/// row the headless command and the TUI both render, doctor report and
/// all, at the house visual bar.
#[tokio::test]
async fn the_lane_probe_carries_blend1s_doctor_report() {
    if !python3_available() {
        eprintln!("python3 is not on PATH; skipping the blend1 probe");
        return;
    }
    let health = lanes::probe_lane(&lane("blend1")).await;
    assert_eq!(health.state, LaneState::Ok, "{}", health.detail);
    assert_eq!(health.name, "blend1");
    assert_eq!(health.role, "grounded");
    assert_eq!(health.server_version.as_deref(), Some("0.1.0"));
    assert!(
        health.tools.contains(&"ask".to_string()),
        "{:?}",
        health.tools
    );
    assert!(
        health.tools.contains(&"doctor".to_string()),
        "{:?}",
        health.tools
    );
    assert!(health.detail.contains("python3 -u"), "{}", health.detail);
    // The doctor report rode back through the typed shape: the five
    // lanes, in the blend's own order, each available or naming why
    // not — and the web leg's own entry beside them, which is not a
    // blend sub-lane but the web leg blend1 serves.
    let lanes: Vec<&str> = health
        .doctor
        .iter()
        .map(|entry| entry.lane.as_str())
        .collect();
    assert_eq!(
        &lanes[..DOCTOR_LANES.len()],
        DOCTOR_LANES,
        "{:?}",
        health.doctor
    );
    assert_eq!(
        lanes.len(),
        DOCTOR_LANES.len() + 1,
        "five sub-lanes plus the web leg, nothing else"
    );
    assert_eq!(lanes.last().copied(), Some("web"), "{:?}", health.doctor);
    for entry in &health.doctor {
        if !entry.available {
            assert!(
                !entry.reason.trim().is_empty(),
                "{} names its reason: {entry:?}",
                entry.lane
            );
        }
    }
    // The card renders it at the house visual bar: status dots and
    // named reasons, never a plain-text table.
    let rendered = lanes::render(std::slice::from_ref(&health));
    assert!(rendered.contains("grounded"), "{rendered}");
    for lane_name in DOCTOR_LANES {
        assert!(
            rendered.contains(lane_name),
            "{lane_name} renders: {rendered}"
        );
    }
    assert!(
        rendered.contains("web leg"),
        "the web leg renders its own line: {rendered}"
    );
    assert!(!rendered.contains("| lane |"), "{rendered}");
}

/// The launch spec and the env scrubbing: blend1's own keys ride
/// through, the sibling cores are path-wired beside its own, and
/// Fleet's own credential never reaches the child (KI-031).
#[test]
fn the_blend1_launch_spec_carries_its_keys_and_scrubs_fleets_own() {
    let home = blend1_home();
    let spec = blend1_spec();
    assert_eq!(spec.program, "python3");
    assert_eq!(spec.args.len(), 2);
    assert_eq!(spec.args[0], "-u");
    assert!(
        spec.args[1].ends_with("service/mcp_server.py"),
        "{:?}",
        spec.args
    );
    assert_eq!(spec.cwd, Some(home.clone()));
    // Its own core first, then the four siblings — five wired paths.
    assert_eq!(spec.env.len(), 1 + BLEND1_SIBLINGS.len(), "{:?}", spec.env);
    // Every declared name rides through; Fleet's own are not.
    for key in BLEND1_KEYS {
        assert!(
            spec.passthrough.iter().any(|name| name == key),
            "{key} rides through"
        );
    }
    assert!(
        !spec
            .passthrough
            .iter()
            .any(|name| name == "DEEPSEEK_API_KEY"),
        "Fleet's own key is on no lane's list"
    );
    assert!(!spec
        .passthrough
        .iter()
        .any(|name| name == "DEEPSEEK_BASE_URL"));

    // The scrubbing itself, on the pure seam: a parent carrying every
    // name yields a child with blend1's keys and without Fleet's own
    // or anything undeclared.
    let mut parent: HashMap<OsString, OsString> = HashMap::new();
    for key in BLEND1_KEYS {
        parent.insert(OsString::from(key), OsString::from("a-lane-key"));
    }
    parent.insert(
        OsString::from("DEEPSEEK_API_KEY"),
        OsString::from("sk-fleets-own"),
    );
    parent.insert(
        OsString::from("DEEPSEEK_BASE_URL"),
        OsString::from("https://api.deepseek.com"),
    );
    parent.insert(OsString::from("UNRELATED_SECRET"), OsString::from("never"));
    parent.insert(OsString::from("PATH"), OsString::from("/usr/bin"));
    parent.insert(OsString::from("HOME"), OsString::from("/home/operator"));
    let child = fleet::mcp::child_env(&spec, &parent);
    let carried = |name: &str| {
        child
            .iter()
            .find(|(var, _)| var.to_string_lossy() == name)
            .map(|(_, value)| value.to_string_lossy().into_owned())
    };
    for key in BLEND1_KEYS {
        assert_eq!(
            carried(key).as_deref(),
            Some("a-lane-key"),
            "{key} reaches the child"
        );
    }
    // The escalation leg's wiring passes, named on its own line: the
    // leg is a named skip until its target is configured, and the
    // configuration is ambient state the lane cannot discover itself.
    for escalation in BLEND1_ESCALATION {
        assert_eq!(
            carried(escalation).as_deref(),
            Some("a-lane-key"),
            "{escalation} reaches the child"
        );
    }
    assert_eq!(carried("BLEND1_VECTORS_DIR").as_deref(), Some("a-lane-key"));
    assert_eq!(carried("BLEND1_PROJECT").as_deref(), Some("a-lane-key"));
    // The critic chain's per-critic overrides and the telemetry root
    // pass, named on their own line: the core reads them directly, so
    // without them an owner's override is unreachable under this launch.
    for name in BLEND1_CRITICS_AND_LOG {
        assert_eq!(
            carried(name).as_deref(),
            Some("a-lane-key"),
            "{name} reaches the child"
        );
    }
    // The path wiring rides last, so the lane's own declaration wins.
    let own_src = home.join("src").to_string_lossy().into_owned();
    assert_eq!(
        carried("BLEND1_PYTHONPATH").as_deref(),
        Some(own_src.as_str())
    );
    for sibling in BLEND1_SIBLINGS {
        let name = format!("BLEND1_{}_PYTHONPATH", sibling.to_uppercase());
        assert!(carried(&name).is_some(), "{name} reaches the child");
    }
    // The base env rides; everything else stays behind.
    assert_eq!(carried("PATH").as_deref(), Some("/usr/bin"));
    assert_eq!(carried("HOME").as_deref(), Some("/home/operator"));
    assert_eq!(
        carried("DEEPSEEK_API_KEY"),
        None,
        "Fleet's own key never reaches the child"
    );
    assert_eq!(carried("DEEPSEEK_BASE_URL"), None);
    assert_eq!(
        carried("UNRELATED_SECRET"),
        None,
        "an undeclared name stays behind"
    );
}

/// The two lane tables carry blend1 as the fifth entry: the install's
/// venv bootstrap (the union of the siblings' package sets, the
/// sibling path-wiring named in its note) and the registry's launch
/// identity (its own path var, its home override, its key allowlist).
#[test]
fn both_lane_tables_carry_blend1_as_the_fifth_entry() {
    // The install's table: the venvs `fleet install --lanes` builds.
    assert_eq!(LANES.len(), 5, "the fifth lane is the last: {LANES:?}");
    assert_eq!(LANES[4].name, "blend1");
    let blend1 = LANES
        .iter()
        .find(|lane| lane.name == "blend1")
        .expect("the fifth lane");
    assert_eq!(blend1.role, "grounded");
    // The union of the three siblings' sets: blend1's own core is
    // stdlib-only, so its venv serves the cores its lanes compose.
    assert_eq!(
        blend1.packages,
        &[
            "gliner2[local]",
            "structured-data-models",
            "sentence-transformers"
        ]
    );
    // No export: BLEND1_PYTHONPATH locates the core's src
    // (services/blend1/src), not this venv root, and the launch spec
    // sets the whole wiring at spawn.
    assert_eq!(blend1.env_var, None);
    // The note names the sibling path-wiring: the cores resolve through
    // the `<SVC>_PYTHONPATH` names, never a pip install here.
    for wired in [
        "BLEND1_SYS1_PYTHONPATH",
        "BLEND1_SEM1_PYTHONPATH",
        "BLEND1_SDM1_PYTHONPATH",
    ] {
        assert!(blend1.note.contains(wired), "{} names {wired}", blend1.note);
    }
    assert!(
        blend1.note.contains("services/<svc>/src"),
        "{}",
        blend1.note
    );
    // The wiring line the install prints is the note itself: no export,
    // because an `export BLEND1_PYTHONPATH=<venv>` would be a wrong
    // value for a hand-run adapter.
    let venv = PathBuf::from("/home/x/.fleet/venvs/blend1");
    let wiring = install::lane_wiring(blend1, &venv);
    assert!(
        !wiring.starts_with("export "),
        "blend1 exports nothing: {wiring}"
    );
    assert!(!wiring.contains("BLEND1_PYTHONPATH=/home"), "{wiring}");
    assert!(wiring.contains("BLEND1_SDM1_PYTHONPATH"), "{wiring}");
    assert!(
        wiring.starts_with("the path wiring is automatic at spawn"),
        "{wiring}"
    );

    // The registry's table: the launch identity the probe and the
    // agent's tool loop resolve.
    let names: Vec<&str> = lanes::LANES.iter().map(|lane| lane.name).collect();
    assert_eq!(names, vec!["sys1", "sdm1", "sem1", "gen1", "blend1"]);
    let registry = lane("blend1");
    assert_eq!(registry.role, "grounded");
    assert_eq!(registry.python_path_env, "BLEND1_PYTHONPATH");
    assert_eq!(registry.home_env, "FLEET_LANE_BLEND1_HOME");
    for key in BLEND1_KEYS {
        assert!(registry.passthrough.contains(key), "{key} is declared");
    }
    assert!(!registry.passthrough.contains(&"DEEPSEEK_API_KEY"));
}
