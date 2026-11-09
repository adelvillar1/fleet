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
/// The update format's two row shapes both live here: the plugin's
/// rows sit under `platforms`, the CLI's row is a top-level `cli`
/// object beside them — Tauri's own feed shape, not a second format.
#[derive(Debug, Clone)]
pub struct ReleaseFeed {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: Value,
    pub cli: Option<Value>,
}

impl ReleaseFeed {
    /// One row's branch-permanent artifact url, with the row's
    /// signature field beside it: the minisign signature for the
    /// shell's platform rows, the sha256 hex for the cli row. The cli
    /// row reads off the feed's top-level `cli` object; every other
    /// key reads off `platforms`.
    fn row(&self, key: &str) -> Result<(String, Option<String>), String> {
        let entry = if key == "cli" {
            self.cli.as_ref().and_then(Value::as_object)
        } else {
            self.platforms.get(key).and_then(Value::as_object)
        }
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

/// Build the struct from one feed document. The one mapping the fix
/// lives in: the cli row is the feed's top-level `cli` object — NOT
/// under `platforms` (KI-483: the parser kept only `platforms`, so
/// `row("cli")` refused a feed that carries the row, and every
/// CLI-channel apply answered "the feed carries no cli row").
fn parse(value: &Value) -> Result<ReleaseFeed, String> {
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
        cli: value.get("cli").cloned(),
    })
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
    parse(&value)
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

/// The update's source: the feed and the artifact bytes. The production
/// source is GitHub and nothing else (the owner's 2026-11-09 ruling —
/// the update is fetched from GitHub, never from the local machine);
/// the trait exists so a test can drive the install leg without a
/// network, the seam the KI-472 lesson asks for at the one path where a
/// wrong artifact costs the binary instead of a red test.
pub trait Source {
    fn feed(&self) -> Result<ReleaseFeed, String>;
    fn artifact(&self, url: &str, timeout: Duration) -> Result<Vec<u8>, String>;
}

/// The only production source: the branch-permanent GitHub feed and the
/// artifacts its rows name.
pub struct Github;

impl Source for Github {
    fn feed(&self) -> Result<ReleaseFeed, String> {
        fetch()
    }

    fn artifact(&self, url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
        download(url, timeout)
    }
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

/// What one apply did, as the receipt and the card word it. `gap` is
/// `same` when the feed names the running build and `ahead` when it
/// installed one; `version` is the feed's version (the one on disk now);
/// `previous` is the kept copy of the binary the rename replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub gap: &'static str,
    pub version: String,
    pub previous: Option<std::path::PathBuf>,
}

impl Update {
    pub fn installed(&self) -> bool {
        self.gap == "ahead"
    }
}

/// The kept copy's path: the binary's own name with a `.prev` suffix,
/// beside it, so a bad release is recoverable from the machine.
pub fn previous_for(exe: &Path) -> std::path::PathBuf {
    let name = exe
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "fleet".to_string());
    exe.with_file_name(format!("{name}.prev"))
}

/// The staged artifact's own answer to `--version`, read by executing
/// it. This is the proof the channel never had: the bytes that were
/// downloaded must be a binary that runs and names the feed's version,
/// so a tarball, a truncation or a foreign build refuses before the
/// rename rather than after it (KI-486).
fn staged_version(path: &Path) -> Result<String, String> {
    let output = std::process::Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("the staged artifact did not run: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        // A file the kernel will not execute reads the same way here
        // whichever way it failed (a tarball's bytes reach `/bin/sh`'s
        // fallback and exit 126; a bad Mach-O header is ENOEXEC) — the
        // refusal names what happened, not a guess about the shape.
        let said = if text.is_empty() {
            String::new()
        } else {
            format!(" and said {text:?}")
        };
        return Err(format!(
            "the staged artifact did not run: it answered {}{said}",
            output.status
        ));
    }
    // The binary prints `fleet <version>`; the last word is the answer.
    let word = text
        .split_whitespace()
        .last()
        .unwrap_or_default()
        .to_string();
    if parse_version(&word).is_err() {
        return Err(format!(
            "the staged artifact answered {text:?}, which is not a version"
        ));
    }
    Ok(word)
}

/// The CLI's own verb: the release's **raw binary** asset — the shape
/// the release ships and the feed's cli row names (`fleet-aarch64-apple-darwin`,
/// no archive) — asserted against the feed's sha256, proven by running
/// it, written beside the running binary and renamed over it. One
/// same-directory rename, so the replace is atomic and the running
/// process keeps its own inode to the end; the binary it replaced is
/// kept as `<exe>.prev`. The launchd agent's restart contract puts the
/// new binary on the port at the next restart, and the receipt names
/// the version the feed installed.
///
/// Three refusals, each named before the rename:
///
/// - a dev build: a checkout's binary is the build's to replace;
/// - a bundle: the `.app` updates through its own window (the About
///   pane's plugin chain), never through this channel — writing a CLI
///   binary over `Contents/MacOS/fleet-desktop` would break the shell;
/// - a feed behind the running build: the feed must not pull a checkout
///   down.
///
/// Every other refusal is the step that would have failed, named before
/// the rename — no partial install survives, because the rename is the
/// last instruction and the staged artifact must have answered
/// `--version` with the feed's version first.
pub fn self_update(exe: &Path) -> Result<Update, String> {
    self_update_over(&Github, exe)
}

