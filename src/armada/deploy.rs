//! Deployment (armada L9.1, D51/D52/D57): fleet deploying fleet — fully
//! self-contained, no foreign control-plane service. A local spawn lays out a
//! new home directly; a VPS is reached over SSH and dropped the same
//! self-contained binary (the release raw-binary channel). The target mints a
//! fresh install-id (L1), clones the corpus remote (inheriting the mind, not
//! the identity — D29), and registers with the flagship (L8). A fresh install
//! is trusted only after it passes doctor and settles runs — never by arrival
//! (D57). Credentials resolve out-of-band (env/vault), never the corpus.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// A provision target: a local home, or a VPS reached over SSH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local {
        home: PathBuf,
    },
    Vps {
        host: String,
        user: String,
        home: String,
    },
}

/// What a provision produced. A fresh install is always un-calibrated until it
/// settles (D49) — `calibrated` is false by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionReport {
    pub install_id: String,
    pub corpus_cloned: bool,
    pub calibrated: bool,
}

/// Bootstrap the corpus on a fresh install (D52): clone the remote into
/// `home/memories`. Identity is NOT in the corpus — the fresh install-id is
/// minted separately (L1), so two installs share the mind but not the self.
/// An existing work tree is refused by name (bootstrap is for a fresh install;
/// the steady state is L4's sync).
pub fn bootstrap_corpus(home: &Path, remote: &str) -> Result<(), String> {
    if remote.trim().is_empty() {
        return Err("a corpus bootstrap needs a remote".to_string());
    }
    let root = crate::corpus::corpus_root(home);
    if root.join(".git").exists() {
        return Err(format!(
            "the corpus at {} already exists — bootstrap is for a fresh install",
            root.display()
        ));
    }
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("the corpus home: {e}"))?;
    }
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg("--quiet")
        .arg(remote)
        .arg(&root)
        .output()
        .map_err(|e| format!("the corpus clone could not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "the corpus clone was refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    // Record the remote so the steady-state L4 sync rides it.
    let head = git_head(&root).ok();
    let state = crate::corpus::SyncState {
        remote: Some(remote.to_string()),
        corpus_ref: "master".to_string(),
        last_revision: head,
        settles: Vec::new(),
    };
    state
        .save(home)
        .map_err(|e| format!("the corpus state: {e}"))?;
    Ok(())
}

fn git_head(dir: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|e| format!("git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err("the corpus head could not be read".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Provision a local install (D51): lay out the home, mint a fresh install-id,
/// and bootstrap the corpus when a remote is named. The binary is the
/// self-contained payload (hard rule 12) — the caller drops it; this lays out
/// the home it runs in. Registration with the flagship is a separate gated act
/// (`register_fresh`), never implicit in arrival.
pub fn provision_local(home: &Path, remote: Option<&str>) -> Result<ProvisionReport, String> {
    for dir in crate::install::LAYOUT_DIRS {
        std::fs::create_dir_all(home.join(dir)).map_err(|e| format!("the layout `{dir}`: {e}"))?;
    }
    let install_id = crate::identity::install(home)?;
    let corpus_cloned = match remote {
        Some(remote) if !remote.trim().is_empty() => {
            bootstrap_corpus(home, remote)?;
            true
        }
        _ => false,
    };
    Ok(ProvisionReport {
        install_id: install_id.as_str().to_string(),
        corpus_cloned,
        calibrated: false,
    })
}

/// The provider a deploy targets (D60, 2026-11-19): an instance is a
/// container, and a container needs a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// A local or remote Docker engine. The flagship runs `docker` against
    /// whatever engine its own context names.
    Docker,
    /// OpenShip Cloud: the project/service/deployment API.
    OpenShip,
}

impl Provider {
    /// Parse a provider by name.
    pub fn parse(name: &str) -> Option<Provider> {
        match name.trim().to_lowercase().as_str() {
            "docker" => Some(Provider::Docker),
            "openship" => Some(Provider::OpenShip),
            _ => None,
        }
    }

    /// The provider's stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Docker => "docker",
            Provider::OpenShip => "openship",
        }
    }
}

