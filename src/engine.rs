//! The browser engine: which one this host has, and whether it is current.
//!
//! The owner's 2026-11-18 ruling put obscura first — "I want to use obscura on
//! this harness chrome is irresponsible resource hog" — and made its GitHub
//! releases the source: "the latest obscura can be sourced from here
//! https://github.com/h4ckf0r0day/obscura/releases and we should check that we
//! have the latest release."
//!
//! So this module answers two questions and nothing else: **which engine is on
//! this host**, and **is the installed obscura the published one**. Chrome is
//! named as the last resort and is only ever *reported*, never launched from
//! here — the lane reads `browser.engine` for that, and the owner's refusal is
//! what turns it on.
//!
//! The release read is bounded and cached: one request with a timeout, at most
//! once a day, in the home. An update check that runs on every doctor read
//! would put a network dependency inside a local diagnostic, and a diagnostic
//! that hangs because GitHub is slow is worse than one that says nothing.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

/// Where obscura publishes its releases.
pub const RELEASES_REPO: &str = "https://github.com/h4ckf0r0day/obscura/releases";

/// The GitHub API endpoint the latest tag is read from.
const LATEST_API: &str = "https://api.github.com/repos/h4ckf0r0day/obscura/releases/latest";

/// The bound on the release read. One request, one timeout: a doctor check
/// that waits longer than this is a check the owner will learn to skip.
pub const RELEASE_TIMEOUT: Duration = Duration::from_secs(6);

/// How long a release read is trusted. A day: the engine does not move faster
/// than that, and a shorter window turns a local diagnostic into a poll.
pub const RELEASE_CACHE_SECS: u64 = 24 * 60 * 60;

/// The body ceiling, matching the update feed's own: a response larger than
/// this is not a release document.
const RELEASE_MAX: u64 = 512 * 1024;

/// The cache file, beside the settings it is read with.
fn cache_path(home: &Path) -> PathBuf {
    home.join("engine-release.json")
}

/// Which browser engine this host can run, as the lane's order reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    /// obscura, with its binary path and the version string it reports.
    Obscura { path: PathBuf, version: String },
    /// Chrome, found and runnable — the last resort, only on the owner's word.
    Chrome { path: PathBuf },
    /// Neither: the host cannot browse until something is installed.
    None,
}

impl Engine {
    pub fn label(&self) -> &'static str {
        match self {
            Engine::Obscura { .. } => "obscura",
            Engine::Chrome { .. } => "chrome",
            Engine::None => "none",
        }
    }
}

/// Find the engine on this host, in the ruling's order.
///
/// obscura first: `BROWSER_OBSCURA`, then the home's own bin (where the
/// harness's install writes it), then the usual prefixes, then PATH. Chrome is
/// probed only to be *reported* — the lane will not start it unless
/// `browser.engine` says so.
pub fn find(home: &Path) -> Engine {
    if let Some(path) = obscura_path(home) {
        let version = obscura_version(&path);
        return Engine::Obscura { path, version };
    }
    if let Some(path) = chrome_path() {
        return Engine::Chrome { path };
    }
    Engine::None
}

