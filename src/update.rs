//! The self-update channels: one release feed, two consumers.
//!
//! The owner's ruling 2026-11-08 — "we need to find a way to update the
//! tauri app from within the app itself, this juggling of versions is
//! getting ridiculous" — names the problem and its release: this module
//! ships in 0.6.0, the last version the owner installs by hand. Every
//! cut after it rides the feed the release script uploads beside the
//! artifacts.
//!
//! The feed is the release's `latest.json` in the Tauri v2 updater's
//! shape, at the branch-permanent
//! `releases/latest/download/latest.json`, so the feed never ages a
//! version number into its own URL. Two consumers read it:
//!
//! - the **desktop shell** — the updater plugin reads the same feed
//!   for its signed install chain; this module hands the settings card
//!   its field, and the apply verb lives on the plugin, not here;
//! - the **CLI** — the `update` verb downloads the release's raw
//!   binary asset, asserts the feed's hash against the received bytes,
//!   and replaces its own path with one same-directory rename; the
//!   launchd agents' restart contracts put the new binary on the port.
//!
//! The honesty clause holds through every refusal: a feed that could
//! not be reached, or a candidate that is not a step forward, names
//! itself — never an "up to date", because no-network is not
//! up-to-date.

use serde_json::{json, Value};
use sha2::Digest;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// The release feed, at the URL that always names the newest release.
pub const FEED: &str = "https://github.com/adelvillar1/fleet/releases/latest/download/latest.json";

/// How long a feed ask may run before it refuses. A loaded machine is
/// not a permission to hang the card.
pub const FEED_TIMEOUT: Duration = Duration::from_secs(10);

/// The largest feed read held in memory. The manifest is a kilobyte;
/// the ceiling exists so a wrong reply cannot fill the heap, and an
/// over-long reply is its own named refusal.
pub const FEED_MAX: u64 = 16 * 1024 * 1024;

/// The largest artifact download held in memory. Above any artifact
/// the house ships.
pub const DOWNLOAD_MAX: u64 = 256 * 1024 * 1024;

/// The vault key that carries the updater's private half, sealed by
/// the owner at signer setup; the public half is pinned in the shell's
/// tauri config.
pub const VAULT_KEY: &str = "FLEET_TAURI_UPDATE_KEY";

/// Install modes, as the card words them.
pub const MODE_BUNDLE: &str = "dmg";
pub const MODE_BUILD: &str = "dev-build";
pub const MODE_COPY: &str = "source";

/// Where the running binary came from, read off its own path: a
/// bundle's `MacOS` directory is the installed app; a build
/// directory's adjacency (`target` beside `release` or `debug`) is a
/// build; every other copy — the CLI's installed home included — is a
/// source install whose channel is the CLI's own verb.
pub fn installed_via(exe: &Path) -> &'static str {
    let text = exe.to_string_lossy().replace('\\', "/");
    if text.contains(".app/Contents/MacOS/") {
        return MODE_BUNDLE;
    }
    let segments: Vec<&str> = text.split('/').collect();
    for window in segments.windows(2) {
        if window[0] == "target" && (window[1] == "release" || window[1] == "debug") {
            return MODE_BUILD;
        }
    }
    MODE_COPY
}

/// A version triple, read from the three numeric parts before any
/// pre-release or build suffix. A string that does not carry exactly
/// three numeric parts is a refusal, not a guess: the feed is
/// machine-written, and a malformed number is exactly when to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

fn parse_version(text: &str) -> Result<Version, String> {
    let core = text.split(['-', '+']).next().unwrap_or(text);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("not a three-part version: {text}"));
    }
    let one = |part: &str| -> Result<u64, String> {
        part.parse::<u64>()
            .map_err(|_| format!("not a number in {text}"))
    };
    Ok(Version {
        major: one(parts[0])?,
        minor: one(parts[1])?,
        patch: one(parts[2])?,
    })
}

/// What the feed's candidate means for the running version, as the
/// card words it: `ahead` names a candidate the card may carry, `same`
/// names nothing to do, `behind` names a feed older than the running
/// build — which a check reports and an update refuses, because a
/// feed that lags a checkout must not pull the checkout down.
pub fn version_gap(current: &str, candidate: &str) -> Result<&'static str, String> {
    let left = parse_version(current)?;
    let right = parse_version(candidate)?;
    match (right.major, right.minor, right.patch).cmp(&(left.major, left.minor, left.patch)) {
        std::cmp::Ordering::Greater => Ok("ahead"),
        std::cmp::Ordering::Equal => Ok("same"),
        std::cmp::Ordering::Less => Ok("behind"),
    }
}