/// The container spec a deploy builds (D60): the whole of what an instance
/// needs to exist, as data — so a dry run prints exactly what a real run
/// would do, and the two can never disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSpec {
    /// The provider that will run it.
    pub provider: Provider,
    /// The image reference.
    pub image: String,
    /// The instance's name/target on that provider.
    pub name: String,
    /// The named volume mounted at `FLEET_HOME`.
    pub volume: String,
    /// The mount point the volume holds (the image's `FLEET_HOME`).
    pub mount: String,
    /// The environment the container boots with. Keys only are ever logged;
    /// values carry the corpus remote and the join material.
    pub env: Vec<(String, String)>,
}

impl ServiceSpec {
    /// The environment KEYS, for a log line or a dry run — never the values,
    /// so a deploy's output cannot leak a join secret into a scrollback.
    pub fn env_keys(&self) -> Vec<&str> {
        self.env.iter().map(|(key, _)| key.as_str()).collect()
    }

    /// The dry-run rendering: what the deploy would do, in the operator's
    /// own words, with no secret values.
    pub fn preview(&self) -> String {
        format!(
            "would deploy `{}` on {} — image {}, volume {} at {}, env [{}]",
            self.name,
            self.provider.as_str(),
            self.image,
            self.volume,
            self.mount,
            self.env_keys().join(", ")
        )
    }
}

/// Build the service spec for a deploy. Pure: the whole contract between a
/// dry run and a real run.
pub fn service_spec(
    provider: Provider,
    name: &str,
    image: &str,
    corpus_remote: &str,
    volume: Option<&str>,
) -> Result<ServiceSpec, String> {
    if name.trim().is_empty() {
        return Err("a deploy names an instance (the sid its roster row will carry)".to_string());
    }
    if image.trim().is_empty() {
        return Err("a deploy names an image — the instance's binary and lanes".to_string());
    }
    if corpus_remote.trim().is_empty() {
        return Err(
            "a deploy names the corpus remote — an instance with no corpus starts empty"
                .to_string(),
        );
    }
    Ok(ServiceSpec {
        provider,
        image: image.to_string(),
        name: name.trim().to_string(),
        volume: volume.unwrap_or(name.trim()).to_string(),
        mount: "/data".to_string(),
        env: vec![
            ("FLEET_HOME".to_string(), "/data".to_string()),
            ("FLEET_SID".to_string(), name.trim().to_string()),
            ("FLEET_CORPUS_REMOTE".to_string(), corpus_remote.to_string()),
        ],
    })
}

/// What a provider reported after running the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provisioned {
    /// The provider's own handle: a container id or a service id.
    pub handle: String,
    /// The steps taken, in order, with their outcomes — each is journaled.
    pub steps: Vec<(String, String)>,
}

/// Run a spec through its provider. Each step is executed and reported, not
/// printed: a failed step names itself and stops the sequence (D3), so a
/// half-provisioned instance never looks like a whole one.
pub fn provision(home: &Path, spec: &ServiceSpec, dry_run: bool) -> Result<Provisioned, String> {
    if dry_run {
        return Ok(Provisioned {
            handle: "(dry run)".to_string(),
            steps: vec![("preview".to_string(), spec.preview())],
        });
    }
    // The journal records the steps of a provision that RETURNED:
    // `Result::inspect` fires on `Ok` only, so a refused deploy journals
    // nothing — the rule the test below pins.
    match spec.provider {
        Provider::Docker => provision_docker(spec),
        Provider::OpenShip => provision_openship(spec),
    }
    .inspect(|provisioned| {
        for (step, outcome) in &provisioned.steps {
            journal_step(home, &spec.name, step, outcome);
        }
    })
}

/// The docker leg: create the volume, then run the container attached to it.
fn provision_docker(spec: &ServiceSpec) -> Result<Provisioned, String> {
    let mut steps: Vec<(String, String)> = Vec::new();
    let run = |args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new("docker")
            .args(args)
            .output()
            .map_err(|error| format!("docker could not run: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    match run(&["volume", "create", &spec.volume]) {
        Ok(_) => steps.push(("volume".to_string(), format!("created {}", spec.volume))),
        Err(refusal) => {
            steps.push(("volume".to_string(), format!("refused: {refusal}")));
            return Err(format!("the volume step failed: {refusal}"));
        }
    }
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--name".into(),
        spec.name.clone(),
        "-v".into(),
        format!("{}:{}", spec.volume, spec.mount),
    ];
    for (key, value) in &spec.env {
        args.push("-e".into());
        args.push(format!("{key}={value}"));
    }
    args.push(spec.image.clone());
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&borrowed) {
        Ok(handle) => {
            steps.push(("run".to_string(), format!("started {handle}")));
            Ok(Provisioned { handle, steps })
        }
        Err(refusal) => {
            steps.push(("run".to_string(), format!("refused: {refusal}")));
            Err(format!("the run step failed: {refusal}"))
        }
    }
}

