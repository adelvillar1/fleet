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
}
