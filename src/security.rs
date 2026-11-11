//! The security substrate — armada's authorization layer (L3).
//!
//! The owner's example: a layer added to fleet that supports the armada
//! vision and exposes well-defined contracts armada consumes. It is
//! principal-agnostic from the first cut (D8): lane 1 has one principal (the
//! owner, holding a wildcard grant), and the org (lane 2) multiplies
//! principals and leaves this code unchanged.
//!
//! The model (R1–R5, D14–D19):
//! - A **grant** is `(principal, dimension, scope, action-class)` (D14). The
//!   dimensions and classes are closed sets; a scope is data, not code.
//! - The **uniform guard** checks the grant default-deny (D15): an action
//!   with no matching grant is refused by name. The owner's wildcard is the
//!   lane-1 bootstrap — nothing the owner does today is refused.
//! - **Step-up** (D18): a `destructive`/`governance` action needs fresh human
//!   proof. Lane 1's in-turn approval (hard rule 7) satisfies it; the signed
//!   [`Assertion`] is the org-scale form, verified here, issued by the
//!   flagship at L8. The agent holds no signing key and cannot forge it.
//! - **Governance** (D19): the grant store, guard config, roles, and org SOUL
//!   are immutable to the agent; grant-making is the root privilege.
//!
//! Authority is code-enforced (R2): a prompt is not a security boundary. The
//! advisory `judge_destructive` sys1 explanation never grants.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::filelock::FileLock;

/// The grant store's path inside a home (D14).
pub const GRANTS_FILE: &str = "grants.json";

/// The freshness window for an assertion, in seconds (D18). Per-action, not
/// a blanket "2FA done, act freely" window.
pub const ASSERTION_WINDOW_SECS: u64 = 120;

/// The layer an action touches — the closed set R4 names (D14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dimension {
    Project,
    Storage,
    Resource,
    Corpus,
    Tool,
    Skill,
    Workflow,
    Loop,
    Governance,
}

/// The verb tier (D14). `destructive` and `governance` are the step-up tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionClass {
    Read,
    Write,
    Dispatch,
    Destructive,
    Governance,
}

impl ActionClass {
    /// Whether this class demands fresh human proof (R5, D18).
    pub fn needs_step_up(self) -> bool {
        matches!(self, ActionClass::Destructive | ActionClass::Governance)
    }
}

/// A grant: `(principal, dimension, scope, action-class)` (D14). A `None`
/// field is a wildcard — the owner's bootstrap grant is all wildcards. A
/// scope is data (`instance`, `project:<id>`, `team:<id>`, `org`), not code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    /// Who the grant is for.
    pub principal: String,
    /// The layer, or `None` for any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension: Option<Dimension>,
    /// The reach, or `None` for any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The verb tier, or `None` for any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<ActionClass>,
}

impl Grant {
    /// The owner's wildcard bootstrap grant (lane 1, D15).
    pub fn owner_wildcard() -> Grant {
        Grant {
            principal: crate::identity::OWNER.to_string(),
            dimension: None,
            scope: None,
            class: None,
        }
    }

    /// The folder grant as a row (D14): `(owner, project, project:<id>,
    /// write)` — the least-privilege seed, generalized from "no folder = no
    /// filesystem."
    pub fn folder(project_id: &str) -> Grant {
        Grant {
            principal: crate::identity::OWNER.to_string(),
            dimension: Some(Dimension::Project),
            scope: Some(format!("project:{project_id}")),
            class: Some(ActionClass::Write),
        }
    }

    /// Whether this grant covers the action.
    pub fn matches(
        &self,
        principal: &str,
        dimension: Dimension,
        scope: &str,
        class: ActionClass,
    ) -> bool {
        self.principal == principal
            && self.dimension.is_none_or(|d| d == dimension)
            && self.scope.as_deref().is_none_or(|s| s == scope)
            && self.class.is_none_or(|c| c == class)
    }
}

/// The grant store: one home, one document (D14). The governance layer —
/// immutable to the agent (D19); written only by the owner / control plane.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GrantStore {
    /// The explicit grants. The owner wildcard is the implicit bootstrap.
    #[serde(default)]
    pub grants: Vec<Grant>,
}

