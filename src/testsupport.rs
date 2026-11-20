//! Test support: the primitives that make a test hermetic.
//!
//! The plan this answers (`2026-11-20-test-env-isolation.md`) measured **85**
//! call sites of `set_var`/`remove_var` across ~20 modules, each of them
//! mutating the whole process from a threaded test runner. Two classes of
//! flake came out of that, both closed the same day as individual fixes:
//! KI-578 (a test inheriting the machine's git default branch) and KI-579
//! (`route`'s test reading a provider key out of the process environment).
//!
//! One lock and one guard, so the next test that needs an environment takes
//! the shared one rather than inventing a third.

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

/// The process-wide environment lock.
///
/// `cargo test` runs a binary's tests on threads. A `set_var` in one test is
/// visible to every other test in the process, mid-flight, so the only way a
/// test can reason about the environment is to hold this lock across the
/// whole read-or-write window — and for every other test that touches the
/// environment to hold the same one.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Take the shared environment lock.
///
/// A poisoned lock is recovered rather than propagated: a test that panicked
/// while holding it has already failed, and every later test would fail on
/// the poison instead of its own assertion — which hides the real fault.
pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Set one variable for the guard's lifetime, restoring whatever was there
/// before — including its absence — on drop.
///
/// The caller holds [`env_lock`] for the guard's whole lifetime; that is the
/// safety condition the `unsafe` blocks below rest on.
pub(crate) struct EnvVar {
    name: String,
    previous: Option<OsString>,
}

impl EnvVar {
    /// Set `name` to `value` until the guard drops.
    pub(crate) fn set(name: &str, value: &str) -> EnvVar {
        let previous = std::env::var_os(name);
        // SAFETY: the caller holds `env_lock()` for this guard's lifetime, so
        // no other test in the binary reads or writes the environment
        // concurrently.
        unsafe { std::env::set_var(name, value) };
        EnvVar {
            name: name.to_string(),
            previous,
        }
    }

    /// Remove `name` until the guard drops, restoring it afterwards.
    pub(crate) fn unset(name: &str) -> EnvVar {
        let previous = std::env::var_os(name);
        // SAFETY: as above.
        unsafe { std::env::remove_var(name) };
        EnvVar {
            name: name.to_string(),
            previous,
        }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        // SAFETY: the guard was constructed under `env_lock()` and the caller
        // keeps holding it until after this drop.
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(&self.name, value),
                None => std::env::remove_var(&self.name),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guard restores what it took, and nested guards unwind to the state
    /// before the first — including restoring an absence.
    #[test]
    fn a_guard_restores_the_variable_it_took() {
        let _lock = env_lock();
        assert!(
            std::env::var_os("FLEET_TESTSUPPORT_ABSENT").is_none(),
            "the fixture name is fresh"
        );
        {
            let _held = EnvVar::set("FLEET_TESTSUPPORT_ABSENT", "while-held");
            assert_eq!(
                std::env::var("FLEET_TESTSUPPORT_ABSENT").as_deref(),
                Ok("while-held")
            );
        }
        assert!(
            std::env::var_os("FLEET_TESTSUPPORT_ABSENT").is_none(),
            "the absence is restored, not an empty string"
        );

        let _seed = EnvVar::set("FLEET_TESTSUPPORT_NESTED", "seeded");
        {
            let _a = EnvVar::set("FLEET_TESTSUPPORT_NESTED", "first");
            let _b = EnvVar::set("FLEET_TESTSUPPORT_NESTED", "second");
            assert_eq!(
                std::env::var("FLEET_TESTSUPPORT_NESTED").as_deref(),
                Ok("second")
            );
        }
        assert_eq!(
            std::env::var("FLEET_TESTSUPPORT_NESTED").as_deref(),
            Ok("seeded"),
            "the nested guards unwind to the first guard's value"
        );
    }

    /// An `unset` guard removes a value and puts it back.
    #[test]
    fn an_unset_guard_restores_the_value_it_removed() {
        let _lock = env_lock();
        let _seed = EnvVar::set("FLEET_TESTSUPPORT_UNSET", "seeded");
        {
            let _gone = EnvVar::unset("FLEET_TESTSUPPORT_UNSET");
            assert!(std::env::var_os("FLEET_TESTSUPPORT_UNSET").is_none());
        }
        assert_eq!(
            std::env::var("FLEET_TESTSUPPORT_UNSET").as_deref(),
            Ok("seeded")
        );
    }
}
