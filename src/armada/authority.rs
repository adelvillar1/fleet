//! The authority transport (armada L8.2, D45): every control frame carries
//! the principal (L1) and, for `destructive`/`governance` verbs, a fresh
//! assertion (L3) signed by the flagship's Ed25519 key. The instance verifies
//! the signature BEFORE it obeys, then re-checks the principal's grants at its
//! own guard. The relay cannot fabricate (it holds no key), an instance cannot
//! escalate (it verifies against the flagship's public key), and an agent
//! cannot borrow (the private key lives only at the flagship, D47).
//!
//! The signing key is L8's to mint and keep (L3 left the custody gap): it is
//! generated once at the flagship and persisted 0600; an instance holds only
//! the public key, distributed out-of-band.

use std::path::{Path, PathBuf};

use ring::signature::{Ed25519KeyPair, KeyPair as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::security::{ActionClass, Assertion};

/// A control frame as it rides the relay (L6 routes it dumbly, D22). The
/// `kind` is always `"control"`; the vocabulary is L8's, the broker parses
/// none of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFrame {
    pub kind: String,
    pub verb: String,
    pub install: String,
    pub principal: String,
    pub payload: Value,
    pub nonce: String,
    pub issued_at: u64,
    /// The flagship's fresh assertion — present for privileged verbs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assertion: Option<Assertion>,
}

/// The flagship's private signing key (PKCS#8), 0600, at the flagship only.
pub fn private_key_path(home: &Path) -> PathBuf {
    crate::armada::armada_home(home).join("flagship-key.pkcs8")
}

/// The flagship's public key (raw 32 bytes) — what an instance holds to
/// verify. Distributed out-of-band, never the private key.
pub fn public_key_path(home: &Path) -> PathBuf {
    crate::armada::armada_home(home).join("flagship-key.pub")
}

/// Mint the flagship signing key once: generate an Ed25519 keypair, persist
/// the PKCS#8 0600 and the raw public key. Idempotent — an existing key is
/// loaded, never regenerated (rotation is an open fork, §13).
pub fn mint_flagship_key(home: &Path) -> Result<Ed25519KeyPair, String> {
    if let Some(existing) = load_flagship_key(home) {
        return Ok(existing);
    }
    let dir = crate::armada::armada_home(home);
    std::fs::create_dir_all(&dir).map_err(|e| format!("the flagship home: {e}"))?;
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| "the flagship key could not be generated".to_string())?;
    let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| "the flagship key could not be read".to_string())?;
    let private = private_key_path(home);
    std::fs::write(&private, pkcs8.as_ref()).map_err(|e| format!("the flagship key write: {e}"))?;
    set_private_permissions(&private);
    std::fs::write(public_key_path(home), key.public_key().as_ref())
        .map_err(|e| format!("the flagship public key write: {e}"))?;
    Ok(key)
}

/// Load the flagship's private key, if this install holds it (the flagship).
/// An instance returns `None` — it has no private key to sign with (D47).
pub fn load_flagship_key(home: &Path) -> Option<Ed25519KeyPair> {
    let bytes = std::fs::read(private_key_path(home)).ok()?;
    Ed25519KeyPair::from_pkcs8(&bytes).ok()
}

/// Load the flagship's public key (what an instance verifies against).
pub fn load_public_key(home: &Path) -> Option<Vec<u8>> {
    std::fs::read(public_key_path(home)).ok()
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) {}

/// Build a control frame, signing a fresh assertion for a privileged verb
/// (D18). The flagship calls this; the assertion binds the principal, class,
/// scope, and the per-action target, so a signature cannot be replayed onto a
/// different action.
#[allow(clippy::too_many_arguments)]
pub fn sign_frame(
    key_pair: &Ed25519KeyPair,
    verb: &str,
    install: &str,
    principal: &str,
    payload: Value,
    nonce: &str,
    issued_at: u64,
    class: ActionClass,
    scope: &str,
    target: &str,
) -> ControlFrame {
    let assertion = if class.needs_step_up() {
        Some(Assertion::sign(
            key_pair, principal, class, scope, target, issued_at, nonce,
        ))
    } else {
        None
    };
    ControlFrame {
        kind: "control".to_string(),
        verb: verb.to_string(),
        install: install.to_string(),
        principal: principal.to_string(),
        payload,
        nonce: nonce.to_string(),
        issued_at,
        assertion,
    }
}