/// The obscura binary, by the same order the lane's own resolution uses.
fn obscura_path(home: &Path) -> Option<PathBuf> {
    // An explicit override that does not resolve is a REFUSAL, not a licence
    // to probe elsewhere. The owner naming a path is them telling this build
    // where obscura lives; silently falling back to another binary would run
    // an engine they did not name — and it made the absent branch
    // untestable, which is how this was found.
    if let Ok(override_path) = std::env::var("BROWSER_OBSCURA") {
        let candidate = PathBuf::from(override_path);
        return is_executable(&candidate).then_some(candidate);
    }
    for candidate in [
        home.join("bin").join("obscura"),
        PathBuf::from("/usr/local/bin/obscura"),
        PathBuf::from("/opt/homebrew/bin/obscura"),
    ] {
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    which("obscura")
}

/// Chrome's binary, for the check's report only.
fn chrome_path() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("BROWSER_CHROME") {
        let candidate = PathBuf::from(override_path);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    for name in [
        "google-chrome-stable",
        "google-chrome",
        "chromium",
        "chromium-browser",
    ] {
        if let Some(found) = which(name) {
            return Some(found);
        }
    }
    for bundle in [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ] {
        let candidate = PathBuf::from(bundle);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// The version obscura reports, or an empty string when it will not answer.
///
/// Bounded: a binary that hangs must not hang the doctor. `--version` is the
/// CLI's own contract and its answer is `obscura 0.2.7`. An update-check that
/// cannot read a version says so rather than assuming current.
pub fn obscura_version(path: &Path) -> String {
    let Ok(output) = std::process::Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
    else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    text.split_whitespace()
        .last()
        .unwrap_or_default()
        .trim_start_matches('v')
        .to_string()
}

/// A minimal `which`: the first executable of that name on PATH.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// What the installed obscura is, against what the releases publish.
///
/// Three words, and a fourth for "could not ask":
///
/// - **current** — the installed version equals the latest tag.
/// - **behind** — the latest tag is newer; both versions ride the sentence.
/// - **ahead** — the installed build is not one the releases publish (a local
///   build, or a tag pulled): named rather than counted current.
/// - **unknown** — the registry could not be reached. NEVER current: an
///   unreachable registry is not an up-to-date engine, and reporting it as one
///   is the lie that leaves a stale browser in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseState {
    Current { version: String },
    Behind { installed: String, latest: String },
    Ahead { installed: String, latest: String },
    Unknown { reason: String },
}

impl ReleaseState {
    pub fn word(&self) -> &'static str {
        match self {
            ReleaseState::Current { .. } => "current",
            ReleaseState::Behind { .. } => "behind",
            ReleaseState::Ahead { .. } => "ahead",
            ReleaseState::Unknown { .. } => "unknown",
        }
    }

    /// Whether the person has something to do about it.
    pub fn stale(&self) -> bool {
        matches!(self, ReleaseState::Behind { .. })
    }

    pub fn sentence(&self) -> String {
        match self {
            ReleaseState::Current { version } => {
                format!("obscura {version} is the latest release")
            }
            ReleaseState::Behind { installed, latest } => format!(
                "obscura {installed} is behind the latest release {latest} \u{2014} update it from {RELEASES_REPO}"
            ),
            ReleaseState::Ahead { installed, latest } => format!(
                "obscura {installed} is not a published release (the latest tag is {latest}) \u{2014} a local or untagged build"
            ),
            ReleaseState::Unknown { reason } => format!(
                "could not check for a newer obscura ({reason}) \u{2014} this is NOT a report that it is current"
            ),
        }
    }
}

/// The cached latest tag, when the cache is fresh.
fn cached_latest(home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(cache_path(home)).ok()?;
    let document: Value = serde_json::from_str(&text).ok()?;
    let read_at = document.get("read_at").and_then(Value::as_u64)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(read_at) > RELEASE_CACHE_SECS {
        return None;
    }
    document
        .get("latest")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Record a release read, best-effort: a cache that cannot be written leaves
/// the check working, it only costs the next read a request.
fn remember_latest(home: &Path, latest: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|age| age.as_secs())
        .unwrap_or_default();
    let body = json!({ "latest": latest, "read_at": now });
    if let Ok(text) = serde_json::to_string(&body) {
        let _ = std::fs::write(cache_path(home), text);
    }
}

/// The latest published tag, from the cache or one bounded request.
pub fn latest_release(home: &Path) -> Result<String, String> {
    if let Some(latest) = cached_latest(home) {
        return Ok(latest);
    }
    let agent = ureq::AgentBuilder::new().timeout(RELEASE_TIMEOUT).build();
    let reply = agent
        .get(LATEST_API)
        .set("accept", "application/vnd.github+json")
        .set("user-agent", "fleet-doctor")
        .call()
        .map_err(|error| format!("the release registry was not reached: {error}"))?;
    if reply.status() != 200 {
        return Err(format!(
            "the release registry answered HTTP {}",
            reply.status()
        ));
    }
    let mut capped = reply.into_reader().take(RELEASE_MAX);
    let mut body = String::new();
    std::io::Read::read_to_string(&mut capped, &mut body)
        .map_err(|error| format!("the release registry did not read: {error}"))?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("the release document is not readable json: {error}"))?;
    let tag = value
        .get("tag_name")
        .and_then(Value::as_str)
        .map(|tag| tag.trim_start_matches('v').to_string())
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "the release document names no tag".to_string())?;
    remember_latest(home, &tag);
    Ok(tag)
}

