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

/// The identity registry: the principals this home knows and the
/// device→principal edges (D8). One store, one home — the communication
/// layer's identity graph (L5) extends this same store, never a second.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    /// The principals known to this home. Lane 1: just the owner.
    #[serde(default)]
    pub principals: Vec<PrincipalId>,
    /// The device→principal edges.
    #[serde(default)]
    pub devices: Vec<DeviceLink>,
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

/// The principal a run acts for. Lane 1: always the owner (D8). The guard
/// (L3) reads this; the run carries it as `AgentOptions.principal`, `None`
/// resolving here to the owner — the run this crate has always had.
pub fn principal_for(_home: &Path) -> PrincipalId {
    PrincipalId::owner()
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
}