/// [`self_update`] over an injected source — the test seam.
pub fn self_update_over(source: &dyn Source, exe: &Path) -> Result<Update, String> {
    match installed_via(exe) {
        MODE_BUILD => {
            return Err(
                "a dev build does not update the checkout — build it again instead".to_string(),
            )
        }
        MODE_BUNDLE => {
            return Err(
                "a bundled app updates through its own window — this channel is the CLI's"
                    .to_string(),
            )
        }
        _ => {}
    }
    let feed = source.feed()?;
    let gap = version_gap(current_version(), &feed.version)?;
    if gap == "same" {
        return Ok(Update {
            gap,
            version: feed.version,
            previous: None,
        });
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
    let bytes = source.artifact(&url, FEED_TIMEOUT * 12)?;
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
    // The proof KI-486 was missing: the artifact must run and answer the
    // feed's version. A refusal takes the staging file with it, so the
    // tree is exactly as it was.
    let answered = match staged_version(&staging) {
        Ok(answered) => answered,
        Err(reason) => {
            let _ = std::fs::remove_file(&staging);
            return Err(reason);
        }
    };
    if answered != feed.version {
        let _ = std::fs::remove_file(&staging);
        return Err(format!(
            "the staged artifact answers {answered}, not the feed's {}",
            feed.version
        ));
    }
    // The keep: a copy of the binary about to be replaced, taken before
    // the rename so a bad release is recoverable from this machine.
    let previous = previous_for(exe);
    std::fs::copy(exe, &previous)
        .map_err(|error| format!("the previous binary was not kept: {error}"))?;
    std::fs::rename(&staging, exe)
        .map_err(|error| format!("the atomic replace failed: {error}"))?;
    Ok(Update {
        gap,
        version: feed.version,
        previous: Some(previous),
    })
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

    /// The one feed shape release.sh ships: platform rows under
    /// `platforms`, the CLI's row as a top-level `cli` object beside
    /// them. Both channels must resolve their rows from it.
    #[test]
    fn feed_with_a_top_level_cli_row_answers_the_cli_channel() {
        let feed = ReleaseFeed {
            version: "0.6.2".into(),
            notes: String::new(),
            pub_date: "2026-11-08T00:00:00Z".into(),
            platforms: json!({
                "darwin-aarch64": {
                    "url": "https://example.com/fleet.app.tar.gz",
                    "signature": "minisign-b64"
                }
            }),
            cli: Some(json!({
                "url": "https://example.com/fleet-cli.tar.gz",
                "signature": "abcdef1234567890"
            })),
        };

        assert_eq!(
            feed.row("cli").expect("cli row must resolve"),
            (
                "https://example.com/fleet-cli.tar.gz".to_string(),
                Some("abcdef1234567890".to_string())
            )
        );
        assert_eq!(
            feed.row("darwin-aarch64")
                .expect("platform rows still resolve"),
            (
                "https://example.com/fleet.app.tar.gz".to_string(),
                Some("minisign-b64".to_string())
            )
        );
    }

    /// A feed with no cli row must still refuse the cli channel with
    /// the same message — not a platform miss misreported.
    #[test]
    fn a_feed_without_a_cli_row_still_refuses_the_cli_channel() {
        let feed = ReleaseFeed {
            version: "0.6.2".into(),
            notes: String::new(),
            pub_date: "2026-11-08T00:00:00Z".into(),
            platforms: json!({
                "darwin-aarch64": {
                    "url": "https://example.com/fleet.app.tar.gz",
                    "signature": "minisign-b64"
                }
            }),
            cli: None,
        };

        assert_eq!(
            feed.row("cli").expect_err("no cli row must refuse"),
            "the feed carries no cli row"
        );
    }

    /// The cli row must not leak into the platform namespace: a feed
    /// carrying only `cli` refuses a platform key.
    #[test]
    fn the_cli_row_does_not_answer_platform_keys() {
        let feed = ReleaseFeed {
            version: "0.6.2".into(),
            notes: String::new(),
            pub_date: "2026-11-08T00:00:00Z".into(),
            platforms: json!({}),
            cli: Some(json!({
                "url": "https://example.com/fleet-cli.tar.gz",
                "signature": "abcdef1234567890"
            })),
        };

        assert_eq!(
            feed.row("darwin-aarch64")
                .expect_err("no platform row must refuse"),
            "the feed carries no darwin-aarch64 row"
        );
    }

    /// KI-483's shape end to end: the real parser must keep the feed's
    /// top-level `cli` object, so `row("cli")` resolves from a real
    /// fetched payload — no hand-built struct standing in for fetch().
    #[test]
    fn the_parser_keeps_the_feed_s_top_level_cli_object() {
        let payload = serde_json::json!({
            "version": "0.6.2",
            "pub_date": "2026-11-08T00:00:00Z",
            "notes": "",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/fleet.app.tar.gz",
                    "signature": "minisign-b64"
                }
            },
            "cli": {
                "url": "https://example.com/fleet-cli.tar.gz",
                "signature": "abcdef1234567890"
            }
        });
        let feed = parse(&payload).expect("the real parser must read the feed");

        assert_eq!(
            feed.row("cli").expect("cli row must survive the parse"),
            (
                "https://example.com/fleet-cli.tar.gz".to_string(),
                Some("abcdef1234567890".to_string())
            )
        );
        assert_eq!(
            feed.row("darwin-aarch64")
                .expect("the platform row must survive the parse"),
            (
                "https://example.com/fleet.app.tar.gz".to_string(),
                Some("minisign-b64".to_string())
            )
        );
    }

    /// Revert-proof at the exact fault line: with the cli mapping
    /// gone from the payload, the parser must refuse the cli channel
    /// again — the way 0.6.0 did against a feed that carried the row.
    #[test]
    fn the_parser_dropping_the_cli_row_fails_the_way_0_6_0_did() {
        let payload = serde_json::json!({
            "version": "0.6.2",
            "pub_date": "2026-11-08T00:00:00Z",
            "notes": "",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://example.com/fleet.app.tar.gz",
                    "signature": "minisign-b64"
                }
            },
            "cli": {
                "url": "https://example.com/fleet-cli.tar.gz",
                "signature": "abcdef1234567890"
            }
        });
        let mut broken = payload.clone();
        broken.as_object_mut().unwrap().remove("cli");
        let feed = parse(&broken).expect("a platforms-only feed still parses");

        assert_eq!(
            feed.row("cli")
                .expect_err("without the cli mapping the channel refuses"),
            "the feed carries no cli row"
        );
    }

    // --- the install leg, over an injected source (the KI-472 seam) ------

    /// A source a test owns: the feed and the artifact bytes, no network.
    struct Fake {
        feed: ReleaseFeed,
        artifact: Result<Vec<u8>, String>,
    }

    impl Source for Fake {
        fn feed(&self) -> Result<ReleaseFeed, String> {
            Ok(self.feed.clone())
        }

        fn artifact(&self, _url: &str, _timeout: Duration) -> Result<Vec<u8>, String> {
            self.artifact.clone()
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("fleet-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// A feed one step ahead of the compiled build, whose cli row names
    /// the artifact's own sha256 — the shape release.sh publishes.
    fn ahead_feed(version: &str, hash: &str) -> ReleaseFeed {
        ReleaseFeed {
            version: version.to_string(),
            notes: String::new(),
            pub_date: "2026-11-09T00:00:00Z".into(),
            platforms: json!({
                "darwin-aarch64": {
                    "url": "https://example.com/fleet.app.tar.gz",
                    "signature": "minisign-b64"
                }
            }),
            cli: Some(json!({
                "url": "https://example.com/fleet-aarch64-apple-darwin",
                "signature": hash
            })),
        }
    }

    /// A scripted stand-in for the release's raw binary: it runs and it
    /// answers `--version` the way the real one does.
    fn artifact_answering(version: &str) -> Vec<u8> {
        format!("#!/bin/sh\necho \"fleet {version}\"\n").into_bytes()
    }

    fn digest(bytes: &[u8]) -> String {
        format!("{:x}", sha2::Sha256::digest(bytes))
    }

    /// C2 — the install leg end to end: the staged artifact runs, the
    /// rename replaces the binary, the replaced binary survives as
    /// `.prev`, and the returned version is the one installed (not the
    /// compiled-in one).
    #[test]
    fn the_install_leg_runs_the_artifact_and_keeps_the_previous_binary() {
        let dir = scratch("install");
        let exe = dir.join("fleet");
        std::fs::write(&exe, b"the previous build").unwrap();
        let artifact = artifact_answering("9.9.9");
        let source = Fake {
            feed: ahead_feed("9.9.9", &digest(&artifact)),
            artifact: Ok(artifact.clone()),
        };

        let update = self_update_over(&source, &exe).expect("the install must land");

        assert_eq!(update.gap, "ahead");
        assert_eq!(update.version, "9.9.9");
        assert_eq!(
            std::fs::read(&exe).unwrap(),
            artifact,
            "the artifact is what the binary now holds"
        );
        let previous = update.previous.expect("the replaced binary is kept");
        assert_eq!(previous, previous_for(&exe));
        assert_eq!(
            std::fs::read(&previous).unwrap(),
            b"the previous build",
            "the keep is the binary the rename replaced"
        );
        assert!(!staging_for(&exe).exists(), "no staging file survives");
    }

    /// A feed naming the running build touches nothing.
    #[test]
    fn a_same_feed_installs_nothing_and_keeps_no_previous() {
        let dir = scratch("same");
        let exe = dir.join("fleet");
        std::fs::write(&exe, b"the running build").unwrap();
        let source = Fake {
            feed: ahead_feed(current_version(), "unused"),
            artifact: Err("the artifact must not be asked for".to_string()),
        };

        let update = self_update_over(&source, &exe).expect("same is not a failure");

        assert_eq!(update.gap, "same");
        assert!(update.previous.is_none());
        assert_eq!(std::fs::read(&exe).unwrap(), b"the running build");
        assert!(!previous_for(&exe).exists());
    }

    /// C3, shape one — the gzip tarball the 0.6.x release actually
    /// shipped (KI-486): the sha256 is the tarball's own, so the digest
    /// check passes and the proof must stop it.
    #[test]
    fn a_tarball_artifact_is_refused_before_the_rename() {
        let dir = scratch("tarball");
        let exe = dir.join("fleet");
        std::fs::write(&exe, b"the running build").unwrap();
        let artifact = b"\x1f\x8b\x08\x00\x00\x00\x00\x00the tarball".to_vec();
        let source = Fake {
            feed: ahead_feed("9.9.9", &digest(&artifact)),
            artifact: Ok(artifact),
        };

        let reason = self_update_over(&source, &exe).expect_err("a tarball is not a binary");

        assert!(reason.contains("did not run"), "{reason}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"the running build");
        assert!(!previous_for(&exe).exists());
        assert!(
            !staging_for(&exe).exists(),
            "the refusal takes the staging file"
        );
    }

    /// C3, shape two — a truncated artifact: the bytes are not a file
    /// the kernel will execute.
    #[test]
    fn a_truncated_artifact_is_refused_before_the_rename() {
        let dir = scratch("truncated");
        let exe = dir.join("fleet");
        std::fs::write(&exe, b"the running build").unwrap();
        let artifact = b"\xcf\xfa\xed\xfe".to_vec();
        let source = Fake {
            feed: ahead_feed("9.9.9", &digest(&artifact)),
            artifact: Ok(artifact),
        };

        let reason = self_update_over(&source, &exe).expect_err("a header is not a binary");

        assert!(!reason.is_empty());
        assert_eq!(std::fs::read(&exe).unwrap(), b"the running build");
        assert!(!staging_for(&exe).exists());
    }

    /// C3, shape three — a real binary that is the wrong build: the
    /// refusal names the answer it gave.
    #[test]
    fn a_foreign_binary_is_refused_by_its_own_answer() {
        let dir = scratch("foreign");
        let exe = dir.join("fleet");
        std::fs::write(&exe, b"the running build").unwrap();
        let artifact = artifact_answering("0.0.1");
        let source = Fake {
            feed: ahead_feed("9.9.9", &digest(&artifact)),
            artifact: Ok(artifact),
        };

        let reason = self_update_over(&source, &exe).expect_err("the wrong build must refuse");

        assert!(
            reason.contains("answers 0.0.1") && reason.contains("9.9.9"),
            "{reason}"
        );
        assert_eq!(std::fs::read(&exe).unwrap(), b"the running build");
        assert!(!previous_for(&exe).exists());
    }

    /// C4 — a bundled executable never takes the CLI channel: the shell
    /// updates through its own window, and a CLI binary must not land on
    /// `Contents/MacOS/fleet-desktop`.
    #[test]
    fn a_bundled_executable_refuses_the_cli_channel_by_name() {
        let dir = scratch("bundle");
        let exe = dir.join("Fleet.app/Contents/MacOS/fleet-desktop");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"the shell").unwrap();
        let source = Fake {
            feed: ahead_feed("9.9.9", "unused"),
            artifact: Err("the artifact must not be asked for".to_string()),
        };

        let reason = self_update_over(&source, &exe).expect_err("the bundle must refuse");

        assert!(reason.contains("own window"), "{reason}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"the shell");
        assert!(!staging_for(&exe).exists());
    }

    #[test]
    fn the_previous_path_sits_beside_the_binary() {
        assert_eq!(
            previous_for(Path::new("/Users/x/.fleet/bin/fleet")),
            Path::new("/Users/x/.fleet/bin/fleet.prev")
        );
    }
}