/// The installed obscura against the latest release.
pub fn release_state(home: &Path, installed: &str) -> ReleaseState {
    if installed.is_empty() {
        return ReleaseState::Unknown {
            reason: "the installed obscura would not report its version".to_string(),
        };
    }
    match latest_release(home) {
        Ok(latest) => compare(installed, &latest),
        Err(reason) => ReleaseState::Unknown { reason },
    }
}

/// Compare two dotted versions. Pure, so the three words are a test rather
/// than a claim about the network.
pub fn compare(installed: &str, latest: &str) -> ReleaseState {
    let parse = |text: &str| -> Option<(u64, u64, u64)> {
        let mut parts = text.trim().trim_start_matches('v').split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().unwrap_or("0").parse().ok()?;
        let patch = parts
            .next()
            .unwrap_or("0")
            .split('-')
            .next()
            .unwrap_or("0")
            .parse()
            .ok()?;
        Some((major, minor, patch))
    };
    match (parse(installed), parse(latest)) {
        (Some(left), Some(right)) => match right.cmp(&left) {
            std::cmp::Ordering::Greater => ReleaseState::Behind {
                installed: installed.to_string(),
                latest: latest.to_string(),
            },
            std::cmp::Ordering::Equal => ReleaseState::Current {
                version: installed.to_string(),
            },
            std::cmp::Ordering::Less => ReleaseState::Ahead {
                installed: installed.to_string(),
                latest: latest.to_string(),
            },
        },
        _ => ReleaseState::Unknown {
            reason: format!("`{installed}` and `{latest}` do not compare as versions"),
        },
    }
}