/// Verify a frame's authority BEFORE obeying (D45). A privileged verb
/// (destructive/governance) must carry a fresh assertion signed by the
/// flagship whose principal matches the frame's; a forged, unsigned, stale,
/// or mismatched assertion is refused by name. A non-privileged verb carries
/// the principal alone — the guard checks its grants next (control.rs).
pub fn verify_frame(
    frame: &ControlFrame,
    class: ActionClass,
    public_key: &[u8],
    now_secs: u64,
) -> Result<(), String> {
    if !class.needs_step_up() {
        return Ok(());
    }
    let assertion = frame.assertion.as_ref().ok_or_else(|| {
        format!(
            "the `{}` verb is privileged and carries no flagship assertion — step-up required",
            frame.verb
        )
    })?;
    if assertion.principal != frame.principal {
        return Err(format!(
            "the assertion's principal `{}` does not match the frame's `{}` — refused",
            assertion.principal, frame.principal
        ));
    }
    if !assertion.verify(public_key, now_secs) {
        return Err(format!(
            "the `{}` verb's flagship assertion is forged or stale — refused before obeying",
            frame.verb
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::ActionClass;

    fn scratch(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-authority-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// C1: a flagship-signed privileged frame verifies; a forged signature
    /// and an unsigned privileged frame are refused by name.
    #[test]
    fn verify_before_obey_refuses_forged_and_unsigned() {
        let home = scratch("verify");
        let key = mint_flagship_key(&home).unwrap();
        let public = load_public_key(&home).unwrap();
        let frame = sign_frame(
            &key,
            "retire",
            "install-a",
            "owner",
            serde_json::json!({}),
            "nonce-1",
            1000,
            ActionClass::Destructive,
            "instance",
            "install-a",
        );
        // The genuine frame verifies within the window.
        assert!(verify_frame(&frame, ActionClass::Destructive, &public, 1000).is_ok());

        // A frame signed by a DIFFERENT key (a forgery) is refused.
        let other_home = scratch("forgery");
        let other_key = mint_flagship_key(&other_home).unwrap();
        let forged = sign_frame(
            &other_key,
            "retire",
            "install-a",
            "owner",
            serde_json::json!({}),
            "nonce-2",
            1000,
            ActionClass::Destructive,
            "instance",
            "install-a",
        );
        let err = verify_frame(&forged, ActionClass::Destructive, &public, 1000).unwrap_err();
        assert!(err.contains("forged or stale"), "{err}");

        // An unsigned privileged frame is refused (step-up required).
        let mut unsigned = frame.clone();
        unsigned.assertion = None;
        let err = verify_frame(&unsigned, ActionClass::Destructive, &public, 1000).unwrap_err();
        assert!(err.contains("step-up required"), "{err}");

        // A stale assertion (past the freshness window) is refused.
        let err = verify_frame(
            &frame,
            ActionClass::Destructive,
            &public,
            1000 + crate::security::ASSERTION_WINDOW_SECS + 1,
        )
        .unwrap_err();
        assert!(err.contains("forged or stale"), "{err}");
    }

    /// C1: a non-privileged verb carries the principal alone — no assertion
    /// needed (the guard checks grants next).
    #[test]
    fn a_read_verb_needs_no_assertion() {
        let home = scratch("read");
        let key = mint_flagship_key(&home).unwrap();
        let public = load_public_key(&home).unwrap();
        let frame = sign_frame(
            &key,
            "health",
            "install-a",
            "owner",
            serde_json::json!({}),
            "nonce",
            1000,
            ActionClass::Read,
            "instance",
            "install-a",
        );
        assert!(frame.assertion.is_none());
        assert!(verify_frame(&frame, ActionClass::Read, &public, 1000).is_ok());
    }

    /// C4: the private key lives only at the flagship; an instance holds the
    /// public key alone and cannot sign (the agent cannot borrow authority).
    #[test]
    fn the_signing_key_is_custodied_at_the_flagship() {
        let flagship = scratch("flagship");
        mint_flagship_key(&flagship).unwrap();
        assert!(
            load_flagship_key(&flagship).is_some(),
            "the flagship holds the key"
        );
        assert!(private_key_path(&flagship).exists());

        // An instance: only the public key is distributed, never the private.
        let instance = scratch("instance");
        std::fs::create_dir_all(crate::armada::armada_home(&instance)).unwrap();
        std::fs::copy(public_key_path(&flagship), public_key_path(&instance)).unwrap();
        assert!(
            load_flagship_key(&instance).is_none(),
            "an instance has no private key"
        );
        assert!(load_public_key(&instance).is_some(), "it can still verify");

        // The private key is 0600 (the agent's tools cannot read it loosely).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(private_key_path(&flagship))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "the flagship key is 0600");
        }
    }

    /// C5: a principal-mismatched assertion is refused — a signature for one
    /// principal cannot ride a frame for another.
    #[test]
    fn an_assertion_cannot_ride_a_different_principal() {
        let home = scratch("mismatch");
        let key = mint_flagship_key(&home).unwrap();
        let public = load_public_key(&home).unwrap();
        let mut frame = sign_frame(
            &key,
            "retire",
            "install-a",
            "owner",
            serde_json::json!({}),
            "nonce",
            1000,
            ActionClass::Destructive,
            "instance",
            "install-a",
        );
        frame.principal = "stranger".to_string();
        let err = verify_frame(&frame, ActionClass::Destructive, &public, 1000).unwrap_err();
        assert!(err.contains("does not match"), "{err}");
    }
}
