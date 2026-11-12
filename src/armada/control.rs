//! The control vocabulary (armada L8.3, D46): each verb reuses a bridge verb
//! and runs through the instance's own guard (D15) and admission. Authority
//! is verified before obeying (authority.rs); the guard then checks the
//! principal's grants; a privileged verb also clears step-up (D18). The
//! flagship cannot overwhelm a small VPS — the instance admits for itself —
//! and cannot trust a fresh install until it settles (D49).

use std::path::Path;

use serde_json::{json, Value};

use crate::armada::authority::{self, ControlFrame};
use crate::security::guard;
use crate::security::{ActionClass, Decision, Dimension, GrantStore};

/// The control verbs (D46). Each maps to the bridge verb it reuses and the
/// action class the guard meters it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// dispatch → bridge.run (gated dispatch, L5).
    Dispatch,
    /// state → bridge.state (read).
    State,
    /// corpus-sync → L4 sync (write).
    CorpusSync,
    /// version-roll → roll the install's version (destructive).
    VersionRoll,
    /// retire → retire an instance (destructive).
    Retire,
    /// health → the health probe (read).
    Health,
    /// register → the roster join (governance).
    Register,
    /// heartbeat → roster liveness (governance).
    Heartbeat,
}

impl Verb {
    pub fn parse(name: &str) -> Option<Verb> {
        match name.trim().to_lowercase().as_str() {
            "dispatch" => Some(Verb::Dispatch),
            "state" => Some(Verb::State),
            "corpus-sync" => Some(Verb::CorpusSync),
            "version-roll" => Some(Verb::VersionRoll),
            "retire" => Some(Verb::Retire),
            "health" => Some(Verb::Health),
            "register" => Some(Verb::Register),
            "heartbeat" => Some(Verb::Heartbeat),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Verb::Dispatch => "dispatch",
            Verb::State => "state",
            Verb::CorpusSync => "corpus-sync",
            Verb::VersionRoll => "version-roll",
            Verb::Retire => "retire",
            Verb::Health => "health",
            Verb::Register => "register",
            Verb::Heartbeat => "heartbeat",
        }
    }

    /// The action class the guard meters this verb by (C3).
    pub fn class(self) -> ActionClass {
        match self {
            Verb::Dispatch => ActionClass::Dispatch,
            Verb::State | Verb::Health => ActionClass::Read,
            Verb::CorpusSync => ActionClass::Write,
            Verb::VersionRoll | Verb::Retire => ActionClass::Destructive,
            Verb::Register | Verb::Heartbeat => ActionClass::Governance,
        }
    }

    /// The grant dimension this verb falls under.
    pub fn dimension(self) -> Dimension {
        match self {
            Verb::Dispatch => Dimension::Workflow,
            Verb::CorpusSync => Dimension::Corpus,
            Verb::VersionRoll | Verb::Retire => Dimension::Resource,
            Verb::Register | Verb::Heartbeat => Dimension::Governance,
            Verb::State | Verb::Health => Dimension::Resource,
        }
    }
}

/// A frame that has cleared authority + the guard + step-up: the instance
/// has admitted it and may obey.
#[derive(Debug, Clone)]
pub struct Admitted {
    pub verb: Verb,
    pub principal: String,
    pub frame: ControlFrame,
}