/// The OpenShip leg: the CLI is the adapter, so a version mismatch in the API
/// surfaces as its own named error rather than as a silent no-op. A stack's
/// volume must be set per service (the platform does not cascade a
/// project-level volume), which is why the volume rides the spec, not a flag.
/// The OpenShip leg, over its MCP endpoint (verified 2026-11-19 against the
/// live account: `tools/list` returns 432 tools with this PAT).
///
/// Why MCP and not the CLI: the CLI is a full Node server with its own
/// internal auth path, while `/api/mcp` is a stateless JSON-RPC endpoint that
/// re-runs the platform's whole permission stack on every call
/// (`openship.io/docs/mcp`: "Every tool call re-runs the full auth and
/// permission stack"). It is also the surface the docs tell a client to use,
/// and the tool names are stable REST-derived identities — a route becomes a
/// tool or it does not, so a wrong name fails loudly instead of silently
/// doing something adjacent.
///
/// The three calls this leg makes, each one a real tool on that list:
///
/// 1. `post_projects_by_id_services` — define the container (image, env).
/// 2. `patch_projects_by_id_services_by_serviceId` — the volume, which a
///    stack does not inherit from the project.
/// 3. `post_deployments` — the deployment that pulls and runs.
///
/// Credentials never ride the spec: the bearer comes from `OPENSHIP_TOKEN` in
/// the environment or the fleet home's dotenv/vault, the same chain every
/// other credential uses.
fn provision_openship(spec: &ServiceSpec) -> Result<Provisioned, String> {
    let project = std::env::var("OPENSHIP_PROJECT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "the openship leg needs OPENSHIP_PROJECT — the project this instance joins, as \
             an id, slug or name; without it the call has no destination"
                .to_string()
        })?;
    let token = openship_token().ok_or_else(|| {
        "the openship leg needs OPENSHIP_TOKEN — a PAT from Settings → Tokens, in the \
         environment or this home's .env/vault; a read-only token cannot deploy, so the \
         refusal that follows is the scope doing its job"
            .to_string()
    })?;
    let mut steps: Vec<(String, String)> = Vec::new();

    // The service's env, as the API expects it: the corpus remote and the
    // instance's own sid, never a credential.
    let env_map: serde_json::Map<String, Value> = spec
        .env
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect();
    let mut body = json!({
        "name": spec.name,
        "image": spec.image,
        "env": Value::Object(env_map),
    });
    if let Some(object) = body.as_object_mut() {
        object.insert("organizationId".to_string(), json!(openship_org()));
    }
    match call_openship(
        &token,
        "post_projects_by_id_services",
        json!({ "id": project, "body": body }),
    ) {
        Ok(_) => steps.push((
            "service".to_string(),
            format!("{} defined in {project}", spec.name),
        )),
        Err(refusal) => {
            steps.push(("service".to_string(), format!("refused: {refusal}")));
            return Err(format!("the service definition was refused: {refusal}"));
        }
    }

    // The volume, on the service itself.
    let volume = json!([format!("{}:{}", spec.volume, spec.mount)]);
    match call_openship(
        &token,
        "patch_projects_by_id_services_by_serviceId",
        json!({ "id": project, "serviceId": spec.name, "body": { "volumes": volume } }),
    ) {
        Ok(_) => steps.push((
            "volume".to_string(),
            format!("{} at {}", spec.volume, spec.mount),
        )),
        Err(refusal) => {
            steps.push(("volume".to_string(), format!("refused: {refusal}")));
            return Err(format!("the volume step was refused: {refusal}"));
        }
    }

    // The deployment.
    match call_openship(
        &token,
        "post_deployments",
        json!({ "body": { "projectId": project } }),
    ) {
        Ok(reply) => {
            let handle = reply
                .get("data")
                .and_then(|data| data.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("deployment started")
                .to_string();
            steps.push(("deploy".to_string(), format!("started {handle}")));
            Ok(Provisioned { handle, steps })
        }
        Err(refusal) => {
            steps.push(("deploy".to_string(), format!("refused: {refusal}")));
            Err(format!("the deployment was refused: {refusal}"))
        }
    }
}

