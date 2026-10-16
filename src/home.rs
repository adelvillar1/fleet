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
/// fallback, and an empty value counts as unset.
pub fn resolve_setting(name: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    let from_env = std::env::var(name).ok().filter(|value| !value.is_empty());
    from_env.or_else(|| dotenv.get(name).filter(|value| !value.is_empty()).cloned())
}

/// The DeepSeek Harness credentials store: `$DSH_HOME/.credentials.yaml`,
/// else `<user home>/.dsh/.credentials.yaml`. One stored key serves both
/// harnesses; Fleet only ever reads it.
pub fn harness_credentials_path(
    dsh_home_env: Option<&str>,
    user_home: Option<&str>,
) -> Option<PathBuf> {
    match dsh_home_env
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => Some(PathBuf::from(value).join(".credentials.yaml")),
        None => user_home.map(|home| PathBuf::from(home).join(".dsh").join(".credentials.yaml")),
    }
}

/// Read `refs.<name>` from the harness credentials store's text. The `refs`
/// block is a flat `NAME: value` map at one indent level, so a line scan
/// stands in for a YAML dependency; values are unquoted. A missing file,
/// block, or name, or an empty value, is `None`.
pub fn harness_refs(text: &str, name: &str) -> Option<String> {
    let mut in_refs = false;
    for line in text.lines() {
        if !in_refs {
            if line.trim_end() == "refs:" {
                in_refs = true;
            }
            continue;
        }
        if !line.starts_with(' ') {
            // A top-level key ends the block.
            return None;
        }
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        if key.trim() == name {
            let value = value.trim();
            let unquoted = value.trim_matches(|c| c == '\'' || c == '"');
            return (!unquoted.is_empty()).then(|| unquoted.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn harness_credentials_path_honors_dsh_home() {
        let path = harness_credentials_path(Some("/tmp/dsh"), Some("/Users/x")).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/dsh/.credentials.yaml"));
        let path = harness_credentials_path(None, Some("/Users/x")).unwrap();
        assert_eq!(path, PathBuf::from("/Users/x/.dsh/.credentials.yaml"));
        assert_eq!(harness_credentials_path(Some("  "), None), None);
    }

    #[test]
    fn harness_refs_reads_the_refs_block_and_stops_at_the_next_key() {
        let text = "version: 1\nrefs:\n  DEEPSEEK_API_KEY: 'sk-test'\n  UNQUOTED: plain-value\nrecords:\n  something:\n    DEEPSEEK_API_KEY: 'decoy'\n";
        assert_eq!(
            harness_refs(text, "DEEPSEEK_API_KEY").as_deref(),
            Some("sk-test")
        );
        assert_eq!(
            harness_refs(text, "UNQUOTED").as_deref(),
            Some("plain-value")
        );
        assert_eq!(harness_refs(text, "MISSING"), None);
        assert_eq!(
            harness_refs(
                "records:\n  DEEPSEEK_API_KEY: 'decoy'\n",
                "DEEPSEEK_API_KEY"
            ),
            None
        );
        assert_eq!(harness_refs("refs:\n  EMPTY: ''\n", "EMPTY"), None);
    }
}
