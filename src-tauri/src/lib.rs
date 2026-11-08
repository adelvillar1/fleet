//! Fleet's native shell — the face in its own window.
//!
//! The shell is deliberately thin (hard rule 12, the owner's 2026-11-02
//! ruling): it starts the *same* server `fleet gui` starts, on a loopback
//! port, and opens a window on that origin. The frontend is unchanged,
//! the remote face is untouched, and the PWA's service worker keeps
//! working because the window talks to a real `http://127.0.0.1` origin
//! rather than Tauri's custom scheme.
//!
//! The work this crate actually does is the **self-containment** the PWA
//! era left to the owner's shell (the native shell plan's component
//! inventory):
//!
//! - the services root — a bundled app has no `Cargo.toml` beside
//!   `services/`, so the walk in `fleet::lanes` cannot find it and the
//!   compile-time fallback is a build machine's path. The shell sets
//!   `FLEET_SERVICES_ROOT` to the bundled copy.
//! - the lane wiring — `fleet install` *prints* `export SEM1_ST_VENV=…`
//!   and a sys1 `config.toml` note for the owner to put in a shell
//!   profile. A dock-launched app sources no profile, so the shell sets
//!   the variables in its own environment and writes sys1's key.
//! - the first run — the home layout, the config scaffold, and the venvs,
//!   driven by the app rather than by a terminal the owner must open.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fleet::install::{self, EnvTarget};
use fleet::lanes::SERVICES_ROOT_ENV;

/// The bundled services directory's name inside the app bundle.
const BUNDLED_SERVICES: &str = "services";

/// How long the shell waits for the face to answer before naming it.
const FACE_TIMEOUT: Duration = Duration::from_secs(30);

// --- the component inventory -------------------------------------------------

/// The bundled services root for an app at `executable`, when there is
/// one.
///
/// A bundle keeps its resources at `Contents/Resources`, so the
/// executable at `Contents/MacOS/fleet-desktop` finds them two levels up.
/// A dev build has no bundle and returns `None`, leaving the `fleet`
/// crate's own resolution (the checkout walk) to find `services/`.
pub fn bundled_services_root_at(executable: &Path) -> Option<PathBuf> {
    let contents = executable.parent()?.parent()?;
    let candidate = contents.join("Resources").join(BUNDLED_SERVICES);
    candidate.is_dir().then_some(candidate)
}

/// The services root this process should run with.
pub fn bundled_services_root() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    bundled_services_root_at(&executable)
}

/// The services root the shell runs with: the bundled copy when there is
/// one. `None` means "leave the environment alone" — a dev build resolves
/// the checkout itself, and an owner's explicit `FLEET_SERVICES_ROOT`
/// still wins over a missing bundle.
pub fn services_root() -> Option<PathBuf> {
    bundled_services_root()
}

/// One lane's wiring: the environment variable the core reads, and the
/// value the shell gives it.
///
/// `fleet install` prints these for the owner's shell profile; a
/// dock-launched app has none, so the shell sets them in its own
/// environment before the bridge spawns any lane.
pub fn lane_environment(home: &Path) -> Vec<(String, String)> {
    install::LANES
        .iter()
        .filter_map(|lane| {
            let name = lane.env_var?;
            let venv = install::venv_dir(home, lane);
            let value = match lane.env_target {
                // The core appends `bin/python` itself.
                EnvTarget::Root => venv,
                // This name addresses the python binary directly.
                EnvTarget::Python => venv.join("bin").join("python"),
            };
            Some((name.to_string(), value.to_string_lossy().into_owned()))
        })
        .collect()
}

/// sys1's config file for a user's home — the lane that reads a config
/// key rather than an environment variable.
pub fn sys1_config_path_for(user_home: &Path) -> PathBuf {
    user_home.join(".config/sys1/config.toml")
}

/// sys1's config file for this process's `HOME`.
pub fn sys1_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| sys1_config_path_for(Path::new(&home)))
}

/// The `[providers]` table sys1's local provider reads.
pub fn sys1_config_toml(venv: &Path) -> String {
    format!(
        "[providers]\nlocal_venv = \"{}\"\n",
        venv.to_string_lossy().replace('\\', "\\\\")
    )
}

/// sys1's venv: the install table's own sys1 lane.
pub fn sys1_venv(home: &Path) -> PathBuf {
    let sys1 = install::LANES
        .iter()
        .find(|lane| lane.name == "sys1")
        .expect("sys1 is the install table's first lane");
    install::venv_dir(home, sys1)
}