impl GrantStore {
    /// The store's path inside a home.
    pub fn path(home: &Path) -> PathBuf {
        home.join(GRANTS_FILE)
    }

    /// Read the store. A home with no grants yet is an empty store (the
    /// owner wildcard still bootstraps); a store that cannot be parsed is an
    /// error, because silently dropping grants would look like a revocation.
    pub fn load(home: &Path) -> io::Result<GrantStore> {
        let path = Self::path(home);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not readable: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(GrantStore::default()),
            Err(error) => Err(error),
        }
    }

    /// Write the store atomically and privately (0600), under the flock — a
    /// governance act (D19), never the agent's.
    pub fn save(&self, home: &Path) -> io::Result<()> {
        let path = Self::path(home);
        let _lock = FileLock::take(&FileLock::beside(&path));
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        write_private(&path, text.as_bytes())
    }
}

/// The guard's answer (D15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The principal holds a matching grant.
    Granted,
    /// Default-deny: no matching grant, refused by name.
    Denied { name: String },
    /// The grant is held but fresh human proof is missing (D18).
    StepUpRequired,
}

/// The uniform guard (D15). One vocabulary at every layer.
pub mod guard {
    use super::*;

    /// Check the grant default-deny. The owner's wildcard is the lane-1
    /// bootstrap (D15): nothing the owner does today is refused. Any other
    /// principal needs an explicit matching grant.
    pub fn check_grant(
        principal: &str,
        dimension: Dimension,
        scope: &str,
        class: ActionClass,
        store: &GrantStore,
    ) -> Decision {
        if principal == crate::identity::OWNER {
            return Decision::Granted;
        }
        for grant in &store.grants {
            if grant.matches(principal, dimension, scope, class) {
                return Decision::Granted;
            }
        }
        Decision::Denied {
            name: format!("no grant for `{principal}` on {dimension:?}/{scope}/{class:?}"),
        }
    }

    /// Check the step-up tier (D18): a `destructive`/`governance` action
    /// needs fresh human proof. Lane 1's in-turn approval satisfies it; a
    /// valid assertion satisfies it; neither yields `StepUpRequired`. A
    /// non-step-up class is always `Granted` here (the grant check governs).
    pub fn check_step_up(
        class: ActionClass,
        in_turn_approved: bool,
        assertion: Option<&Assertion>,
        public_key: Option<&[u8]>,
        now_secs: u64,
    ) -> Decision {
        if !class.needs_step_up() {
            return Decision::Granted;
        }
        if in_turn_approved {
            return Decision::Granted;
        }
        if let (Some(assertion), Some(key)) = (assertion, public_key) {
            if assertion.verify(key, now_secs) {
                return Decision::Granted;
            }
        }
        Decision::StepUpRequired
    }
}

/// A signed, per-action, fresh token (D18): the org-scale step-up proof. The
/// flagship signs it (private key); the instance verifies it (public key).
/// The agent holds no signing key and cannot forge it (R5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// The principal the assertion is for.
    pub principal: String,
    /// The verb tier it authorizes.
    pub class: ActionClass,
    /// The scope it authorizes.
    pub scope: String,
    /// The specific target (per-action, not blanket).
    pub target: String,
    /// When it was issued, seconds since the epoch.
    pub issued_at: u64,
    /// A fresh nonce.
    pub nonce: String,
    /// The Ed25519 signature over the canonical message, base64.
    pub signature: String,
}

impl Assertion {
    /// The canonical message the signature covers.
    pub fn signing_message(&self) -> String {
        format!(
            "{}|{:?}|{}|{}|{}|{}",
            self.principal, self.class, self.scope, self.target, self.issued_at, self.nonce
        )
    }

