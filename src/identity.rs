//! Fleet's identity: the install-id and the principal — armada's bedrock (L1).
//!
//! Two identities, deliberately distinct (the design-decisions ledger D0–D2,
//! D7–D9):
//!
//! - The **install-id** identifies one fleet install. It is minted once, at
//!   install, persisted in the home, and never re-minted while it exists. A
//!   fresh home is a fresh install. The corpus cursor, the roster, and
//!   deployment all key on it (D7).
//! - The **principal** is the unit of authority an action acts for, distinct
//!   from the device it arrives on. Lane 1 has exactly one principal, the
//!   `owner`; every contract is principal-agnostic so the org (lane 2)
//!   multiplies principals without changing the code (D8). A paired device
//!   maps to a principal, never the reverse (D2).
//!
//! This module only *carries* identity. It grants nothing and refuses
//! nothing — the uniform guard that reads the principal is L3. The
//! principal rides the run as [`crate::agent::AgentOptions::principal`],
//! `None` being the owner, the run this crate has always had (D9).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::filelock::FileLock;

/// The install-id file, inside the home: 32 hex, minted once (D7).
pub const INSTALL_ID_FILE: &str = "install-id";
/// The identity registry, inside the home: principals + device links (D8).
pub const IDENTITY_FILE: &str = "identity.json";

/// The one principal lane 1 knows. The org adds more; the code does not
/// change (D8).
pub const OWNER: &str = "owner";

/// The environment variable naming the device this PROCESS acts as. Set by
/// the instance container's boot (`fleet armada join`); unset on an ordinary
/// CLI run, which resolves to the owner. A property of the process, not the
/// home — two processes over one home are two devices.
pub const PROCESS_DEVICE_ENV: &str = "FLEET_DEVICE_ID";

/// A stable per-install identity: 32 lowercase hex characters (D7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallId(String);

impl InstallId {
    /// The id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InstallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The unit of authority an action acts for (D8). A lowercase slug; lane 1
/// is exactly [`OWNER`]. Distinct from a device and from the install.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalId(String);

impl PrincipalId {
    /// The owner — the one principal of lane 1.
    pub fn owner() -> Self {
        PrincipalId(OWNER.to_string())
    }

    /// A named principal — the org's users (lane 2). Lane 1 only ever
    /// resolves [`Self::owner`]; the constructor exists so the substrate is
    /// principal-agnostic from the first cut.
    pub fn new(slug: &str) -> Self {
        PrincipalId(slug.to_string())
    }

    /// The principal as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PrincipalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A paired client's id — the same id [`crate::remote::pairing::Device`]
/// carries. A device maps to a principal; it is never itself an authority.
pub type DeviceId = String;

/// The install-id's path inside a home.
pub fn install_id_path(home: &Path) -> PathBuf {
    home.join(INSTALL_ID_FILE)
}

/// The identity registry's path inside a home.
pub fn identity_path(home: &Path) -> PathBuf {
    home.join(IDENTITY_FILE)
}

/// Read the install-id, minting it on first use (D7). Idempotent: a home
/// that already holds an id keeps it byte for byte; a fresh home mints a
/// new one. The mint is under the file's flock, and re-checks after taking
/// it, so two writers racing on a fresh home converge on one id rather
/// than each writing its own.
pub fn install(home: &Path) -> Result<InstallId, String> {
    let path = install_id_path(home);
    if let Some(existing) = read_id(&path) {
        return Ok(InstallId(existing));
    }
    let _lock = FileLock::take(&FileLock::beside(&path));
    // Another writer may have minted while we waited for the lock.
    if let Some(existing) = read_id(&path) {
        return Ok(InstallId(existing));
    }
    let id = mint_id();
    write_private(&path, id.as_bytes())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(InstallId(id))
}

/// Read a persisted id, trimmed; an absent or empty file is `None`.
fn read_id(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Mint 16 random bytes as 32 hex — the relay sid's shape, from the OS
/// entropy source so the id is unguessable.
fn mint_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// One device→principal edge (D2). The device is the channel; the principal
/// is the authority it carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLink {
    /// The paired device's id.
    pub device: DeviceId,
    /// The principal the device acts for.
    pub principal: PrincipalId,
}

/// One peer→principal edge (L5, D31): a channel peer (a Telegram chat, a
/// webhook caller) resolved to the principal it acts for. The same human on
/// two channels is two peers linked to one principal — the identity graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLink {
    /// The channel the peer arrives on (`telegram`, `webhook`, `relay`).
    pub channel: String,
    /// The peer's id on that channel.
    pub peer: String,
    /// The principal the peer acts for.
    pub principal: PrincipalId,
}

/// The identity registry: the principals this home knows and the
/// device→principal and peer→principal edges (D8). One store, one home — the
/// communication layer's identity graph (L5) extends this same store, never
/// a second.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    /// The principals known to this home. Lane 1: just the owner.
    #[serde(default)]
    pub principals: Vec<PrincipalId>,
    /// The device→principal edges.
    #[serde(default)]
    pub devices: Vec<DeviceLink>,
    /// The peer→principal edges (L5, D31).
    #[serde(default)]
    pub peers: Vec<PeerLink>,
}

