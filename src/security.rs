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

    /// The class's own name, for the surfaces and the audit line: the serde
    /// spelling every store and journal row already uses.
    pub fn as_str(self) -> &'static str {
        match self {
            ActionClass::Read => "read",
            ActionClass::Write => "write",
            ActionClass::Dispatch => "dispatch",
            ActionClass::Destructive => "destructive",
            ActionClass::Governance => "governance",
        }
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

/// The role templates (lane-2 O2, built 2026-11-19): a named set of grants
/// over the SAME `(principal, dimension, scope, class)` vocabulary every
/// other grant uses. A template is not a second permission system — it is a
/// recipe that expands into rows, so the guard that reads `grants.json`
/// needs no new code path.
///
/// The org roles the owner named map onto this store:
///
/// * `fleet-owner` — the bootstrap principal. It holds everything by the
///   owner wildcard and its template is therefore empty by construction:
///   materializing it would write rows the wildcard already covers, and
///   (more importantly) those rows could be edited away. The owner's
///   authority is not a template's to grant or revoke.
/// * `org-admin` — governs within a scope: `governance` and `dispatch`
///   there, so it can manage other principals and drive work. It
///   **cannot** grant or revoke the owner's authority, and it cannot
///   alter `fleet-owner`'s own grants: that check is
///   [`refuse_owner_tampering`], enforced in code (R3's actor-blind
///   boundary — the request's origin does not matter, only its effect).
/// * `operator` — `dispatch` + `read` on granted instances. Runs work,
///   reads state, changes nothing structural.
/// * `auditor` — `read` of state and the audit stream. Sees, touches
///   nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoleTemplate {
    /// The bootstrap principal: everything, by the wildcard (never
    /// materialized as rows).
    FleetOwner,
    /// Governs a scope: governance + dispatch there.
    OrgAdmin,
    /// Runs work: dispatch + read on granted instances.
    Operator,
    /// Sees: read of state and audit.
    Auditor,
}

impl RoleTemplate {
    /// Every template the build ships, in the order a picker shows them.
    pub const ALL: [RoleTemplate; 4] = [
        RoleTemplate::FleetOwner,
        RoleTemplate::OrgAdmin,
        RoleTemplate::Operator,
        RoleTemplate::Auditor,
    ];

    /// The template's stable name (its serde spelling).
    pub fn as_str(self) -> &'static str {
        match self {
            RoleTemplate::FleetOwner => "fleet-owner",
            RoleTemplate::OrgAdmin => "org-admin",
            RoleTemplate::Operator => "operator",
            RoleTemplate::Auditor => "auditor",
        }
    }

    /// Parse a template by name.
    pub fn parse(name: &str) -> Option<RoleTemplate> {
        match name.trim().to_lowercase().as_str() {
            "fleet-owner" => Some(RoleTemplate::FleetOwner),
            "org-admin" => Some(RoleTemplate::OrgAdmin),
            "operator" => Some(RoleTemplate::Operator),
            "auditor" => Some(RoleTemplate::Auditor),
            _ => None,
        }
    }

    /// One line for the picker: what the template lets a principal do.
    pub fn description(self) -> &'static str {
        match self {
            RoleTemplate::FleetOwner => {
                "everything, by the owner wildcard — not granted, and not revocable"
            }
            RoleTemplate::OrgAdmin => {
                "governs a scope: grants within it, and dispatches work there"
            }
            RoleTemplate::Operator => "dispatches work to granted instances, and reads their state",
            RoleTemplate::Auditor => "reads state and the audit stream, and changes nothing",
        }
    }

    /// Expand the template into grant rows for one principal at one scope.
    ///
    /// `scope` is data (`instance`, `project:<id>`, `team:<id>`, `org`), as
    /// D14 requires. The owner template expands to NOTHING: its authority is
    /// the wildcard in `guard::check_grant`, which no row can revoke.
    pub fn grants(self, principal: &str, scope: &str) -> Vec<Grant> {
        let row = |dimension: Option<Dimension>, class: Option<ActionClass>| Grant {
            principal: principal.to_string(),
            dimension,
            scope: Some(scope.to_string()),
            class,
        };
        match self {
            RoleTemplate::FleetOwner => Vec::new(),
            RoleTemplate::OrgAdmin => vec![
                row(Some(Dimension::Governance), Some(ActionClass::Governance)),
                row(Some(Dimension::Workflow), Some(ActionClass::Dispatch)),
                row(Some(Dimension::Resource), Some(ActionClass::Read)),
            ],
            RoleTemplate::Operator => vec![
                row(Some(Dimension::Workflow), Some(ActionClass::Dispatch)),
                row(Some(Dimension::Resource), Some(ActionClass::Read)),
            ],
            RoleTemplate::Auditor => vec![row(None, Some(ActionClass::Read))],
        }
    }
}