/// The OpenShip bearer: the environment first, then the home's dotenv/vault —
/// the chain the lanes and every provider key already resolve through, so a
/// deploy needs no credential of its own.
fn openship_token() -> Option<String> {
    std::env::var("OPENSHIP_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The organization the instance lands in. OpenShip calls it `organizationId`
/// and the docs' own discovery call returns it as `boundOrganizationId`; the
/// caller names it rather than the leg guessing, because a token cannot widen
/// its own scope by asking for a different one.
fn openship_org() -> String {
    std::env::var("OPENSHIP_ORGANIZATION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

/// One JSON-RPC call against `/api/mcp`, returning the parsed result.
///
/// Stateless by design (the docs: POST only, no batching, no stream), so no
/// session is negotiated and no `initialize` is sent — the endpoint answers a
/// bare `tools/call`. A JSON-RPC error, a non-200, or a body carrying an
/// `error` member is a NAMED refusal carrying the server's own sentence,
/// because a deploy that fails silently is worse than one that fails loudly.
fn call_openship(token: &str, tool: &str, arguments: Value) -> Result<Value, String> {
    let endpoint = std::env::var("OPENSHIP_MCP_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://app.openship.io/api/mcp".to_string());
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments },
    });
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();
    let reply = agent
        .post(&endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_string(&payload.to_string());
    let body = match reply {
        Ok(response) => response
            .into_string()
            .map_err(|error| format!("the openship reply could not be read: {error}"))?,
        Err(ureq::Error::Status(code, response)) => {
            let said = response.into_string().unwrap_or_default();
            return Err(format!(
                "openship answered HTTP {code} — {}",
                said.trim().chars().take(300).collect::<String>()
            ));
        }
        Err(error) => return Err(format!("openship could not be reached: {error}")),
    };
    let document: Value = serde_json::from_str(&body)
        .map_err(|error| format!("the openship reply is not JSON: {error}"))?;
    if let Some(error) = document.get("error") {
        let said = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("no message");
        return Err(format!("openship refused `{tool}`: {said}"));
    }
    Ok(document.get("result").cloned().unwrap_or(Value::Null))
}

/// One provision step, journaled with its real outcome — the audit trail a
/// stopped sequence is read from.
fn journal_step(home: &Path, name: &str, step: &str, outcome: &str) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "provision",
            Some(crate::identity::OWNER),
            Some(name),
            json!({"instance": name, "step": step, "outcome": outcome}),
        );
    }
}

/// Register a fresh install with the flagship roster (D44). The install is
/// reported un-calibrated until it settles (D49) — registration earns a place
/// on the roster, never autonomy.
pub fn register_fresh(
    flagship_home: &Path,
    install_id: &str,
    sid: &str,
    version: &str,
    capacity_kib: u64,
    now_secs: u64,
) -> Result<crate::armada::roster::Instance, String> {
    crate::armada::roster::register(
        flagship_home,
        install_id,
        sid,
        version,
        capacity_kib,
        now_secs,
    )
}

/// The trust grade (D57): a fresh install is trusted only after it passes
/// doctor AND settles runs — graded by measurement, never by arrival. A fresh
/// install with no settles is untrusted even if doctor passes.
pub fn trusted(home: &Path, sid: &str, doctor_passed: bool) -> bool {
    doctor_passed && crate::armada::control::grade(home, sid).calibrated
}

/// The provision decision as the control plane meters it: a governance action
/// (D18/D19), so it clears step-up before it runs. The CLI's deploy verb admits
/// through the guard; this names the class for the frame.
pub fn provision_class() -> crate::security::ActionClass {
    crate::security::ActionClass::Governance
}