impl Registry {
    /// Read the registry. A home with no registry yet is an empty one (the
    /// owner is implicit), not an error; a registry that cannot be parsed
    /// is an error, because silently dropping links would look like a
    /// revocation.
    pub fn load(home: &Path) -> io::Result<Registry> {
        let path = identity_path(home);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not readable: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(error) => Err(error),
        }
    }

    /// Write the registry back atomically and privately (0600), under the
    /// file's flock — the temp-plus-rename discipline the device store
    /// uses, so a reader never sees a half-written registry and two
    /// writers do not lose each other's link.
    pub fn save(&self, home: &Path) -> io::Result<()> {
        let path = identity_path(home);
        let _lock = FileLock::take(&FileLock::beside(&path));
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        write_private(&path, text.as_bytes())
    }
}

/// The principal a run acts for, from the calling CONTEXT (D8, extended
/// 2026-11-19 for the armada console).
///
/// Three contexts, in precedence order:
///
/// 1. The explicit device the caller names — a paired phone or a relayed
///    peer. A device the registry does not link is a **named refusal**,
///    never a silent fallback: resolving an unlinked device to the owner
///    would be the confused-deputy escalation D32 exists to prevent.
/// 2. This process's own device identity, when one was minted for it (an
///    instance's container boot sets it; a desktop launch may too).
/// 3. The owner — the run this crate has always had, and the default every
///    single-owner home stays on.
///
/// The guard (L3) reads this; the run carries it as
/// `AgentOptions.principal`, `None` resolving here.
pub fn principal_for(home: &Path) -> PrincipalId {
    match process_device(home) {
        Some(device) => principal_of_device_or_owner(home, &device),
        None => PrincipalId::owner(),
    }
}

/// The principal a run acts for when the caller names the device it
/// arrived on — the armada console's and the served endpoint's path.
/// An unlinked device is refused by name (D32), exactly as
/// `device_principal` returning `None` is turned into a refusal by the
/// communication layer.
pub fn principal_for_device(home: &Path, device: &str) -> Result<PrincipalId, String> {
    let device = device.trim();
    if device.is_empty() {
        return Ok(PrincipalId::owner());
    }
    device_principal(home, device).ok_or_else(|| {
        format!(
            "the device `{device}` is not linked to any principal — pairing links one, and an \
             unlinked device never falls back to the owner"
        )
    })
}

/// Resolve a device the registry may know; an unlinked one answers the
/// owner, which is correct for the process's OWN identity (a desktop
/// install that never paired is the owner's) and deliberately different
/// from `principal_for_device`, whose caller named a FOREIGN device.
fn principal_of_device_or_owner(home: &Path, device: &str) -> PrincipalId {
    device_principal(home, device).unwrap_or_else(PrincipalId::owner)
}

/// The device this PROCESS acts as, when one was minted for it. The
/// instance container's boot writes `FLEET_DEVICE_ID`; a desktop that
/// paired may set it; an ordinary CLI run leaves it unset and resolves to
/// the owner. Read from the environment because it is a property of the
/// process, not of the home — two processes over one home are two devices.
pub fn process_device(home: &Path) -> Option<String> {
    let named = std::env::var(PROCESS_DEVICE_ENV).ok()?;
    let named = named.trim();
    if named.is_empty() {
        return None;
    }
    // A named device that the registry does not know is not silently
    // dropped here: `principal_for` answers the owner for the process's
    // OWN identity, and the named-refusal rule belongs to the callers
    // that name a foreign device. The home is accepted for symmetry with
    // the rest of the module and for a future per-home device store.
    let _ = home;
    Some(named.to_string())
}

