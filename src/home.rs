//! Fleet's on-disk home: session logs and the optional `.env` credential
//! fallback live under `~/.fleet`. Nothing else is persisted.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Fleet's home directory: `$FLEET_HOME` when set and non-empty, else
/// `<user home>/.fleet`. `user_home` is the OS home (`HOME`/`USERPROFILE`),
/// passed in so callers decide their own environment precedence.
/// The absolute home path. An error when neither `$FLEET_HOME` nor a
/// user home is known.
pub fn fleet_home(
    fleet_home_env: Option<&str>,
    user_home: Option<&str>,
) -> Result<PathBuf, String> {
    match fleet_home_env
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => Ok(PathBuf::from(value)),
        None => user_home
            .map(|home| PathBuf::from(home).join(".fleet"))
            .ok_or_else(|| "no home directory known; set FLEET_HOME or HOME".to_string()),
    }
}

/// Directory holding session JSONL logs, inside `home`.
pub fn sessions_dir(home: &Path) -> PathBuf {
    home.join("sessions")
}

/// The identity slot's bound, in characters. A persona file rides every
/// request; past this bound it is cut with a marked truncation rather
/// than silently pasted whole.
pub const MAX_SOUL_CHARS: usize = 8192;

/// Read `home/SOUL.md` — the run's identity override. A missing or
/// empty file is `None` (the compiled-in constant serves); a present
/// one is sanitized the way the memory writer sanitizes: a leading BOM
/// stripped, control characters removed (not replaced, so a stripped
/// escape leaves its payload visible and inert), trimmed, then bounded
/// at [`MAX_SOUL_CHARS`] with a marked cut — the prompt never carries
/// the file raw.
pub fn read_soul(home: &Path) -> Option<String> {
    let text = fs::read_to_string(home.join("SOUL.md")).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut cleaned: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.chars().count() > MAX_SOUL_CHARS {
        let cut: String = cleaned.chars().take(MAX_SOUL_CHARS).collect();
        cleaned = format!("{cut}\n[truncated at {MAX_SOUL_CHARS} characters]");
    }
    Some(cleaned)
}

/// Write the SOUL.md slot: the owner's own identity text, written
/// through the one home the slot lives in. The read sanitizes and
/// bounds whatever lands here, so the write stores what the owner
/// typed and the read is the only filter — a write that pre-judged the
/// text would be a second sanitizer to disagree with the first.
pub fn write_soul(home: &Path, text: &str) -> std::io::Result<()> {
    fs::write(home.join("SOUL.md"), text)
}

/// The dotenv map with the vault's entries laid over it: a name sealed
/// in the vault answers with its sealed value even while the plaintext
/// line is still in the file, which is what makes `fleet vault init`'s
/// leave-the-lines migration honest — the sealed copy is the one that
/// answers, and deleting the plaintext line is housekeeping, not the
/// step that switches anything on. The passphrase comes in rather than
/// being read here, so the tests never touch the process environment.
/// Without a passphrase, or with none that opens the vault, the map is
/// the plain dotenv and resolution answers as it always has.
pub fn read_dotenv_with_vault(home: &Path, passphrase: Option<&str>) -> HashMap<String, String> {
    let mut map = read_dotenv(home);
    let passphrase = match passphrase.filter(|passphrase| !passphrase.is_empty()) {
        Some(passphrase) => passphrase,
        None => return map,
    };
    let Ok(text) = std::fs::read_to_string(home.join(crate::vault::VAULT_FILE)) else {
        return map;
    };
    let Ok(vault) = crate::vault::Vault::from_json(&text) else {
        return map;
    };
    for (name, value) in vault.open_all(passphrase) {
        if !value.is_empty() {
            map.insert(name, value);
        }
    }
    map
}

/// The process's vault unlock. The environment variable is the
/// launch-time source; the GUI's unlock (or lock) overrides it for this
/// process only. The passphrase is held in memory and never written.
#[derive(Debug)]
enum VaultUnlock {
    Unset,
    Locked,
    Unlocked(String),
}

static VAULT_UNLOCK: std::sync::OnceLock<std::sync::Mutex<VaultUnlock>> =
    std::sync::OnceLock::new();

fn vault_unlock_state() -> &'static std::sync::Mutex<VaultUnlock> {
    VAULT_UNLOCK.get_or_init(|| std::sync::Mutex::new(VaultUnlock::Unset))
}

/// The passphrase this process would use to open the vault: an explicit
/// runtime unlock first, otherwise `FLEET_VAULT_PASSPHRASE`.
pub fn vault_passphrase() -> Option<String> {
    let state = vault_unlock_state()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    match &*state {
        VaultUnlock::Unlocked(passphrase) => Some(passphrase.clone()),
        VaultUnlock::Locked => None,
        VaultUnlock::Unset => std::env::var("FLEET_VAULT_PASSPHRASE")
            .ok()
            .filter(|passphrase| !passphrase.is_empty()),
    }
}