/// The feed read once: the version it names, its notes and date, and
/// the platform rows the plugin and the CLI read their artifacts from.
#[derive(Debug, Clone)]
pub struct ReleaseFeed {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: Value,
}

impl ReleaseFeed {
    /// One platform row's branch-permanent artifact url, with the
    /// row's signature field beside it: the minisign signature for the
    /// shell's rows, the sha256 hex for the cli row.
    fn row(&self, key: &str) -> Result<(String, Option<String>), String> {
        let entry = self
            .platforms
            .get(key)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("the feed carries no {key} row"))?;
        let url = entry
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("the feed's {key} row names no url"))?
            .to_string();
        let signature = entry
            .get("signature")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok((url, signature))
    }
}

/// Read the feed: one bounded HTTPS GET, parsed once. The one network
/// question the check rests on; every refusal names itself.
pub fn fetch() -> Result<ReleaseFeed, String> {
    let agent = ureq::AgentBuilder::new().timeout(FEED_TIMEOUT).build();
    let reply = agent
        .get(FEED)
        .call()
        .map_err(|error| format!("the release feed was not reached: {error}"))?;
    if reply.status() != 200 {
        return Err(format!(
            "the release feed was not reached: HTTP {}",
            reply.status()
        ));
    }
    if let Some(length) = reply
        .header("content-length")
        .and_then(|len| len.parse::<u64>().ok())
    {
        if length > FEED_MAX {
            return Err("the feed is unreadable: larger than the ceiling".to_string());
        }
    }
    let mut capped = reply.into_reader().take(FEED_MAX);
    let mut body = String::new();
    capped
        .read_to_string(&mut body)
        .map_err(|error| format!("the feed did not read: {error}"))?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| format!("the feed is not readable json: {error}"))?;
    Ok(ReleaseFeed {
        version: value
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| "the feed names no version".to_string())?
            .to_string(),
        notes: value
            .get("notes")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pub_date: value
            .get("pub_date")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        platforms: value
            .get("platforms")
            .and_then(Value::as_object)
            .map(|rows| Value::Object(rows.clone()))
            .unwrap_or_else(|| json!({})),
    })
}

/// The update card's field, computed at check time. The shape: the
/// running and candidate versions, the notes' head, the release date,
/// the install mode, and the CLI artifact's url — or the refusal,
/// carried verbatim, so the card shows exactly what went wrong and
/// never shows a refusal as an up-to-date.
pub fn check_field(current: &str) -> Value {
    let mut field = json!({
        "current": current,
        "installed_via": installed_via(&std::env::current_exe().unwrap_or_default()),
    });
    match fetch() {
        Ok(feed) => match version_gap(current, &feed.version) {
            Ok("same") => {
                field["candidate"] = Value::Null;
                field["notes"] = json!(feed.notes.lines().next().unwrap_or_default());
                field["pub_date"] = json!(feed.pub_date);
            }
            Ok(_) => {
                field["candidate"] = json!(feed.version);
                field["notes"] = json!(feed.notes.lines().next().unwrap_or_default());
                field["pub_date"] = json!(feed.pub_date);
                if let Ok((url, _)) = feed.row("cli") {
                    field["url"] = json!(url);
                }
            }
            Err(gap) => {
                field["refused"] = json!({"reason": gap, "version": feed.version});
            }
        },
        Err(reason) => {
            field["refused"] = json!({"reason": reason});
        }
    }
    field
}

/// One bounded artifact download, bytes in memory, under the ceiling.
fn download(url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let reply = agent
        .get(url)
        .call()
        .map_err(|error| format!("the artifact was not reached: {error}"))?;
    if reply.status() != 200 {
        return Err(format!(
            "the artifact was not reached: HTTP {}",
            reply.status()
        ));
    }
    let mut capped = reply.into_reader().take(DOWNLOAD_MAX);
    let mut bytes = Vec::new();
    capped
        .read_to_end(&mut bytes)
        .map_err(|error| format!("the artifact did not read: {error}"))?;
    Ok(bytes)
}

/// The running binary's own version, as the crate was compiled. The
/// receipt the CLI prints after the rename — the version it just put
/// on disk, whether or not this process ran it yet.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// A staging path beside the binary: same name, `.update` suffix, so
/// the rename that replaces the running file is a rename and never a
/// cross-volume copy.
pub fn staging_for(exe: &Path) -> std::path::PathBuf {
    let name = exe
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "fleet".to_string());
    exe.with_file_name(format!("{name}.update"))
}