/// The governance boundary (R3, D19): the owner's authority is not a
/// template's to move. Refused in CODE, before any write, because a rule
/// that lived only in prose would be the "polite fiction" R2 warns about.
///
/// What it refuses:
///
/// * granting or revoking anything for the `owner` principal;
/// * granting or revoking the `fleet-owner` template for anyone (it is the
///   owner's role; `fleet-owner` on a second principal would be a second
///   owner, which the wildcard path does not even model).
///
/// It deliberately does NOT refuse an org-admin acting within its own
/// scope on other principals — that is the job.
pub fn refuse_owner_tampering(
    principal: &str,
    template: Option<RoleTemplate>,
    op: &str,
) -> Result<(), String> {
    if principal.trim() == crate::identity::OWNER {
        return Err(format!(
            "refused: {op} for the owner principal — the owner's authority is the bootstrap \
             wildcard, and no governance act can grant or revoke it"
        ));
    }
    if template == Some(RoleTemplate::FleetOwner) {
        return Err(format!(
            "refused: {op} of the `fleet-owner` template — that is the owner's role, and a \
             second holder would be a second owner"
        ));
    }
    Ok(())
}

/// A governance act's outcome, for the surface and the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceAct {
    /// What happened: `granted` or `revoked`.
    pub what: &'static str,
    /// The principal whose authority changed.
    pub principal: String,
    /// The template applied.
    pub template: RoleTemplate,
    /// The scope it was applied at.
    pub scope: String,
    /// How many grant rows the act added or removed.
    pub rows: usize,
}

impl GovernanceAct {
    /// One line for the audit stream and the console.
    pub fn note(&self) -> String {
        format!(
            "{} {} ({}) at {} — {} grant row{}",
            self.what,
            self.template.as_str(),
            self.principal,
            self.scope,
            self.rows,
            if self.rows == 1 { "" } else { "s" }
        )
    }
}

/// Apply a role template to a principal at a scope: a governance act.
///
/// The order is the whole contract: the boundary
/// ([`refuse_owner_tampering`]) runs **before** any read of the store, so a
/// refused act cannot partially write; then the template expands; then the
/// store is written under its flock by [`GrantStore::save`]. The caller
/// journals the [`GovernanceAct`] — this function does not append, so the
/// store and the journal each keep one writer.
pub fn grant_template(
    home: &Path,
    principal: &str,
    template: RoleTemplate,
    scope: &str,
    revoke: bool,
) -> Result<GovernanceAct, String> {
    let what = if revoke { "revoke" } else { "grant" };
    refuse_owner_tampering(principal, Some(template), what)?;
    if principal.trim().is_empty() {
        return Err(format!("refused: {what} names no principal"));
    }
    if scope.trim().is_empty() {
        return Err(format!("refused: {what} names no scope"));
    }
    let mut store =
        GrantStore::load(home).map_err(|error| format!("cannot read the grant store: {error}"))?;
    let expanded = template.grants(principal, scope);
    if expanded.is_empty() {
        return Err(format!(
            "refused: `{}` expands to no grant rows — the owner's authority is the bootstrap \
             wildcard, not a template",
            template.as_str()
        ));
    }
    let before = store.grants.len();
    if revoke {
        store
            .grants
            .retain(|held| !expanded.iter().any(|row| same_row(held, row)));
    } else {
        for row in &expanded {
            if !store.grants.iter().any(|held| same_row(held, row)) {
                store.grants.push(row.clone());
            }
        }
    }
    let rows = if revoke {
        before - store.grants.len()
    } else {
        store.grants.len() - before
    };
    store
        .save(home)
        .map_err(|error| format!("cannot write the grant store: {error}"))?;
    Ok(GovernanceAct {
        what,
        principal: principal.to_string(),
        template,
        scope: scope.to_string(),
        rows,
    })
}