/// Hold `passphrase` for this process until it exits or [`lock_vault`].
pub fn unlock_vault(passphrase: &str) {
    let mut state = vault_unlock_state()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    *state = VaultUnlock::Unlocked(passphrase.to_string());
}

/// Stop using the vault for this process, even when the environment
/// variable is present.
pub fn lock_vault() {
    let mut state = vault_unlock_state()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    *state = VaultUnlock::Locked;
}

/// Put the process back on the launch-time source. Tests use this so one
/// global unlock does not leak into the next test.
#[cfg(test)]
pub fn reset_vault_unlock() {
    let mut state = vault_unlock_state()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    *state = VaultUnlock::Unset;
}

/// The dotenv with this process's own vault unlock laid over it.
pub fn read_dotenv_with_process_vault(home: &Path) -> HashMap<String, String> {
    read_dotenv_with_vault(home, vault_passphrase().as_deref())
}

/// Read `<home>/fleet.env` into a map: the env-var counterparts the
/// settings writer owns, beside `.env` rather than inside it. Same
/// KEY=VALUE grammar as [`read_dotenv`], because the two files are the
/// same grammar — one is hand-edited credentials, the other is the
/// settings surface's env-var writes, and the runtime now reads both.
pub fn read_fleet_env(home: &Path) -> HashMap<String, String> {
    read_env_file(&home.join("fleet.env"))
}