    /// Sign an assertion's fields with the flagship's Ed25519 key (L8 owns
    /// the key; this is the format + a test seam). Returns the assertion.
    pub fn sign(
        key_pair: &ring::signature::Ed25519KeyPair,
        principal: &str,
        class: ActionClass,
        scope: &str,
        target: &str,
        issued_at: u64,
        nonce: &str,
    ) -> Assertion {
        let mut assertion = Assertion {
            principal: principal.to_string(),
            class,
            scope: scope.to_string(),
            target: target.to_string(),
            issued_at,
            nonce: nonce.to_string(),
            signature: String::new(),
        };
        let message = assertion.signing_message();
        let signature = key_pair.sign(message.as_bytes());
        assertion.signature = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signature.as_ref(),
        );
        assertion
    }

    /// Verify the signature against the flagship public key, and the
    /// freshness window (D18). A forged, stale, or tampered assertion fails.
    pub fn verify(&self, public_key: &[u8], now_secs: u64) -> bool {
        // Freshness: per-action, short-lived.
        if now_secs < self.issued_at || now_secs - self.issued_at > ASSERTION_WINDOW_SECS {
            return false;
        }
        let Ok(signature) =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.signature)
        else {
            return false;
        };
        let message = self.signing_message();
        let key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key);
        key.verify(message.as_bytes(), &signature).is_ok()
    }
}

/// Whether a principal is admitted to a tool (D16) — the per-principal
/// declaration filter. The owner's wildcard admits every tool (lane 1,
/// unchanged); any other principal needs a tool grant whose scope names the
/// tool. An ungranted tool is not even declared.
pub fn admits_tool(principal: &str, tool: &str, store: &GrantStore) -> bool {
    if principal == crate::identity::OWNER {
        return true;
    }
    matches!(
        guard::check_grant(principal, Dimension::Tool, tool, ActionClass::Read, store),
        Decision::Granted
    ) || matches!(
        guard::check_grant(principal, Dimension::Tool, tool, ActionClass::Write, store),
        Decision::Granted
    )
}

/// Whether an action targets the governance layer (D19) — the grant store,
/// guard config, roles, org SOUL. Immutable to the agent; grant-making is the
/// root privilege. The agent is never declared a governance tool.
pub fn is_governance_target(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == GRANTS_FILE || name == "identity.json")
        .unwrap_or(false)
}