/// The principal a paired device acts for, if it is linked (D2). An
/// unlinked device is `None` — the communication layer (L5) turns that
/// into a named refusal, never a silent fallback to the owner.
pub fn device_principal(home: &Path, device: &str) -> Option<PrincipalId> {
    let registry = Registry::load(home).ok()?;
    registry
        .devices
        .iter()
        .find(|link| link.device == device)
        .map(|link| link.principal.clone())
}

/// Link a device to a principal (D2) — a governance act, performed at
/// pairing by the owner or the control plane, never by an agent. Upserts
/// the edge and ensures the principal is known. Lane 1 links every device
/// to the owner.
pub fn link_device(home: &Path, device: &str, principal: &PrincipalId) -> Result<(), String> {
    let mut registry = Registry::load(home)
        .map_err(|error| format!("cannot read the identity registry: {error}"))?;
    registry.devices.retain(|link| link.device != device);
    registry.devices.push(DeviceLink {
        device: device.to_string(),
        principal: principal.clone(),
    });
    if !registry.principals.contains(principal) {
        registry.principals.push(principal.clone());
    }
    registry
        .save(home)
        .map_err(|error| format!("cannot write the identity registry: {error}"))
}

/// Resolve a channel peer to the principal it acts for (L5, D31/D32). An
/// unlinked peer is a **named refusal**, never a silent fallback to the
/// owner — a fallback would be a confused-deputy escalation. The edge is a
/// measured pairing, not an assertion by the message.
pub fn resolve(home: &Path, channel: &str, peer: &str) -> Result<PrincipalId, String> {
    let registry = Registry::load(home)
        .map_err(|error| format!("cannot read the identity registry: {error}"))?;
    registry
        .peers
        .iter()
        .find(|link| link.channel == channel && link.peer == peer)
        .map(|link| link.principal.clone())
        .ok_or_else(|| {
            format!("peer `{peer}` on `{channel}` is not linked to a principal — pair it first")
        })
}

/// Link a channel peer to a principal (L5, D31) — a governance act at
/// pairing, never the agent's (D19). Upserts the edge. Lane 1 links every
/// paired peer to the owner.
pub fn link_peer(
    home: &Path,
    channel: &str,
    peer: &str,
    principal: &PrincipalId,
) -> Result<(), String> {
    let mut registry = Registry::load(home)
        .map_err(|error| format!("cannot read the identity registry: {error}"))?;
    registry
        .peers
        .retain(|link| !(link.channel == channel && link.peer == peer));
    registry.peers.push(PeerLink {
        channel: channel.to_string(),
        peer: peer.to_string(),
        principal: principal.clone(),
    });
    if !registry.principals.contains(principal) {
        registry.principals.push(principal.clone());
    }
    registry
        .save(home)
        .map_err(|error| format!("cannot write the identity registry: {error}"))
}