/// Admit a control frame (C2): verify the flagship's authority before obeying
/// (D45), then the instance's own guard (D15), then step-up for a privileged
/// verb (D18). An ungranted verb is refused by name and settles a `guard`
/// journal event; the refusal never reaches execution.
pub fn admit(
    home: &Path,
    frame: &ControlFrame,
    public_key: &[u8],
    now_secs: u64,
) -> Result<Admitted, String> {
    let verb =
        Verb::parse(&frame.verb).ok_or_else(|| format!("unknown control verb `{}`", frame.verb))?;
    // 1. Authority before obeying — the relay cannot fabricate (D45).
    authority::verify_frame(frame, verb.class(), public_key, now_secs)?;
    // 2. The instance's own guard — an instance cannot escalate (D15).
    let store = GrantStore::load(home).map_err(|e| format!("the grant store: {e}"))?;
    let scope = format!("instance:{}", frame.install);
    let decision = guard::check_grant(
        &frame.principal,
        verb.dimension(),
        &scope,
        verb.class(),
        &store,
    );
    match decision {
        Decision::Granted => {}
        Decision::Denied { name } => {
            journal_guard(home, frame, "denied", &name);
            return Err(format!(
                "the principal `{}` is not granted `{}` on this instance: {name}",
                frame.principal, frame.verb
            ));
        }
        Decision::StepUpRequired => {
            journal_guard(home, frame, "step_up", "step-up required");
            return Err(format!(
                "the `{}` verb requires step-up approval",
                frame.verb
            ));
        }
    }
    // 3. Step-up for a privileged verb — the assertion was verified above;
    //    the guard's step-up companion confirms it clears (D18).
    if verb.class().needs_step_up() {
        let step = guard::check_step_up(
            verb.class(),
            false,
            frame.assertion.as_ref(),
            Some(public_key),
            now_secs,
        );
        if !matches!(step, Decision::Granted) {
            return Err(format!(
                "the `{}` verb requires step-up approval",
                frame.verb
            ));
        }
    }
    Ok(Admitted {
        verb,
        principal: frame.principal.clone(),
        frame: frame.clone(),
    })
}

/// What obeying an admitted frame produces. The read/governance/sync verbs
/// perform here; `dispatch` and the destructive verbs return the action the
/// caller (the bridge / the deployment layer) executes — dispatch reuses
/// `bridge.run`, the destructive acts are L9's, both already admitted.
#[derive(Debug, Clone)]
pub enum Outcome {
    Done(Value),
    Dispatch { task: String, principal: String },
    Destructive { verb: Verb, target: String },
}