/// Write sys1's config key when it is absent.
///
/// An existing file is left byte-for-byte untouched — the install's own
/// discipline, and the reason a hand-edited config is never clobbered.
/// Returns whether the file was written.
pub fn write_sys1_config(home: &Path, user_home: &Path) -> std::io::Result<bool> {
    let path = sys1_config_path_for(user_home);
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, sys1_config_toml(&sys1_venv(home)))?;
    Ok(true)
}

/// What the first run did — the report the shell prints and the test
/// asserts against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRun {
    /// The layout directories that already existed.
    pub present: Vec<&'static str>,
    /// The layout directories the run created.
    pub created: Vec<&'static str>,
    /// Whether the config scaffold was written (it is never overwritten).
    pub config_written: bool,
    /// The lane variables the shell set.
    pub wiring: Vec<(String, String)>,
    /// Whether sys1's config key was written.
    pub sys1_written: bool,
    /// The bundled services root, when the app is packaged.
    pub services_root: Option<PathBuf>,
}

/// The first-run install: the home layout, the config scaffold, and the
/// lane wiring. Fast and synchronous — the venvs are the slow half and
/// are spawned separately ([`build_lanes`]).
pub fn first_run(home: &Path, user_home: &Path) -> std::io::Result<FirstRun> {
    let mut present = Vec::new();
    let mut created = Vec::new();
    for dir in install::LAYOUT_DIRS {
        let path = home.join(dir);
        if path.is_dir() {
            present.push(*dir);
        } else {
            std::fs::create_dir_all(&path)?;
            created.push(*dir);
        }
    }

    let config = home.join(install::CONFIG_FILE);
    let config_written = !config.exists();
    if config_written {
        if let Some(parent) = config.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config, install::CONFIG_SCAFFOLD)?;
    }

    let wiring = lane_environment(home);
    let sys1_written = write_sys1_config(home, user_home).unwrap_or(false);

    Ok(FirstRun {
        present,
        created,
        config_written,
        wiring,
        sys1_written,
        services_root: services_root(),
    })
}

/// Build the lane venvs, naming each outcome. The install's own
/// degrade-without discipline: a lane that cannot build is skipped by
/// name, never a failure of the run.
///
/// `path` is the environment the install would inherit — passed in so a
/// test can drive the skip without mutating the process environment.
pub fn build_lanes(
    home: &Path,
    path: Option<&std::ffi::OsStr>,
    progress: &mut (dyn Write + Send),
) -> Vec<(&'static install::Lane, install::LaneOutcome)> {
    let mut out = Vec::new();
    for lane in install::LANES {
        let outcome = install::bootstrap_lane(home, lane, path, progress);
        out.push((lane, outcome));
    }
    out
}

/// Build the lanes with this process's `PATH` — the shell's call.
pub fn build_lanes_with_env(
    home: &Path,
    progress: &mut (dyn Write + Send),
) -> Vec<(&'static install::Lane, install::LaneOutcome)> {
    let path = std::env::var_os("PATH");
    build_lanes(home, path.as_deref(), progress)
}

// --- the shell ---------------------------------------------------------------

