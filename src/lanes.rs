//! The lane registry: the four vendored service cores, where each one
//! launches from, and the live health probe behind `fleet lanes`.
//!
//! Launch details are replicated faithfully from the fork's
//! `@fleet/dsh-components` bundle: `python3 -u <home>/service/mcp_server.py`
//! with `<SVC>_PYTHONPATH=<home>/src` in the child env and the lane's home
//! as the working directory. Home resolution keeps the fork's order — a
//! per-lane environment home, then the bundled copy under `services/`,
//! then unavailable with the reason named — so a live checkout can shadow
//! the vendored copy without touching this file.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::mcp::{McpError, McpServer, ServerSpec};

use palette::{ACCENT, BOLD, DIM, RESET, TEXT};

/// Environment variable overriding the whole services root.
pub const SERVICES_ROOT_ENV: &str = "FLEET_SERVICES_ROOT";

/// How many ancestors above the executable the services-root walk checks.
/// Twelve covers any real install depth, matching the fork's bound.
const ROOT_WALK_DEPTH: usize = 12;

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
}

/// The four lanes, in launch order.
pub const LANES: &[Lane] = &[
    Lane { name: "sys1", role: "judgment", python_path_env: "SYS1_PYTHONPATH", home_env: "FLEET_LANE_SYS1_HOME" },
    Lane { name: "sdm1", role: "tabular", python_path_env: "SDM1_PYTHONPATH", home_env: "FLEET_LANE_SDM1_HOME" },
    Lane { name: "sem1", role: "embeddings", python_path_env: "SEM1_PYTHONPATH", home_env: "FLEET_LANE_SEM1_HOME" },
    Lane { name: "gen1", role: "media", python_path_env: "GEN1_PYTHONPATH", home_env: "FLEET_LANE_GEN1_HOME" },
];

/// Where a resolved lane home came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeSource {
    /// The per-lane environment override named it.
    Env(String),
    /// The bundled `services/` tree answered.
    Bundled,
}

impl std::fmt::Display for HomeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HomeSource::Env(name) => write!(f, "{name}"),
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
    /// The launch command when the lane is ok; the failure reason when it
    /// is not.
    pub detail: String,
    /// How long the whole probe took.
    pub elapsed_ms: u128,
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