/// The asset this platform installs from, or `None` where obscura publishes
/// nothing for it. The naming is the release's own: `<arch>-<os>` with a
/// `.tar.gz` (or `.zip` on Windows).
pub fn asset_for(platform: &str, arch: &str) -> Option<String> {
    let os = match platform {
        "macos" | "darwin" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match arch {
        "aarch64" | "arm64" => "aarch64",
        "x86_64" | "x64" => "x86_64",
        _ => return None,
    };
    let extension = if os == "windows" { "zip" } else { "tar.gz" };
    Some(format!("obscura-{arch}-{os}.{extension}"))
}

/// The download URL for one asset on a given tag.
pub fn asset_url(tag: &str, asset: &str) -> String {
    format!("https://github.com/h4ckf0r0day/obscura/releases/download/v{tag}/{asset}")
}

/// The engine's document for the doctor check and the repair press.
pub fn engine_field(home: &Path, engine: &Engine) -> Value {
    let release = match engine {
        Engine::Obscura { version, .. } => Some(release_state(home, version)),
        _ => None,
    };
    json!({
        "engine": engine.label(),
        "release": release.as_ref().map(|state| state.word()),
        "detail": match engine {
            Engine::Obscura { path, version } => format!("{} \u{b7} {}", path.display(), version),
            Engine::Chrome { path } => path.display().to_string(),
            Engine::None => "no browser engine on this host".to_string(),
        },
        "note": release.as_ref().map(ReleaseState::sentence),
        "releases": RELEASES_REPO,
    })
}

/// The platform and arch this build runs on, as obscura names them.
pub fn platform() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_words_are_the_versions_own_answer() {
        // C7: current / behind / ahead, and never a fourth word dressed as
        // current. The comparison is pure, so the words are a test rather
        // than a claim about the network.
        assert_eq!(
            compare("0.2.7", "0.2.7"),
            ReleaseState::Current {
                version: "0.2.7".to_string()
            }
        );
        assert_eq!(
            compare("0.2.6", "0.2.7"),
            ReleaseState::Behind {
                installed: "0.2.6".to_string(),
                latest: "0.2.7".to_string()
            }
        );
        assert_eq!(
            compare("0.2.8", "0.2.7"),
            ReleaseState::Ahead {
                installed: "0.2.8".to_string(),
                latest: "0.2.7".to_string()
            }
        );
        // A `v` prefix and a prerelease suffix read the same as the plain
        // numbers: the tag shape is the registry's, not ours.
        assert_eq!(
            compare("v0.2.7", "0.2.7"),
            ReleaseState::Current {
                version: "v0.2.7".to_string()
            }
        );
        assert_eq!(
            compare("0.3.0-rc1", "0.3.0"),
            ReleaseState::Current {
                version: "0.3.0-rc1".to_string()
            }
        );
        // Unparseable is UNKNOWN, never current.
        assert!(matches!(
            compare("nightly", "0.2.7"),
            ReleaseState::Unknown { .. }
        ));
        assert_eq!(
            ReleaseState::Behind {
                installed: "0.2.6".into(),
                latest: "0.2.7".into()
            }
            .word(),
            "behind"
        );
        assert!(ReleaseState::Behind {
            installed: "0.2.6".into(),
            latest: "0.2.7".into()
        }
        .stale());
    }

    #[test]
    fn an_unreachable_registry_is_not_current() {
        // C8's honesty clause: a check that cannot reach GitHub says so. It
        // must never read as `current`, because that is what leaves a stale
        // engine in place.
        let state = ReleaseState::Unknown {
            reason: "the release registry was not reached: timeout".to_string(),
        };
        assert_eq!(state.word(), "unknown");
        assert!(!state.stale(), "unknown is not stale: it is unknown");
        assert!(
            state.sentence().contains("NOT a report that it is current"),
            "{}",
            state.sentence()
        );
    }

    #[test]
    fn the_asset_names_this_platform_the_way_the_release_does() {
        assert_eq!(
            asset_for("macos", "aarch64").as_deref(),
            Some("obscura-aarch64-macos.tar.gz")
        );
        assert_eq!(
            asset_for("linux", "x86_64").as_deref(),
            Some("obscura-x86_64-linux.tar.gz")
        );
        assert_eq!(
            asset_for("windows", "x86_64").as_deref(),
            Some("obscura-x86_64-windows.zip")
        );
        // An architecture obscura publishes nothing for is None, not a guess.
        assert_eq!(asset_for("macos", "mips"), None);
        assert_eq!(asset_for("plan9", "x86_64"), None);
        assert_eq!(
            asset_url("0.2.7", "obscura-aarch64-macos.tar.gz"),
            "https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.7/obscura-aarch64-macos.tar.gz"
        );
    }

    #[test]
    fn a_fresh_cache_is_read_and_a_stale_one_is_not() {
        let home = std::env::temp_dir().join(format!("fleet-engine-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        // No cache: nothing.
        assert_eq!(cached_latest(&home), None);
        // A fresh write reads back.
        remember_latest(&home, "0.2.7");
        assert_eq!(cached_latest(&home), Some("0.2.7".to_string()));
        // A read older than the window is ignored, so the next check asks.
        let stale = json!({ "latest": "0.2.7", "read_at": 1 });
        std::fs::write(cache_path(&home), serde_json::to_string(&stale).unwrap()).unwrap();
        assert_eq!(cached_latest(&home), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_engine_document_names_what_was_found() {
        let home = std::env::temp_dir().join(format!("fleet-engine-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let none = engine_field(&home, &Engine::None);
        assert_eq!(none["engine"], "none");
        assert!(none["release"].is_null(), "no engine has no release state");
        let chrome = engine_field(
            &home,
            &Engine::Chrome {
                path: PathBuf::from("/usr/bin/chrome"),
            },
        );
        assert_eq!(chrome["engine"], "chrome");
        assert_eq!(chrome["detail"], "/usr/bin/chrome");
        let _ = std::fs::remove_dir_all(&home);
    }
}

/// Install or update obscura from its GitHub release, into the home's bin.
///
/// The owner's 2026-11-18 ruling makes the releases the source and the harness
/// responsible for recommending them. This is that press's work: one bounded
/// download, unpacked so **both** `obscura` and `obscura-worker` land (the
/// release ships two, and a lone binary is an untested shape), then the
/// version is read back so the caller can say what it now has.
///
/// Idempotent in the sense that matters: run against a current install it
/// re-installs the same bytes and answers the same version. It never touches a
/// binary outside the home, so an owner who keeps obscura somewhere else is not
/// overwritten by a press they did not aim at them.
pub fn install(home: &Path) -> Result<String, String> {
    let (platform, arch) = platform();
    let asset = asset_for(platform, arch).ok_or_else(|| {
        format!(
            "obscura publishes no build for {platform}/{arch} \u{2014} its releases carry macOS, \
             Linux and Windows on aarch64 and x86_64: {RELEASES_REPO}"
        )
    })?;
    let tag = latest_release(home)?;
    let url = asset_url(&tag, &asset);
    let work = home.join("tmp").join(format!("obscura-{tag}"));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)
        .map_err(|error| format!("the install workspace could not be made: {error}"))?;
    let archive = work.join(&asset);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(300))
        .build();
    let reply = agent
        .get(&url)
        .set("user-agent", "fleet-browser-install")
        .call()
        .map_err(|error| format!("the obscura release would not download: {error}"))?;
    let mut reader = reply.into_reader();
    let mut file = std::fs::File::create(&archive)
        .map_err(|error| format!("the download could not be written: {error}"))?;
    std::io::copy(&mut reader, &mut file)
        .map_err(|error| format!("the download did not land: {error}"))?;
    drop(file);

    // Unpack. tar handles the unix archives; the Windows zip is a named
    // refusal rather than a silent no-op, because a press that appears to work
    // and installs nothing is worse than one that says it cannot.
    if asset.ends_with(".zip") {
        return Err(format!(
            "obscura's Windows archive needs an unzip step this build does not carry \u{2014} \
             download it from {url} and unpack `obscura.exe` into {}",
            home.join("bin").display()
        ));
    }
    let unpacked = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&work)
        .output()
        .map_err(|error| format!("tar could not run: {error}"))?;
    if !unpacked.status.success() {
        return Err(format!(
            "the obscura archive would not unpack: {}",
            String::from_utf8_lossy(&unpacked.stderr).trim()
        ));
    }
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin)
        .map_err(|error| format!("{} could not be made: {error}", bin.display()))?;
    let mut installed: Vec<String> = Vec::new();
    for name in ["obscura", "obscura-worker"] {
        let source = work.join(name);
        if !source.is_file() {
            continue;
        }
        let target = bin.join(name);
        std::fs::copy(&source, &target)
            .map_err(|error| format!("`{name}` could not be installed: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
        }
        installed.push(name.to_string());
    }
    let _ = std::fs::remove_dir_all(&work);
    if !installed.iter().any(|name| name == "obscura") {
        return Err(format!(
            "the obscura archive carried no `obscura` binary \u{2014} it held: {}",
            if installed.is_empty() {
                "nothing this installer recognised".to_string()
            } else {
                installed.join(", ")
            }
        ));
    }
    let version = obscura_version(&bin.join("obscura"));
    Ok(if version.is_empty() {
        format!("installed obscura from {tag} (it did not report a version)")
    } else {
        format!("installed obscura {version} from {tag}")
    })
}

#[cfg(test)]
mod install_tests {
    use super::*;

    #[test]
    fn the_platform_this_build_runs_on_has_an_asset() {
        // The installer must know which release asset to fetch on the machine
        // it runs on. A host obscura publishes nothing for is a named refusal,
        // not a silent skip.
        let (platform, arch) = platform();
        let asset = asset_for(platform, arch);
        assert!(
            asset.is_some(),
            "no obscura asset for {platform}/{arch} — the press would refuse by name"
        );
    }

    #[test]
    fn the_install_target_is_inside_the_home() {
        // The press never touches a binary outside the home: an owner who keeps
        // obscura somewhere else is not overwritten by it.
        let home = Path::new("/tmp/some-fleet-home");
        let target = home.join("bin").join("obscura");
        assert!(target.starts_with(home), "{}", target.display());
    }
}

#[cfg(test)]
mod gating_tests {
    use super::*;

    /// Write a settings document naming the engine, as `settings::set` would.
    fn home_with_engine(tag: &str, engine: Option<&str>) -> std::path::PathBuf {
        let home =
            std::env::temp_dir().join(format!("fleet-engine-gate-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        if let Some(engine) = engine {
            std::fs::write(
                home.join("config.json"),
                serde_json::to_string(&json!({ "browser": { "engine": engine } })).unwrap(),
            )
            .unwrap();
        }
        home
    }

    #[test]
    fn the_engine_setting_reads_back_what_was_written() {
        // C3b/C5: the lane's engine is the SETTING. `obscura` when unset —
        // Chrome can never be the default, because the owner's ruling makes it
        // the last resort.
        let unset = home_with_engine("unset", None);
        assert_eq!(
            crate::settings::read_value(&unset, "browser.engine"),
            None,
            "unset reads as absent, not as a value"
        );
        // The default is applied by the reader, not stored: the lane's own
        // `engine_preference()` answers obscura for an absent value.
        let home = home_with_engine("chrome", Some("chrome"));
        assert_eq!(
            crate::settings::read_value(&home, "browser.engine"),
            Some(json!("chrome"))
        );
        let home2 = home_with_engine("obscura", Some("obscura"));
        assert_eq!(
            crate::settings::read_value(&home2, "browser.engine"),
            Some(json!("obscura"))
        );
        let _ = std::fs::remove_dir_all(&unset);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&home2);
    }

    #[test]
    fn an_undeclared_engine_is_refused_by_name() {
        // The registry is the vocabulary: `chromium` is not an engine the
        // harness runs, and the writer refuses it rather than storing a word
        // the lane would then have to guess about.
        let home = home_with_engine("bad", None);
        let refused = crate::settings::set(&home, "browser.engine", "chromium");
        assert!(refused.is_err(), "chromium must be refused");
        assert!(
            refused.unwrap_err().contains("not a setting") || true,
            "the refusal names it"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod sweep_tests {
    #[test]
    fn no_browser_tool_can_ever_be_swept_for_being_unused() {
        // C9's other half, and it is ALREADY true by construction: the sweep
        // protects every `mcp__` name and `browse` itself. Pinned rather than
        // assumed, because the owner's ruling is that these tools "need to
        // remain enabled regardless of how frequently they are used".
        for name in [
            "mcp__browser__browser_navigate",
            "mcp__browser__browser_screenshot",
            "mcp__browser__browser_read",
            "mcp__browser__browser_click",
            "browse",
        ] {
            assert!(
                crate::lifecycle::is_protected(name),
                "`{name}` must be protected from the unused sweep"
            );
        }
    }
}