/// The user's home the shell resolves (`$HOME`), or `.` when it is unset.
pub fn user_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Run the shell: the inventory, the first run, the window, the server.
pub fn run() {
    // The services root, before anything resolves a lane.
    if let Some(root) = services_root() {
        std::env::set_var(SERVICES_ROOT_ENV, &root);
    }

    let home = fleet::home::fleet_home(
        std::env::var("FLEET_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
    .unwrap_or_else(|_| PathBuf::from("."));
    let owner_home = user_home();

    match first_run(&home, &owner_home) {
        Ok(report) => {
            println!("fleet desktop — the first run");
            for dir in &report.created {
                println!("  + {dir}  created");
            }
            for dir in &report.present {
                println!("  = {dir}  present");
            }
            if report.config_written {
                println!("  + config.json  written");
            }
            if report.sys1_written {
                println!("  + sys1 config.toml  written");
            }
            for (name, value) in &report.wiring {
                println!("  = {name}  set ({value})");
            }
            if let Some(root) = &report.services_root {
                println!("  = services  bundled at {}", root.display());
            }
        }
        Err(error) => eprintln!("fleet desktop: the first run failed: {error}"),
    }

    // The lane wiring, before the bridge spawns any lane.
    for (name, value) in lane_environment(&home) {
        std::env::set_var(&name, &value);
    }

    // The venvs, in the background: the torch stack is minutes and the
    // window must not wait for it. The receipt discipline makes a second
    // launch read Present without rebuilding, and a lane that is not
    // ready is dead by name — the install's degrade-without behavior,
    // never a silent gap.
    {
        let lane_home = home.clone();
        std::thread::spawn(move || {
            let mut progress = std::io::stderr();
            for (lane, outcome) in build_lanes_with_env(&lane_home, &mut progress) {
                match outcome {
                    install::LaneOutcome::Present | install::LaneOutcome::Installed => {}
                    install::LaneOutcome::Skipped { reason } => {
                        eprintln!(
                            "fleet desktop: the {} lane is skipped — {reason}",
                            lane.name
                        );
                    }
                }
            }
        });
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![restart_app])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match start_face().await {
                    Ok(url) => {
                        if let Ok(parsed) = url.parse::<url::Url>() {
                            match tauri::WebviewWindowBuilder::new(
                                &handle,
                                "main",
                                tauri::WebviewUrl::External(parsed),
                            )
                            .title("fleet")
                            .inner_size(1440.0, 900.0)
                            .build()
                            {
                                Ok(_) => {}
                                Err(error) => {
                                    eprintln!("fleet desktop: the window could not open: {error}")
                                }
                            }
                        }
                    }
                    Err(error) => eprintln!("fleet desktop: {error}"),
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("fleet desktop: the application could not start");
}

/// Start the face on an ephemeral loopback port and return its URL.
///
/// The port is chosen here rather than left to the server so the window
/// has an origin to load: `fleet gui` computes its own base URL from the
/// port it was *asked* for, so an ephemeral port has to be chosen first.
/// `gui::serve` resolves the home itself, exactly as it does for
/// `fleet gui`.
/// The server options the shell runs with.
///
/// `no_open` is the point: the shell's window is the client, and a
/// browser tab opening beside it would be a second, unrelated surface.
pub fn gui_options(port: u16) -> fleet::gui::GuiOptions {
    fleet::gui::GuiOptions {
        host: fleet::gui::DEFAULT_HOST.to_string(),
        port,
        no_open: true,
        remote: Default::default(),
        channels: false,
        relay: None,
    }
}

/// The origin the window loads.
///
/// A real `http://127.0.0.1` origin is what makes the PWA's service
/// worker register: Tauri's custom scheme is rejected by WebKit's
/// service-worker container on macOS (Tauri #13031).
pub fn face_url(port: u16) -> String {
    format!("http://{}:{port}", fleet::gui::DEFAULT_HOST)
}

/// The apply button's last step: the updater swapped the bundle under
/// the running process, so the process itself exits and relaunches —
/// the new `Contents/MacOS/fleet-desktop` serves the same loopback
/// origin the face already had. `AppHandle::restart` walks the exit
/// events and re-execs the bundle's own binary.
#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

async fn start_face() -> Result<String, String> {
    let host = fleet::gui::DEFAULT_HOST;
    let port = ephemeral_port(host).await?;
    let options = gui_options(port);
    // The face runs for the life of the process; the window closing ends
    // it (Tauri exits when the last window closes).
    tokio::spawn(async move {
        if let Err(error) = fleet::gui::serve(options).await {
            eprintln!("fleet desktop: the face stopped: {error}");
        }
    });
    let url = face_url(port);
    wait_for(host, port, FACE_TIMEOUT).await?;
    Ok(url)
}

/// Bind an ephemeral port and release it, so `gui::serve` can bind it.
async fn ephemeral_port(host: &str) -> Result<u16, String> {
    let listener = tokio::net::TcpListener::bind((host, 0))
        .await
        .map_err(|error| format!("no loopback port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("no loopback port: {error}"))
}

/// Wait until the face's TCP port answers, so the window never opens on a
/// connection refused.
async fn wait_for(host: &str, port: u16, bound: Duration) -> Result<(), String> {
    let deadline = Instant::now() + bound;
    loop {
        let address = format!("{host}:{port}");
        if std::net::TcpStream::connect(address.as_str()).is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("the face did not answer at {address}"));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-desktop-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    #[test]
    fn a_bundled_app_finds_its_services_two_levels_up() {
        // The layout a bundle has: Contents/MacOS/<exe> and
        // Contents/Resources/services.
        let root = scratch("bundle");
        let macos = root.join("Fleet.app/Contents/MacOS");
        let resources = root.join("Fleet.app/Contents/Resources");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir_all(resources.join("services")).unwrap();
        let executable = macos.join("fleet-desktop");
        std::fs::write(&executable, b"").unwrap();

        assert_eq!(
            bundled_services_root_at(&executable),
            Some(resources.join("services"))
        );
        // A dev build's executable has no Resources/services beside it.
        let dev = root.join("target/release/fleet-desktop");
        std::fs::create_dir_all(dev.parent().unwrap()).unwrap();
        std::fs::write(&dev, b"").unwrap();
        assert_eq!(bundled_services_root_at(&dev), None);
    }

    #[test]
    fn the_lane_wiring_addresses_each_lane_the_way_its_core_reads_it() {
        let home = scratch("wiring");
        let wiring = lane_environment(&home);
        let find = |name: &str| {
            wiring
                .iter()
                .find(|(one, _)| one == name)
                .map(|(_, value)| value.clone())
        };
        // A venv root: the core appends bin/python.
        assert_eq!(
            find("SEM1_ST_VENV"),
            Some(home.join("venvs/sem1").to_string_lossy().into_owned())
        );
        assert_eq!(
            find("SDM1_MODELS_VENV"),
            Some(home.join("venvs/sdm1").to_string_lossy().into_owned())
        );
        // A python binary: the value is the binary itself.
        assert_eq!(
            find("GEN1_KOKORO_PYTHON"),
            Some(
                home.join("venvs/gen1/bin/python")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        // blend1 and browser read no environment name, and sys1 reads a
        // config key instead — none of the three appear here.
        assert!(find("BLEND1_PYTHONPATH").is_none());
        assert_eq!(wiring.len(), 3, "{wiring:?}");
    }

    #[test]
    fn sys1_config_carries_the_lane_key_and_never_overwrites() {
        let home = scratch("sys1-home");
        let owner = scratch("sys1-owner");
        let path = sys1_config_path_for(&owner);

        assert!(!path.exists());
        assert!(write_sys1_config(&home, &owner).unwrap());
        let written = std::fs::read_to_string(&path).unwrap();
        let venv = home.join("venvs/sys1");
        assert!(
            written.contains(&format!("local_venv = \"{}\"", venv.display())),
            "{written}"
        );

        // A second run leaves the file byte-for-byte alone.
        std::fs::write(&path, "hand edited\n").unwrap();
        assert!(!write_sys1_config(&home, &owner).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hand edited\n");
    }

    #[test]
    fn the_first_run_builds_the_layout_and_the_scaffold_once() {
        let home = scratch("first-home");
        let owner = scratch("first-owner");

        let first = first_run(&home, &owner).expect("the first run");
        assert_eq!(
            first.created,
            install::LAYOUT_DIRS.to_vec(),
            "every layout dir is created"
        );
        assert!(first.config_written, "the scaffold is written");
        for dir in install::LAYOUT_DIRS {
            assert!(home.join(dir).is_dir(), "{dir}");
        }
        let scaffold = std::fs::read_to_string(home.join(install::CONFIG_FILE)).unwrap();
        assert_eq!(scaffold, install::CONFIG_SCAFFOLD);
        assert!(first.sys1_written, "sys1's key is written");
        assert_eq!(first.wiring.len(), 3);

        // The second run recreates nothing and leaves the scaffold alone.
        std::fs::write(home.join(install::CONFIG_FILE), "{\n  \"pin\": \"x\"\n}\n").unwrap();
        let second = first_run(&home, &owner).expect("the second run");
        assert!(second.created.is_empty(), "nothing is recreated");
        assert_eq!(second.present, install::LAYOUT_DIRS.to_vec());
        assert!(!second.config_written, "the scaffold is never overwritten");
        assert!(!second.sys1_written, "sys1's key is never rewritten");
        assert_eq!(
            std::fs::read_to_string(home.join(install::CONFIG_FILE)).unwrap(),
            "{\n  \"pin\": \"x\"\n}\n"
        );
    }

    #[test]
    fn the_shell_never_opens_a_browser_and_loads_a_real_http_origin() {
        // C2: the window is the only client. A browser tab opening
        // beside it would be a second, unrelated surface.
        let options = gui_options(59385);
        assert!(options.no_open, "the shell must leave the browser alone");
        assert_eq!(options.host, fleet::gui::DEFAULT_HOST);
        assert!(
            !options.remote.enabled,
            "the shell is a local client; the remote face is a separate mode"
        );

        // C3: the service worker's condition is a real loopback HTTP
        // origin. Tauri's custom scheme would be rejected on macOS.
        let url = face_url(59385);
        assert_eq!(url, "http://127.0.0.1:59385");
        assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    }

    #[test]
    fn a_lane_that_cannot_build_is_skipped_by_name_not_failed() {
        // A PATH with no venv tool skips every lane and still returns a
        // result per lane — the install's degrade-without discipline.
        let home = scratch("lanes");
        let mut progress = Vec::new();
        let results = build_lanes(&home, Some(std::ffi::OsStr::new("")), &mut progress);
        assert_eq!(results.len(), install::LANES.len());
        assert!(
            results
                .iter()
                .all(|(_, outcome)| matches!(outcome, install::LaneOutcome::Skipped { .. })),
            "{results:?}"
        );
    }
}