/// Resolve one lane's home: the environment override first, then the
/// bundled copy, then a named reason.
pub fn resolve_home(lane: &Lane) -> LaneHome {
    if let Some(home) = std::env::var_os(lane.home_env) {
        let home = PathBuf::from(home);
        return if looks_like_service_home(&home) {
            LaneHome::Resolved { home, source: HomeSource::Env(lane.home_env.to_string()) }
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
    let Some(root) = services_root() else {
        return LaneHome::Unavailable {
            reason: format!(
                "no services root found (set {} or {})",
                SERVICES_ROOT_ENV, lane.home_env
            ),
        };
    };
    let home = root.join(lane.name);
    if looks_like_service_home(&home) {
        LaneHome::Resolved { home, source: HomeSource::Bundled }
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
pub fn services_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os(SERVICES_ROOT_ENV) {
        return Some(PathBuf::from(root));
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..ROOT_WALK_DEPTH {
            let Some(candidate) = dir else { break };
            if candidate.join("services").is_dir() && candidate.join("Cargo.toml").is_file() {
                return Some(candidate.join("services"));
            }
            dir = candidate.parent().map(Path::to_path_buf);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("services");
    manifest.is_dir().then_some(manifest)
}

/// The launch spec for one lane, or the reason it cannot launch.
pub fn launch_spec(lane: &Lane, home: &Path) -> ServerSpec {
    ServerSpec {
        program: "python3".to_string(),
        args: vec![
            "-u".to_string(),
            home.join("service").join("mcp_server.py").to_string_lossy().into_owned(),
        ],
        env: vec![(
            lane.python_path_env.to_string(),
            home.join("src").to_string_lossy().into_owned(),
        )],
        cwd: Some(home.to_path_buf()),
    }
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

/// Probe every lane in order, tearing each child down before the next.
/// Sequential on purpose: four small children under a loaded machine is
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
                detail: reason,
                elapsed_ms: started.elapsed().as_millis(),
            };
        }
    };
    let (home, source) = home.expect("a resolved home");
    let spec = launch_spec(lane, &home);
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
                detail: error.to_string(),
                elapsed_ms: started.elapsed().as_millis(),
            };
        }
    };
    // Every exit path below tears the child down first, so a failed probe
    // never leaks a Python process.
    let outcome = probe_handshake(&mut server).await;
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
            detail: error.to_string(),
            elapsed_ms: started.elapsed().as_millis(),
        },
    }
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
/// never a plain-text table.
pub fn render(health: &[LaneHealth]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD}fleet lanes{RESET} {DIM}— the four house lanes, probed live{RESET}\n\n"
    ));
    for lane in health {
        out.push_str(&render_card(lane));
    }
    let ok = health.iter().filter(|lane| lane.state == LaneState::Ok).count();
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
            format!("{} · v{} · {} tool{}", lane.role, version, count, plural(count))
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
    if let Some(source) = &lane.source {
        lines.push(format!(
            "{DIM}│{RESET} {DIM}home: {source} · {} ms{RESET}",
            lane.elapsed_ms
        ));
    }
    let bottom = format!("{DIM}└{bottom}┘{RESET}", bottom = "─".repeat(CARD_WIDTH - 2));
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
fn wrap_detail(text: &str, width: usize) -> Vec<String> {
    let width = width.max(16);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        current.push(character);
        if current.chars().count() >= width {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() || chunks.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(name: &'static str) -> Lane {
        *LANES.iter().find(|lane| lane.name == name).expect("a registry lane")
    }

    #[test]
    fn the_registry_names_the_four_lanes_in_launch_order() {
        let names: Vec<&str> = LANES.iter().map(|lane| lane.name).collect();
        assert_eq!(names, vec!["sys1", "sdm1", "sem1", "gen1"]);
        // Each lane's env names follow the fork's convention.
        assert_eq!(lane("sem1").python_path_env, "SEM1_PYTHONPATH");
        assert_eq!(lane("sem1").home_env, "FLEET_LANE_SEM1_HOME");
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
    fn the_launch_spec_replicates_the_forks_launcher() {
        let home = PathBuf::from("/opt/services/sem1");
        let spec = launch_spec(&lane("sem1"), &home);
        assert_eq!(spec.program, "python3");
        assert_eq!(spec.args, vec!["-u".to_string(), "/opt/services/sem1/service/mcp_server.py".to_string()]);
        assert_eq!(spec.env, vec![(
            "SEM1_PYTHONPATH".to_string(),
            "/opt/services/sem1/src".to_string()
        )]);
        assert_eq!(spec.cwd, Some(home.clone()));
        assert_eq!(
            launch_command(&lane("sem1"), &home),
            "SEM1_PYTHONPATH=/opt/services/sem1/src python3 -u /opt/services/sem1/service/mcp_server.py"
        );
    }

    #[test]
    fn a_lane_without_a_home_reports_a_named_reason() {
        // No env override is set for this lane in the test process, so the
        // resolution falls through to the bundled root or names its absence.
        let home = resolve_home(&lane("gen1"));
        match home {
            LaneHome::Resolved { home, .. } => {
                assert!(looks_like_service_home(&home));
            }
            LaneHome::Unavailable { reason } => {
                assert!(reason.contains("FLEET_LANE_GEN1_HOME") || reason.contains("services root"));
            }
        }
    }

    #[test]
    fn the_services_root_walk_finds_a_repo_root_from_a_nested_exe() {
        // The walk starts at the executable: in tests that is
        // target/debug/deps/..., and the repo root carries both services/
        // and Cargo.toml. This asserts the discovery mechanism, not the
        // machine's install.
        let root = services_root();
        if let Some(root) = root {
            assert!(root.join("sem1").join("VENDOR.json").is_file(), "{root:?}");
        }
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
                detail: "no services root found (set FLEET_SERVICES_ROOT or FLEET_LANE_GEN1_HOME)".to_string(),
                elapsed_ms: 0,
            },
        ];
        let rendered = render(&health);
        // Cards: box-drawing borders, one per lane.
        assert_eq!(rendered.matches('┌').count(), 2, "{rendered}");
        assert_eq!(rendered.matches('└').count(), 2, "{rendered}");
        // Status dots, colored by state.
        assert!(rendered.contains(&format!("{}✓ ok{RESET}", palette::OK)), "{rendered}");
        assert!(rendered.contains(&format!("{}○ unavailable{RESET}", palette::DIM)), "{rendered}");
        // The facts a health command owes: role, version, tool count, names.
        assert!(rendered.contains("embeddings · v0.1.0 · 2 tools"), "{rendered}");
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
            detail: "`tools/list` timed out after 10s".to_string(),
            elapsed_ms: 10_000,
        }];
        let rendered = render(&health);
        assert!(rendered.contains(&format!("{}✕ error{RESET}", palette::ERROR)), "{rendered}");
        assert!(rendered.contains("timed out"), "{rendered}");
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