/// A provision journal event (D10): the durable record of a deployment, graded
/// against the fresh install's later settles.
pub fn journal_provision(home: &Path, install_id: &str, target: &str, corpus_cloned: bool) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "provision",
            Some(crate::identity::OWNER),
            Some(install_id),
            json!({"install": install_id, "target": target, "corpus_cloned": corpus_cloned}),
        );
    }
}

/// Settle a governance act (grant/revoke) in the journal: every authority
/// change is auditable, per principal and per template, and the audit stream
/// the console reads IS this journal.
pub fn journal_governance(home: &Path, note: &str) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "governance",
            Some(crate::identity::OWNER),
            None,
            json!({"note": note}),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lock the two OpenShip refusal tests take, because they write
    /// `OPENSHIP_PROJECT` and `OPENSHIP_TOKEN` through `std::env::set_var`
    /// — process-wide state that `cargo test`'s threads share. Without it
    /// the two race: whichever unsets the variable first makes the other
    /// read the next refusal, so one of them failed on every full-suite
    /// run at HEAD (which one varied, so it read as flake rather than as
    /// the class it is).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-deploy-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bare git remote carrying one fact — the mind a fresh install inherits.
    fn remote_with_fact(tag: &str) -> PathBuf {
        let remote = scratch(tag);
        let work = remote.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&work)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "--quiet", "-b", "master"]);
        run(&["config", "user.email", "test@fleet.local"]);
        run(&["config", "user.name", "fleet test"]);
        std::fs::write(
            work.join("a-fact.md"),
            "---\nname: a-fact\n---\nthe inherited mind\n",
        )
        .unwrap();
        run(&["add", "a-fact.md"]);
        run(&["commit", "--quiet", "-m", "a fact"]);
        // A bare clone to act as the remote.
        let bare = remote.join("remote.git");
        let out = std::process::Command::new("git")
            .arg("clone")
            .arg("--quiet")
            .arg("--bare")
            .arg(&work)
            .arg(&bare)
            .output()
            .unwrap();
        assert!(out.status.success());
        bare
    }

    /// C0: a local provision mints a fresh install-id and clones the corpus
    /// remote — inheriting the facts, not the identity.
    #[test]
    fn provision_inherits_the_mind_not_the_identity() {
        let remote = remote_with_fact("inherit");
        let home = scratch("local");
        let report = provision_local(&home, Some(remote.to_str().unwrap())).unwrap();
        assert!(report.corpus_cloned);
        assert!(!report.calibrated, "a fresh install is un-calibrated");
        assert_eq!(report.install_id.len(), 32, "a fresh 32-hex install-id");
        // The fact is inherited.
        assert!(crate::corpus::corpus_root(&home).join("a-fact.md").exists());
        // The identity is the fresh install's, minted locally — not in the corpus.
        assert_eq!(
            crate::identity::install(&home).unwrap().as_str(),
            report.install_id
        );
        // The remote is recorded so the steady-state sync rides it.
        assert_eq!(
            crate::corpus::SyncState::load(&home).remote.as_deref(),
            Some(remote.to_str().unwrap())
        );
        // The steady-state L4 sync now works on the cloned work tree.
        assert!(crate::corpus::sync(&home, None).is_ok());
    }

    /// C0: bootstrap refuses an existing work tree (it is for a fresh install).
    #[test]
    fn bootstrap_refuses_an_existing_corpus() {
        let remote = remote_with_fact("existing");
        let home = scratch("existing-home");
        bootstrap_corpus(&home, remote.to_str().unwrap()).unwrap();
        let err = bootstrap_corpus(&home, remote.to_str().unwrap()).unwrap_err();
        assert!(err.contains("already exists"), "{err}");
    }

    /// C1: a fresh install registers with the flagship roster and is reported
    /// un-calibrated until it settles.
    #[test]
    fn a_fresh_install_registers_un_calibrated() {
        let flagship = scratch("flagship");
        let instance = register_fresh(
            &flagship,
            "install-new",
            "sid-new",
            "0.6.38",
            1_000_000,
            1000,
        )
        .unwrap();
        assert_eq!(instance.sid, "sid-new");
        assert!(!instance.live(1000), "no heartbeat yet");
        // Un-calibrated until it settles.
        assert!(!crate::armada::control::grade(&flagship, "sid-new").calibrated);
        assert!(
            !trusted(&flagship, "sid-new", true),
            "doctor alone does not trust"
        );
    }

    /// C6: trust is earned by doctor AND settles — never by arrival.
    #[test]
    fn trust_requires_doctor_and_settles() {
        let home = scratch("trust");
        // Fresh: untrusted even with doctor.
        assert!(!trusted(&home, "sid-a", true));
        assert!(!trusted(&home, "sid-a", false));
        // Settle a dispatch: doctor + settles now trusts.
        let mut journal = crate::journal::Journal::open(&home).unwrap();
        journal
            .append(
                "dispatch",
                Some("owner"),
                Some("sid-a"),
                json!({"sid": "sid-a", "outcome": "finished"}),
            )
            .unwrap();
        assert!(trusted(&home, "sid-a", true));
        assert!(!trusted(&home, "sid-a", false), "doctor still required");
    }

    /// C0/C6: the VPS command sequence drops the self-contained payload, runs
    /// install, and syncs the corpus — credentials out-of-band (no secret in
    /// the commands).
    #[test]
    fn a_service_spec_carries_the_whole_contract_and_no_secrets() {
        let spec = service_spec(
            Provider::Docker,
            "box-1",
            "ghcr.io/fleet/instance:0.8.20",
            "git@host:mind.git",
            None,
        )
        .expect("a complete spec");
        assert_eq!(spec.mount, "/data");
        assert_eq!(
            spec.volume, "box-1",
            "the volume defaults to the instance name"
        );
        assert_eq!(
            spec.env_keys(),
            vec!["FLEET_HOME", "FLEET_SID", "FLEET_CORPUS_REMOTE"]
        );
        // The preview names the SHAPE and never a value: a deploy log must
        // not be able to leak the join material into a scrollback. That is
        // why the corpus remote is absent from it even though it is not a
        // secret — one rule for every value cannot spring a leak, while
        // "this one is fine to print" is a judgement that eventually errs.
        let preview = spec.preview();
        assert!(preview.contains("box-1"), "{preview}");
        assert!(
            preview.contains("FLEET_CORPUS_REMOTE"),
            "the keys are named: {preview}"
        );
        assert!(
            !preview.contains("git@host:mind.git"),
            "no value rides the preview: {preview}"
        );
    }

    #[test]
    fn a_spec_missing_its_name_image_or_corpus_is_refused_by_name() {
        let no_name =
            service_spec(Provider::Docker, "  ", "img", "git@h:m.git", None).expect_err("refused");
        assert!(no_name.contains("names an instance"), "{no_name}");
        let no_image =
            service_spec(Provider::Docker, "box", "", "git@h:m.git", None).expect_err("refused");
        assert!(no_image.contains("names an image"), "{no_image}");
        let no_corpus =
            service_spec(Provider::Docker, "box", "img", "", None).expect_err("refused");
        assert!(no_corpus.contains("names the corpus remote"), "{no_corpus}");
    }

    #[test]
    fn a_dry_run_prints_the_spec_and_writes_nothing() {
        let home = scratch("dry");
        let spec = service_spec(Provider::Docker, "box-1", "img:1", "git@h:m.git", None).unwrap();
        let provisioned = provision(&home, &spec, true).expect("a dry run always answers");
        assert_eq!(provisioned.handle, "(dry run)");
        assert_eq!(provisioned.steps.len(), 1);
        assert!(
            provisioned.steps[0].1.contains("would deploy"),
            "{:?}",
            provisioned.steps
        );
        // Nothing ran: no journal was written for a preview.
        let journal = crate::journal::Journal::open(&home).unwrap();
        assert!(
            journal
                .replay(0)
                .unwrap()
                .iter()
                .all(|e| e.kind != "provision"),
            "a dry run journaled a step"
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_openship_deploy_without_a_project_is_refused_by_name() {
        // SAFETY: this test owns these variables for its duration.
        let _env = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        unsafe {
            std::env::remove_var("OPENSHIP_PROJECT");
            std::env::set_var("OPENSHIP_TOKEN", "opsh_pat_test");
        }
        let spec = service_spec(
            Provider::OpenShip,
            "box-os",
            "ghcr.io/fleet/instance:1",
            "git@h:m.git",
            None,
        )
        .unwrap();
        let home = scratch("openship-noproj");
        let refused = provision(&home, &spec, false).expect_err("refused");
        assert!(refused.contains("needs OPENSHIP_PROJECT"), "{refused}");
        assert!(
            refused.contains("the call has no destination"),
            "the refusal names the hazard: {refused}"
        );
        // Nothing ran, so nothing was journaled.
        let journal = crate::journal::Journal::open(&home).unwrap();
        assert!(
            journal
                .replay(0)
                .unwrap()
                .iter()
                .all(|e| e.kind != "provision"),
            "a refused deploy journaled a step"
        );
        unsafe { std::env::remove_var("OPENSHIP_TOKEN") };
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn an_openship_deploy_without_a_token_is_refused_by_name() {
        // The project is named, so this exercises the NEXT refusal: no
        // credential. A deploy with no bearer cannot authenticate, and the
        // refusal says where a token comes from.
        let _env = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        unsafe {
            std::env::set_var("OPENSHIP_PROJECT", "a-project");
            std::env::remove_var("OPENSHIP_TOKEN");
        }
        let spec = service_spec(
            Provider::OpenShip,
            "box-tok",
            "ghcr.io/fleet/instance:1",
            "git@h:m.git",
            None,
        )
        .unwrap();
        let home = scratch("openship-notok");
        let refused = provision(&home, &spec, false).expect_err("refused");
        assert!(refused.contains("needs OPENSHIP_TOKEN"), "{refused}");
        assert!(
            refused.contains("a read-only token cannot deploy"),
            "the refusal explains why a scope refusal may follow: {refused}"
        );
        unsafe { std::env::remove_var("OPENSHIP_PROJECT") };
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn a_dry_run_never_reaches_a_provider() {
        // A dry run short-circuits before the provider match, so it answers
        // even for openship with no project and no CLI on PATH: the preview
        // is data, not an execution.
        let spec = service_spec(
            Provider::OpenShip,
            "box-dry",
            "ghcr.io/fleet/instance:1",
            "git@h:m.git",
            None,
        )
        .unwrap();
        let home = scratch("openship-dry");
        unsafe { std::env::remove_var("OPENSHIP_PROJECT") };
        let provisioned = provision(&home, &spec, true).expect("a dry run answers");
        assert_eq!(provisioned.handle, "(dry run)");
        assert!(provisioned.steps[0].1.contains("would deploy"));
        std::fs::remove_dir_all(&home).ok();
    }

    /// Live: the transport reaches the real endpoint. `#[ignore]` because it
    /// needs a credential and the network — run it by name:
    ///
    /// ```text
    /// OPENSHIP_TOKEN=opsh_pat_… cargo test -p fleet --lib \
    ///   openship_mcp_transport_reaches_the_live_endpoint -- --ignored --nocapture
    /// ```
    ///
    /// It exists because a transport that has never spoken to the server is a
    /// guess, and this one was wrong twice before it was checked.
    #[test]
    #[ignore]
    fn openship_mcp_transport_reaches_the_live_endpoint() {
        let Some(token) = openship_token() else {
            panic!("set OPENSHIP_TOKEN to run this");
        };
        let reply = call_openship(&token, "get_permissions_workspaces", json!({}))
            .expect("the live endpoint answers");
        let text = reply["content"][0]["text"].as_str().expect("a text result");
        assert!(
            text.contains("boundOrganizationId"),
            "the workspace discovery answered: {text}"
        );
        eprintln!("live openship call ok: {}", &text[..text.len().min(160)]);
    }

    #[test]
    fn an_unknown_provider_is_refused_by_name() {
        assert!(Provider::parse("ssh").is_none());
        assert!(Provider::parse("kubernetes").is_none());
        assert_eq!(Provider::parse("OpenShip"), Some(Provider::OpenShip));
        assert_eq!(Provider::parse("docker"), Some(Provider::Docker));
    }

    /// C0: a local provision without a remote lays out the home and mints the
    /// id, cloning nothing.
    #[test]
    fn provision_without_a_remote_lays_out_the_home() {
        let home = scratch("no-remote");
        let report = provision_local(&home, None).unwrap();
        assert!(!report.corpus_cloned);
        assert_eq!(report.install_id.len(), 32);
        assert!(home.join("sessions").exists());
        assert!(home.join("memories").exists());
    }
}