/// The CLI's own verb: the release's raw binary asset, asserted
/// against the feed's sha256, written beside the running binary and
/// renamed over it — one same-directory rename, so the replace is
/// atomic and the running process keeps its own inode to the end. The
/// preserved mode keeps the exec bit; the launchd agent's restart
/// contract puts the new binary on the port at the next restart, and
/// the receipt answers `--version` at `0.6.0`.
///
/// Builds refuse: a checkout's binary is the build's to replace, and
/// the refusal names it. A feed behind the running version refuses:
/// the feed must not pull a checkout down. Every other refusal is the
/// step that would have failed, named before the rename — no partial
/// install survives, because the rename is the last instruction.
pub fn self_update(exe: &Path) -> Result<&'static str, String> {
    if installed_via(exe) == MODE_BUILD {
        return Err(
            "a dev build does not update the checkout — build it again instead".to_string(),
        );
    }
    let feed = fetch()?;
    let gap = version_gap(current_version(), &feed.version)?;
    if gap == "same" {
        return Ok("same");
    }
    if gap == "behind" {
        return Err(format!(
            "the release feed names {} — older than this build's {}; rebuild instead",
            feed.version,
            current_version()
        ));
    }
    let (url, hash) = feed.row("cli")?;
    let expected = hash
        .ok_or_else(|| "the feed's cli row carries no sha256".to_string())?
        .to_ascii_lowercase();
    let bytes = download(&url, FEED_TIMEOUT * 12)?;
    let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
    if digest != expected {
        return Err(format!(
            "the artifact's digest does not match the feed's ({digest} != {expected})"
        ));
    }
    let staging = staging_for(exe);
    std::fs::write(&staging, &bytes)
        .map_err(|error| format!("the staging write failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(exe) {
            let mode = meta.permissions().mode();
            let _ =
                std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(mode | 0o111));
        }
    }
    std::fs::rename(&staging, exe)
        .map_err(|error| format!("the atomic replace failed: {error}"))?;
    Ok("ahead")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bundle_path_is_the_dmg_mode() {
        assert_eq!(
            installed_via(Path::new(
                "/Applications/fleet.app/Contents/MacOS/fleet-desktop"
            )),
            MODE_BUNDLE
        );
    }

    #[test]
    fn a_target_path_is_the_dev_mode() {
        assert_eq!(
            installed_via(Path::new(
                "/Users/x/Projects/fleet/target/release/fleet-desktop"
            )),
            MODE_BUILD
        );
    }

    #[test]
    fn an_installed_home_is_the_source_mode() {
        assert_eq!(
            installed_via(Path::new("/Users/x/.fleet/bin/fleet")),
            MODE_COPY
        );
        assert_eq!(installed_via(Path::new("/usr/local/bin/fleet")), MODE_COPY);
    }

    #[test]
    fn the_gap_matrix_names_ahead_same_behind() {
        assert_eq!(version_gap("0.5.1", "0.6.0"), Ok("ahead"));
        assert_eq!(version_gap("0.5.1", "0.5.2"), Ok("ahead"));
        assert_eq!(version_gap("0.5.1", "0.5.1"), Ok("same"));
        assert_eq!(version_gap("0.6.0", "0.5.1"), Ok("behind"));
    }

    #[test]
    fn a_malformed_version_is_a_refusal() {
        assert!(version_gap("0.5.1", "november").is_err());
        assert!(version_gap("zero", "0.5.1").is_err());
        assert!(version_gap("0.5.1", "0.5").is_err());
    }

    #[test]
    fn a_prerelease_suffix_reads_its_core_triple() {
        assert_eq!(version_gap("0.5.0-beta.1", "0.5.1"), Ok("ahead"));
        assert_eq!(version_gap("0.5.1", "0.6.0-rc.1"), Ok("ahead"));
    }

    #[test]
    fn the_staging_path_sits_beside_the_binary() {
        let staging = staging_for(Path::new("/Users/x/.fleet/bin/fleet"));
        assert_eq!(staging, Path::new("/Users/x/.fleet/bin/fleet.update"));
        assert_eq!(
            staging_for(Path::new(
                "/Applications/fleet.app/Contents/MacOS/fleet-desktop"
            )),
            Path::new("/Applications/fleet.app/Contents/MacOS/fleet-desktop.update")
        );
    }

    #[test]
    fn the_feed_and_the_key_name_their_world() {
        assert!(FEED.starts_with("https://github.com/adelvillar1/fleet/"));
        assert!(FEED.ends_with("/latest/download/latest.json"));
        assert_eq!(VAULT_KEY, "FLEET_TAURI_UPDATE_KEY");
    }
}