/// Write bytes atomically (temp + rename) and privately (0600).
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        use std::io::Write;
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.flush()?;
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
    use ring::signature::KeyPair as _;

    fn scratch() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fleet-security-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_key() -> ring::signature::Ed25519KeyPair {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
    }

    #[test]
    fn the_owner_wildcard_grants_everything() {
        let store = GrantStore::default();
        for class in [
            ActionClass::Read,
            ActionClass::Write,
            ActionClass::Dispatch,
            ActionClass::Destructive,
            ActionClass::Governance,
        ] {
            assert_eq!(
                guard::check_grant("owner", Dimension::Tool, "instance", class, &store),
                Decision::Granted
            );
        }
    }

    #[test]
    fn an_ungranted_principal_is_refused_by_name() {
        let store = GrantStore::default();
        let decision = guard::check_grant(
            "intruder",
            Dimension::Tool,
            "instance",
            ActionClass::Write,
            &store,
        );
        match decision {
            Decision::Denied { name } => assert!(name.contains("intruder"), "{name}"),
            other => panic!("expected default-deny, got {other:?}"),
        }
    }

    #[test]
    fn the_folder_grant_is_a_row_and_grants_its_principal() {
        let home = scratch();
        let mut store = GrantStore::default();
        store.grants.push(Grant::folder("demo"));
        store.save(&home).unwrap();
        let reloaded = GrantStore::load(&home).unwrap();
        // The folder grant is (owner, project, project:demo, write).
        let grant = &reloaded.grants[0];
        assert_eq!(grant.dimension, Some(Dimension::Project));
        assert_eq!(grant.scope.as_deref(), Some("project:demo"));
        assert_eq!(grant.class, Some(ActionClass::Write));
        // A non-owner principal granted the folder row matches it.
        let mut alice = Grant::folder("demo");
        alice.principal = "alice".to_string();
        let mut store2 = GrantStore::default();
        store2.grants.push(alice);
        assert_eq!(
            guard::check_grant(
                "alice",
                Dimension::Project,
                "project:demo",
                ActionClass::Write,
                &store2
            ),
            Decision::Granted
        );
        // But not another project.
        assert!(matches!(
            guard::check_grant(
                "alice",
                Dimension::Project,
                "project:other",
                ActionClass::Write,
                &store2
            ),
            Decision::Denied { .. }
        ));
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_principal_sees_only_the_tools_it_is_granted() {
        // The owner sees every tool (lane 1, unchanged).
        let empty = GrantStore::default();
        assert!(admits_tool("owner", "bash", &empty));
        assert!(admits_tool("owner", "write_file", &empty));
        // A non-owner with a grant for one tool sees only that tool.
        let mut store = GrantStore::default();
        store.grants.push(Grant {
            principal: "alice".to_string(),
            dimension: Some(Dimension::Tool),
            scope: Some("read_file".to_string()),
            class: Some(ActionClass::Read),
        });
        assert!(admits_tool("alice", "read_file", &store));
        assert!(!admits_tool("alice", "bash", &store));
        assert!(!admits_tool("alice", "write_file", &store));
        // A principal with no grants sees nothing.
        assert!(!admits_tool("bob", "read_file", &store));
    }

    #[test]
    fn step_up_accepts_in_turn_approval_or_a_valid_assertion_and_otherwise_requires_it() {
        let key = test_key();
        let public = key.public_key().as_ref().to_vec();
        let now = 1_000_000u64;
        let assertion = Assertion::sign(
            &key,
            "owner",
            ActionClass::Destructive,
            "instance",
            "delete-thing",
            now,
            "nonce-1",
        );
        // In-turn approval satisfies it (lane 1, unchanged).
        assert_eq!(
            guard::check_step_up(ActionClass::Destructive, true, None, None, now),
            Decision::Granted
        );
        // A valid assertion satisfies it (org scale).
        assert_eq!(
            guard::check_step_up(
                ActionClass::Destructive,
                false,
                Some(&assertion),
                Some(&public),
                now
            ),
            Decision::Granted
        );
        // Neither yields StepUpRequired.
        assert_eq!(
            guard::check_step_up(ActionClass::Destructive, false, None, None, now),
            Decision::StepUpRequired
        );
        // A non-step-up class never requires it.
        assert_eq!(
            guard::check_step_up(ActionClass::Read, false, None, None, now),
            Decision::Granted
        );
    }

    #[test]
    fn an_assertion_verifies_and_rejects_forged_stale_and_tampered() {
        let key = test_key();
        let public = key.public_key().as_ref().to_vec();
        let now = 1_000_000u64;
        let assertion = Assertion::sign(
            &key,
            "owner",
            ActionClass::Destructive,
            "instance",
            "delete-thing",
            now,
            "nonce-1",
        );
        assert!(assertion.verify(&public, now));
        // Stale: past the freshness window.
        assert!(!assertion.verify(&public, now + ASSERTION_WINDOW_SECS + 1));
        // Forged: a different key's signature.
        let other = test_key();
        let forged = Assertion::sign(
            &other,
            "owner",
            ActionClass::Destructive,
            "instance",
            "delete-thing",
            now,
            "nonce-1",
        );
        assert!(!forged.verify(&public, now));
        // Tampered: the target changed after signing.
        let mut tampered = assertion.clone();
        tampered.target = "delete-everything".to_string();
        assert!(!tampered.verify(&public, now));
    }

    #[test]
    fn the_governance_layer_is_named() {
        assert!(is_governance_target(Path::new("/home/grants.json")));
        assert!(is_governance_target(Path::new("/home/identity.json")));
        assert!(!is_governance_target(Path::new("/home/notes.md")));
        assert!(ActionClass::Governance.needs_step_up());
    }

    #[test]
    fn a_malformed_store_is_an_error_not_a_silent_reset() {
        let home = scratch();
        fs::write(GrantStore::path(&home), "{ not json").unwrap();
        assert!(GrantStore::load(&home).is_err());
        fs::remove_dir_all(&home).ok();
    }
}
