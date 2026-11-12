//! The control plane (armada L8): the hand. A flagship — a fleet install in
//! a flagship role, one binary — commands instances over the relay. Authority
//! rides *in the frame*, signed by the flagship's Ed25519 key and verified at
//! the instance's guard before it obeys; the relay stays a dumb broker (L6).
//! An instance earns dispatch autonomy by measured reliability, never by
//! registration (D49).
//!
//! This module composes every lower contract and re-implements none: identity
//! (L1) for the install-id and principal, the guard + assertion (L3) for
//! admission and the second factor, corpus sync (L4) for the `corpus-sync`
//! verb, gated dispatch (L5) and the sealed push transport (L6).

pub mod authority;
pub mod control;
pub mod deploy;
pub mod roster;

use std::path::{Path, PathBuf};

/// The flagship role's home: `<home>/armada`. The signing key lives here, at
/// the flagship, never on an instance the agent can reach (D47).
pub fn armada_home(home: &Path) -> PathBuf {
    home.join("armada")
}

/// Whether this install is configured as a flagship: it holds a roster. An
/// install with no roster is an ordinary single-instance fleet, unchanged
/// (C8) — the flagship role is additive, role + config, never a forked binary.
pub fn is_flagship(home: &Path) -> bool {
    roster::path(home).exists()
}