/// Create `home` and its sessions directory when missing.
/// The sessions directory path.
pub fn ensure_sessions_dir(home: &Path) -> std::io::Result<PathBuf> {
    let dir = sessions_dir(home);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Read `home/.env` into a map: `KEY=VALUE` lines, `#` comments, surrounding
/// whitespace trimmed. A missing file is an empty map; malformed lines are
/// skipped rather than fatal because the file is a convenience, not a config
/// contract.
pub fn read_dotenv(home: &Path) -> HashMap<String, String> {
    read_env_file(&home.join(".env"))
}

/// The KEY=VALUE grammar, over one file's own path. A missing file is
/// an empty map; malformed lines are skipped rather than fatal because
/// the file is a convenience, not a config contract.
fn read_env_file(path: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let Ok(text) = fs::read_to_string(path) else {
        return values;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(name.trim().to_string(), value.trim().to_string());
    }
    values
}

/// Resolve a setting: the process environment wins, `home/.env` is the
/// fallback, and an empty value counts as unset. Fleet's own home is
/// the whole story — no other harness's config is ever read (the
/// owner's 2026-10-16 ruling).
pub fn resolve_setting(name: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    let from_env = std::env::var(name).ok().filter(|value| !value.is_empty());
    from_env.or_else(|| dotenv.get(name).filter(|value| !value.is_empty()).cloned())
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_vault_lays_over_the_dotenv_and_a_missing_passphrase_leaves_it_plain() {
        use crate::vault::Vault;
        let dir = std::env::temp_dir().join(format!("fleet-vault-overlay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The dotenv carries a stale value for a name the vault also
        // holds, and a name the vault does not.
        std::fs::write(
            dir.join(".env"),
            "STEPTEST_API_KEY=plaintext-stale\nOTHER_NAME=other-value\n",
        )
        .unwrap();
        let mut vault = Vault::init();
        vault.set("passphrase", "STEPTEST_API_KEY", "sealed-fresh");
        std::fs::write(dir.join(crate::vault::VAULT_FILE), vault.to_json()).unwrap();

        // The sealed value answers over the stale plaintext line.
        let overlaid = read_dotenv_with_vault(&dir, Some("passphrase"));
        assert_eq!(
            overlaid.get("STEPTEST_API_KEY").map(String::as_str),
            Some("sealed-fresh")
        );
        assert_eq!(
            overlaid.get("OTHER_NAME").map(String::as_str),
            Some("other-value")
        );

        // Without the passphrase the map is the plain dotenv: the stale
        // plaintext answers, and nothing fails.
        let plain = read_dotenv_with_vault(&dir, None);
        assert_eq!(
            plain.get("STEPTEST_API_KEY").map(String::as_str),
            Some("plaintext-stale")
        );

        // A wrong passphrase opens nothing: the plain dotenv answers.
        let wrong = read_dotenv_with_vault(&dir, Some("not the passphrase"));
        assert_eq!(
            wrong.get("STEPTEST_API_KEY").map(String::as_str),
            Some("plaintext-stale")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
    use super::*;

    #[test]
    fn an_absent_or_empty_soul_is_no_identity() {
        let dir = std::env::temp_dir().join(format!("fleet-soul-absent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(read_soul(&dir).is_none(), "no file is no identity");
        std::fs::write(dir.join("SOUL.md"), "   \n\n  ").unwrap();
        assert!(read_soul(&dir).is_none(), "an empty file is no identity");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_soul_is_sanitized_bom_and_control_characters_removed() {
        let dir = std::env::temp_dir().join(format!("fleet-soul-clean-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let raw = "\u{feff}You are Fleet.\n\nBe direct.\u{1}\n\tKeep the newline and tab.\n";
        std::fs::write(dir.join("SOUL.md"), raw).unwrap();
        let soul = read_soul(&dir).expect("the file loads");
        assert!(!soul.contains('\u{feff}'), "{soul:?}");
        assert!(
            !soul.contains('\u{1}'),
            "the control character is gone, not replaced: {soul:?}"
        );
        assert!(soul.starts_with("You are Fleet."));
        assert!(soul.contains('\n'));
        assert!(soul.contains('\t'));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_oversized_soul_is_bounded_with_a_marked_cut() {
        let dir = std::env::temp_dir().join(format!("fleet-soul-big-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let body = "x".repeat(MAX_SOUL_CHARS + 500);
        std::fs::write(dir.join("SOUL.md"), &body).unwrap();
        let soul = read_soul(&dir).expect("the file loads");
        assert!(soul.contains("[truncated at 8192 characters]"), "{soul:?}");
        assert!(
            soul.chars().count() < MAX_SOUL_CHARS + 100,
            "the body is cut at the bound, not carried whole: {}",
            soul.chars().count()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Set `name` for the duration of the guard, restoring whatever was
    /// there before — including its absence — on the way out.
    struct EnvVar {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVar {
        fn set(name: &str, value: &str) -> EnvVar {
            let previous = std::env::var_os(name);
            std::env::set_var(name, value);
            EnvVar {
                name: name.to_string(),
                previous,
            }
        }
    }

    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(&self.name, value),
                None => std::env::remove_var(&self.name),
            }
        }
    }

    #[test]
    fn home_prefers_fleet_home_env() {
        let home = fleet_home(Some("/tmp/custom-fleet"), Some("/Users/x")).unwrap();
        assert_eq!(home, PathBuf::from("/tmp/custom-fleet"));
    }

    #[test]
    fn home_defaults_to_dot_fleet_under_the_user_home() {
        let home = fleet_home(None, Some("/Users/x")).unwrap();
        assert_eq!(home, PathBuf::from("/Users/x/.fleet"));
    }

    #[test]
    fn home_fails_loud_without_any_home() {
        let error = fleet_home(Some("  "), None).unwrap_err();
        assert!(error.contains("FLEET_HOME"));
    }

    #[test]
    fn dotenv_skips_comments_and_malformed_lines() {
        let dir = std::env::temp_dir().join(format!("fleet-home-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(".env"),
            "# comment\nDEEPSEEK_API_KEY = sk-test\nBROKEN LINE\n\nEMPTY=\n",
        )
        .unwrap();
        let values = read_dotenv(&dir);
        assert_eq!(
            values.get("DEEPSEEK_API_KEY").map(String::as_str),
            Some("sk-test")
        );
        // An empty value is kept in the map; resolution treats it as unset.
        assert_eq!(values.get("EMPTY").map(String::as_str), Some(""));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn resolve_setting_lets_the_environment_win() {
        let mut dotenv = HashMap::new();
        dotenv.insert("FLEET_TEST_VAR".to_string(), "from-dotenv".to_string());
        // Not set in this process: the dotenv fallback applies.
        if std::env::var("FLEET_TEST_VAR").is_err() {
            assert_eq!(
                resolve_setting("FLEET_TEST_VAR", &dotenv).as_deref(),
                Some("from-dotenv")
            );
        }
        assert_eq!(resolve_setting("FLEET_TEST_UNSET_VAR_XYZ", &dotenv), None);
    }

    /// The owner's 2026-10-16 ruling, pinned: a key that lives only in
    /// another harness's store does not resolve, even when that store's
    /// home is named in the environment.
    #[test]
    fn resolve_setting_reads_fleets_home_and_no_other_harnesss_store() {
        // The env-mutating test serializes on the same lock the other
        // modules use; `cargo test` runs a module's tests on threads.
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());

        let name = "FLEET_TEST_HARNESS_ONLY_KEY";
        // Its own directory: the sibling dotenv test shares the pid-keyed
        // path, and `cargo test` runs a module's tests on threads, so two
        // tests writing and removing one directory race each other.
        let home = std::env::temp_dir().join(format!("fleet-home-own-{}", std::process::id()));
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(".env"), "OWNED_KEY=from-fleets-own-env\n").unwrap();

        // A neighboring harness store that names this key, its home
        // named in the environment exactly as the removed leg read it.
        let dsh = std::env::temp_dir().join(format!("fleet-dsh-test-{}", std::process::id()));
        fs::create_dir_all(&dsh).unwrap();
        fs::write(
            dsh.join(".credentials.yaml"),
            format!("version: 1\nrefs:\n  {name}: from-the-other-harness\n"),
        )
        .unwrap();
        let _dsh_home = EnvVar::set("DSH_HOME", &dsh.display().to_string());

        let dotenv = read_dotenv(&home);
        assert_eq!(resolve_setting(name, &dotenv), None);
        assert_eq!(
            resolve_setting("OWNED_KEY", &dotenv).as_deref(),
            Some("from-fleets-own-env")
        );

        fs::remove_dir_all(&home).ok();
        fs::remove_dir_all(&dsh).ok();
    }
}
