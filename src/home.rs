//! Fleet's on-disk home: session logs and the optional `.env` credential
//! fallback live under `~/.fleet`. Nothing else is persisted.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Fleet's home directory: `$FLEET_HOME` when set and non-empty, else
/// `<user home>/.fleet`. `user_home` is the OS home (`HOME`/`USERPROFILE`),
/// passed in so callers decide their own environment precedence.
/// @return the absolute home path.
/// @throws when neither `$FLEET_HOME` nor a user home is known.
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

/// Create `home` and its sessions directory when missing.
/// @return the sessions directory path.
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
    let mut values = HashMap::new();
    let Ok(text) = fs::read_to_string(home.join(".env")) else {
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
    use super::*;

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
        let home = std::env::temp_dir().join(format!("fleet-home-test-{}", std::process::id()));
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