/// Write bytes to `path` atomically (temp + rename) and privately (0600 on
/// unix). The temp name carries the pid so two writers do not rename each
/// other's file out from under it — the device store's own discipline.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut file = fs::File::create(&temp)?;
        use std::io::Write;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-identity-{}-{}",
            std::process::id(),
            mint_id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn install_mints_a_stable_id_and_a_fresh_home_mints_a_different_one() {
        let home = scratch();
        let first = install(&home).unwrap();
        assert_eq!(first.as_str().len(), 32);
        assert!(first.as_str().chars().all(|c| c.is_ascii_hexdigit()));
        // Idempotent: a second read keeps it byte for byte.
        let again = install(&home).unwrap();
        assert_eq!(first, again);
        // A fresh home is a fresh install.
        let other = scratch();
        let different = install(&other).unwrap();
        assert_ne!(first, different);
        fs::remove_dir_all(&home).ok();
        fs::remove_dir_all(&other).ok();
    }

    #[test]
    fn install_id_is_persisted_not_remembered() {
        let home = scratch();
        let first = install(&home).unwrap();
        // The id lives on disk: a fresh read from the path agrees.
        let on_disk = fs::read_to_string(install_id_path(&home)).unwrap();
        assert_eq!(on_disk.trim(), first.as_str());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn lane_one_resolves_every_run_to_the_owner() {
        let home = scratch();
        assert_eq!(principal_for(&home), PrincipalId::owner());
        assert_eq!(principal_for(&home).as_str(), "owner");
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_named_device_resolves_to_its_principal() {
        let home = scratch();
        let operator = PrincipalId::new("operator");
        link_device(&home, "phone-1", &operator).unwrap();
        assert_eq!(principal_for_device(&home, "phone-1").unwrap(), operator);
        // An empty name is the default run, not a refusal.
        assert_eq!(
            principal_for_device(&home, "  ").unwrap(),
            PrincipalId::owner()
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_unlinked_named_device_is_refused_by_name() {
        let home = scratch();
        let refused = principal_for_device(&home, "stranger").expect_err("refused");
        assert!(
            refused.contains("not linked to any principal"),
            "the refusal names the condition: {refused}"
        );
        assert!(
            refused.contains("never falls back to the owner"),
            "the refusal names the rule it refuses to break: {refused}"
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_process_device_env_resolves_the_run() {
        let home = scratch();
        let operator = PrincipalId::new("operator");
        link_device(&home, "box-7", &operator).unwrap();
        // SAFETY: this test owns the variable for its own duration; the
        // suite serializes env-touching cases behind the crate's ambient
        // discipline elsewhere, and this name is unique to this test.
        unsafe { std::env::set_var(PROCESS_DEVICE_ENV, "box-7") };
        assert_eq!(principal_for(&home), operator);
        // An unset variable is the owner's ordinary run.
        unsafe { std::env::remove_var(PROCESS_DEVICE_ENV) };
        assert_eq!(principal_for(&home), PrincipalId::owner());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_linked_device_resolves_to_its_principal_and_persists() {
        let home = scratch();
        // Unlinked is None, never a silent owner.
        assert!(device_principal(&home, "device-1").is_none());
        link_device(&home, "device-1", &PrincipalId::owner()).unwrap();
        assert_eq!(
            device_principal(&home, "device-1"),
            Some(PrincipalId::owner())
        );
        // The link survives a reload (a restart).
        let reloaded = Registry::load(&home).unwrap();
        assert_eq!(reloaded.devices.len(), 1);
        assert!(reloaded.principals.contains(&PrincipalId::owner()));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn link_device_upserts_rather_than_duplicates() {
        let home = scratch();
        link_device(&home, "device-1", &PrincipalId::owner()).unwrap();
        link_device(&home, "device-1", &PrincipalId::owner()).unwrap();
        let registry = Registry::load(&home).unwrap();
        assert_eq!(registry.devices.len(), 1);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_malformed_registry_is_an_error_not_a_silent_reset() {
        let home = scratch();
        fs::write(identity_path(&home), "{ not json").unwrap();
        assert!(Registry::load(&home).is_err());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_identity_graph_resolves_one_principal_across_channels_and_devices() {
        let home = scratch();
        // The same human on Telegram, webhook, and a paired device links to
        // one principal (L5, D31).
        link_peer(&home, "telegram", "chat-1", &PrincipalId::owner()).unwrap();
        link_peer(&home, "webhook", "caller-1", &PrincipalId::owner()).unwrap();
        link_device(&home, "device-1", &PrincipalId::owner()).unwrap();
        assert_eq!(
            resolve(&home, "telegram", "chat-1").unwrap(),
            PrincipalId::owner()
        );
        assert_eq!(
            resolve(&home, "webhook", "caller-1").unwrap(),
            PrincipalId::owner()
        );
        assert_eq!(
            device_principal(&home, "device-1"),
            Some(PrincipalId::owner())
        );
        // The edges persist across a reload.
        let registry = Registry::load(&home).unwrap();
        assert_eq!(registry.peers.len(), 2);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_unlinked_peer_is_a_named_refusal_never_a_silent_owner_fallback() {
        let home = scratch();
        let error = resolve(&home, "telegram", "stranger").unwrap_err();
        assert!(error.contains("not linked to a principal"), "{error}");
        // Linking then re-resolving succeeds; upsert does not duplicate.
        link_peer(&home, "telegram", "stranger", &PrincipalId::owner()).unwrap();
        link_peer(&home, "telegram", "stranger", &PrincipalId::owner()).unwrap();
        assert_eq!(
            resolve(&home, "telegram", "stranger").unwrap(),
            PrincipalId::owner()
        );
        assert_eq!(Registry::load(&home).unwrap().peers.len(), 1);
        fs::remove_dir_all(&home).ok();
    }
}