/// Two rows are the same grant when all four fields match — the identity the
/// guard's `matches` reads, minus the wildcard semantics: removal must be
/// exact, never a looser match that takes a neighbouring row with it.
fn same_row(left: &Grant, right: &Grant) -> bool {
    left.principal == right.principal
        && left.dimension == right.dimension
        && left.scope == right.scope
        && left.class == right.class
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
    #[test]
    fn a_template_expands_into_rows_the_guard_reads() {
        let home = scratch();
        let act = grant_template(&home, "alice", RoleTemplate::Operator, "instance", false)
            .expect("granted");
        assert_eq!(act.rows, 2, "operator is dispatch + read: {}", act.note());
        let store = GrantStore::load(&home).unwrap();
        // The guard reads the expanded rows with no new code path.
        assert_eq!(
            guard::check_grant(
                "alice",
                Dimension::Workflow,
                "instance",
                ActionClass::Dispatch,
                &store
            ),
            Decision::Granted
        );
        assert_eq!(
            guard::check_grant(
                "alice",
                Dimension::Workflow,
                "instance",
                ActionClass::Destructive,
                &store
            ),
            Decision::Denied {
                name: "no grant for `alice` on Workflow/instance/Destructive".to_string()
            }
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_grant_is_idempotent_and_a_revoke_removes_exactly_those_rows() {
        let home = scratch();
        let first = grant_template(&home, "alice", RoleTemplate::Auditor, "org", false).unwrap();
        assert_eq!(first.rows, 1);
        let again = grant_template(&home, "alice", RoleTemplate::Auditor, "org", false).unwrap();
        assert_eq!(
            again.rows, 0,
            "a second grant of the same template adds nothing"
        );
        let gone = grant_template(&home, "alice", RoleTemplate::Auditor, "org", true).unwrap();
        assert_eq!(gone.rows, 1, "the revoke removed the row it granted");
        let store = GrantStore::load(&home).unwrap();
        assert!(store.grants.is_empty(), "{:?}", store.grants);
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_owner_is_not_governable_by_any_act() {
        let home = scratch();
        for revoke in [false, true] {
            let refused = grant_template(&home, "owner", RoleTemplate::OrgAdmin, "org", revoke)
                .expect_err("the owner is refused");
            assert!(
                refused.contains("the owner's authority is the bootstrap wildcard"),
                "refusal names the rule: {refused}"
            );
        }
        // Nothing was written by either refused act.
        let store = GrantStore::load(&home).unwrap();
        assert!(
            store.grants.is_empty(),
            "a refused act wrote rows: {:?}",
            store.grants
        );
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_second_fleet_owner_is_refused_by_name() {
        let home = scratch();
        let refused = grant_template(&home, "alice", RoleTemplate::FleetOwner, "org", false)
            .expect_err("refused");
        assert!(
            refused.contains("a second holder would be a second owner"),
            "refusal names the consequence: {refused}"
        );
        // The owner template expands to nothing even if the boundary were
        // bypassed: the wildcard is not a row.
        assert!(RoleTemplate::FleetOwner.grants("alice", "org").is_empty());
        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn the_boundary_refuses_before_reading_the_store() {
        // A corrupt store proves the order: a boundary refusal must not
        // even parse it, so a tampering attempt on a broken home is still
        // the boundary's answer rather than a parse error.
        let home = scratch();
        fs::write(GrantStore::path(&home), "{ not json").unwrap();
        let refused = grant_template(&home, "owner", RoleTemplate::Operator, "instance", false)
            .expect_err("the boundary answers first");
        assert!(
            refused.contains("no governance act can grant or revoke"),
            "{refused}"
        );
        assert!(
            !refused.contains("cannot read the grant store"),
            "{refused}"
        );
        fs::remove_dir_all(&home).ok();
    }
}