/// Obey an admitted frame (C3). `corpus-sync` triggers the L4 sync; `health`
/// and `state` read; `register`/`heartbeat` ride the roster; `dispatch` and
/// the destructive verbs return the action the caller executes.
pub fn execute(home: &Path, admitted: &Admitted) -> Result<Outcome, String> {
    let frame = &admitted.frame;
    match admitted.verb {
        Verb::CorpusSync => {
            let report = crate::corpus::sync(home, None)?;
            Ok(Outcome::Done(json!({
                "type": "corpus-sync",
                "merged": report.merged,
                "summed": report.summed,
                "resolved": report.resolved,
                "refused": report.refused,
            })))
        }
        Verb::Health => {
            let report = crate::health::probe(home, true);
            Ok(Outcome::Done(json!({
                "type": "health",
                "services": report.services.iter().map(|s| json!({
                    "name": s.name, "up": s.up,
                })).collect::<Vec<_>>(),
            })))
        }
        Verb::State => Ok(Outcome::Done(
            json!({"type": "state", "install": frame.install}),
        )),
        Verb::Register => {
            let install = frame.payload["install_id"]
                .as_str()
                .unwrap_or(&frame.install);
            let version = frame.payload["version"].as_str().unwrap_or("unknown");
            let capacity = frame.payload["capacity_kib"].as_u64().unwrap_or(0);
            let instance = crate::armada::roster::register(
                home,
                install,
                &frame.install,
                version,
                capacity,
                frame.issued_at,
            )?;
            Ok(Outcome::Done(
                json!({"type": "register", "sid": instance.sid}),
            ))
        }
        Verb::Heartbeat => {
            crate::armada::roster::heartbeat(home, &frame.install, frame.issued_at)?;
            Ok(Outcome::Done(
                json!({"type": "heartbeat", "sid": frame.install}),
            ))
        }
        Verb::Dispatch => {
            let task = frame.payload["task"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            Ok(Outcome::Dispatch {
                task,
                principal: admitted.principal.clone(),
            })
        }
        Verb::VersionRoll | Verb::Retire => Ok(Outcome::Destructive {
            verb: admitted.verb,
            target: frame.install.clone(),
        }),
    }
}

fn journal_guard(home: &Path, frame: &ControlFrame, decision: &str, name: &str) {
    if let Ok(mut journal) = crate::journal::Journal::open(home) {
        let _ = journal.append(
            "guard",
            Some(&frame.principal),
            Some(&frame.install),
            json!({"action": frame.verb, "decision": decision, "name": name}),
        );
    }
}

/// An instance's measured reliability (D49): the dispatch outcomes it has
/// settled over the journal. A fresh instance is UN-CALIBRATED until it
/// settles — autonomy is earned by measurement, never by registration. The
/// grade is a beat over the journal, never a synchronous sys1 ask.
#[derive(Debug, Clone, PartialEq)]
pub struct Reliability {
    pub settled: usize,
    pub succeeded: usize,
    pub calibrated: bool,
    pub reliability: f64,
}

/// Grade an instance's dispatch outcomes from the journal (C6). Only settled
/// dispatches count; a fresh instance with none is un-calibrated.
pub fn grade(home: &Path, sid: &str) -> Reliability {
    let mut settled = 0usize;
    let mut succeeded = 0usize;
    if let Ok(journal) = crate::journal::Journal::open(home) {
        if let Ok(events) = journal.replay(0) {
            for event in events {
                if event.kind != "dispatch" {
                    continue;
                }
                if event.payload["sid"].as_str() != Some(sid) {
                    continue;
                }
                let Some(outcome) = event.payload["outcome"].as_str() else {
                    continue;
                };
                settled += 1;
                if matches!(outcome, "finished" | "delivered" | "ok") {
                    succeeded += 1;
                }
            }
        }
    }
    Reliability {
        settled,
        succeeded,
        calibrated: settled > 0,
        reliability: if settled > 0 {
            succeeded as f64 / settled as f64
        } else {
            0.0
        },
    }
}

/// The relay-down degradation, named (D50): instances run autonomously, the
/// corpus still syncs over git (L4 is independent of the relay), and live
/// control resumes when the relay returns. Never a silent stop.
pub fn relay_down_note() -> &'static str {
    "the relay is down: instances run autonomously and the corpus still syncs over \
     git; live control resumes when the relay returns"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armada::authority::{mint_flagship_key, sign_frame};
    use crate::identity::PrincipalId;
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-control-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn frame(home: &Path, verb: &str, principal: &str, class: ActionClass) -> ControlFrame {
        let key = mint_flagship_key(home).unwrap();
        sign_frame(
            &key,
            verb,
            "install-a",
            principal,
            json!({"task": "do the work"}),
            "nonce",
            1000,
            class,
            "instance:install-a",
            "install-a",
        )
    }

    /// C3: each verb's action class is correct.
    #[test]
    fn verb_classes_are_correct() {
        assert_eq!(Verb::Dispatch.class(), ActionClass::Dispatch);
        assert_eq!(Verb::CorpusSync.class(), ActionClass::Write);
        assert_eq!(Verb::VersionRoll.class(), ActionClass::Destructive);
        assert_eq!(Verb::Retire.class(), ActionClass::Destructive);
        assert_eq!(Verb::Health.class(), ActionClass::Read);
        assert_eq!(Verb::State.class(), ActionClass::Read);
        assert_eq!(Verb::Register.class(), ActionClass::Governance);
        assert_eq!(Verb::Heartbeat.class(), ActionClass::Governance);
        assert_eq!(Verb::parse("bogus"), None);
    }

    /// C2: the owner wildcard admits; an ungranted principal is refused by
    /// name and the refusal settles a guard journal event.
    #[test]
    fn admission_gates_the_principal() {
        let home = scratch("admit");
        let public = crate::armada::authority::load_public_key(&home).unwrap_or_else(|| {
            mint_flagship_key(&home).unwrap();
            crate::armada::authority::load_public_key(&home).unwrap()
        });
        // The owner is admitted (lane 1 wildcard).
        let owner_frame = frame(&home, "dispatch", "owner", ActionClass::Dispatch);
        let admitted = admit(&home, &owner_frame, &public, 1000).unwrap();
        assert_eq!(admitted.verb, Verb::Dispatch);

        // A stranger with no grant is refused.
        let stranger_frame = frame(&home, "dispatch", "stranger", ActionClass::Dispatch);
        let err = admit(&home, &stranger_frame, &public, 1000).unwrap_err();
        assert!(err.contains("not granted"), "{err}");
        let journal = crate::journal::Journal::open(&home).unwrap();
        let events = journal.replay(0).unwrap();
        assert!(events
            .iter()
            .any(|e| e.kind == "guard" && e.payload["decision"] == "denied"));
    }

    /// C3/C5: a destructive verb without a valid assertion is refused
    /// step-up; with the flagship's fresh assertion it admits.
    #[test]
    fn a_destructive_verb_requires_the_second_factor() {
        let home = scratch("stepup");
        let key = mint_flagship_key(&home).unwrap();
        let public = crate::armada::authority::load_public_key(&home).unwrap();
        // Unsigned destructive frame: refused before obeying.
        let mut unsigned = sign_frame(
            &key,
            "retire",
            "install-a",
            "owner",
            json!({}),
            "nonce",
            1000,
            ActionClass::Destructive,
            "instance:install-a",
            "install-a",
        );
        unsigned.assertion = None;
        let err = admit(&home, &unsigned, &public, 1000).unwrap_err();
        assert!(err.contains("step-up"), "{err}");
        // The signed frame admits and obeys to a Destructive action.
        let signed = frame(&home, "retire", "owner", ActionClass::Destructive);
        let admitted = admit(&home, &signed, &public, 1000).unwrap();
        match execute(&home, &admitted).unwrap() {
            Outcome::Destructive { verb, target } => {
                assert_eq!(verb, Verb::Retire);
                assert_eq!(target, "install-a");
            }
            other => panic!("expected a destructive action: {other:?}"),
        }
    }

    /// C3: corpus-sync triggers the L4 sync (which names its refusal without
    /// a remote — proving the wiring reaches L4).
    #[test]
    fn corpus_sync_triggers_the_l4_sync() {
        let home = scratch("corpus");
        let public = {
            mint_flagship_key(&home).unwrap();
            crate::armada::authority::load_public_key(&home).unwrap()
        };
        let f = frame(&home, "corpus-sync", "owner", ActionClass::Write);
        let admitted = admit(&home, &f, &public, 1000).unwrap();
        // No remote configured: L4's named refusal proves the sync was reached.
        let err = execute(&home, &admitted).unwrap_err();
        assert!(err.contains("remote") || err.contains("corpus"), "{err}");
    }

    /// C2: dispatch obeys to a Dispatch action carrying the principal (the
    /// caller reuses bridge.run); the instance admits for itself.
    #[test]
    fn dispatch_reuses_the_run_seam_under_the_principal() {
        let home = scratch("dispatch");
        let public = {
            mint_flagship_key(&home).unwrap();
            crate::armada::authority::load_public_key(&home).unwrap()
        };
        let f = frame(&home, "dispatch", "owner", ActionClass::Dispatch);
        let admitted = admit(&home, &f, &public, 1000).unwrap();
        match execute(&home, &admitted).unwrap() {
            Outcome::Dispatch { task, principal } => {
                assert_eq!(task, "do the work");
                assert_eq!(principal, "owner");
            }
            other => panic!("expected a dispatch action: {other:?}"),
        }
    }

    /// C6: a fresh instance is un-calibrated until it settles; the grade
    /// counts measured dispatch outcomes over the journal.
    #[test]
    fn reliability_is_earned_by_settles() {
        let home = scratch("reliability");
        // Fresh: no settles, un-calibrated.
        let fresh = grade(&home, "install-a");
        assert!(!fresh.calibrated);
        assert_eq!(fresh.settled, 0);

        // Settle three dispatches: two finished, one failed.
        let mut journal = crate::journal::Journal::open(&home).unwrap();
        for outcome in ["finished", "finished", "failed"] {
            journal
                .append(
                    "dispatch",
                    Some("owner"),
                    Some("install-a"),
                    json!({"sid": "install-a", "outcome": outcome}),
                )
                .unwrap();
        }
        let graded = grade(&home, "install-a");
        assert!(graded.calibrated);
        assert_eq!(graded.settled, 3);
        assert_eq!(graded.succeeded, 2);
        assert!((graded.reliability - 2.0 / 3.0).abs() < 1e-9);
        // Another instance's settles do not count toward this one.
        assert!(!grade(&home, "install-b").calibrated);
    }

    /// C7: relay-down is a named degradation, not a silent stop.
    #[test]
    fn relay_down_is_named() {
        let note = relay_down_note();
        assert!(note.contains("autonomously"));
        assert!(note.contains("git"));
        assert!(note.contains("resumes"));
    }

    /// C8: an install with no roster behaves as today (the flagship role is
    /// additive). The owner principal still resolves.
    #[test]
    fn a_no_roster_install_is_unchanged() {
        let home = scratch("no-roster");
        assert!(!crate::armada::is_flagship(&home));
        assert_eq!(crate::identity::principal_for(&home), PrincipalId::owner());
    }
}
