//! Where Fleet finds the harness checkout, the user home, and the dsh data home.

use std::path::{Path, PathBuf};

/// The harness CLI entry identifying a checkout; the path the launcher boots.
pub const HARNESS_ENTRY: &str = "apps/cli/src/bin.ts";

/// True when `dir` carries the harness CLI entry.
pub fn looks_like_harness(dir: &Path) -> bool {
    dir.join(HARNESS_ENTRY).is_file()
}

/// Resolves the harness checkout from the first candidate that carries the
/// CLI entry: the `--home` flag, then `FLEET_HOME`, then the installed
/// checkout under the user home, then the current directory.
pub fn resolve_root(
    flag: Option<&Path>,
    env_home: Option<&str>,
    installed: Option<PathBuf>,
    cwd: &Path,
) -> Result<PathBuf, String> {
    let candidates = [
        flag.map(|path| path.to_path_buf()),
        env_home.map(PathBuf::from),
        installed,
        Some(cwd.to_path_buf()),
    ];
    let mut tried = Vec::new();
    for candidate in candidates.into_iter().flatten() {
        if looks_like_harness(&candidate) {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    Err(format!(
        "fleet: no DeepSeek Harness checkout found; looked at:\n  {}",
        tried.join("\n  ")
    ))
}

/// The installed-checkout location under a user home: `<home>/.fleet/harness`.
pub fn installed_home(home: &Path) -> PathBuf {
    home.join(".fleet").join("harness")
}

/// The user home directory: `HOME` on Unix, `USERPROFILE` on Windows.
pub fn home_dir(env_home: Option<&str>, env_userprofile: Option<&str>) -> Option<PathBuf> {
    env_home
        .map(PathBuf::from)
        .or_else(|| env_userprofile.map(PathBuf::from))
}

/// The dsh data home: `DSH_HOME` when set, otherwise `<user home>/.dsh`.
pub fn dsh_home(env: Option<&str>, user_home: Option<&Path>) -> Option<PathBuf> {
    env.map(PathBuf::from)
        .or_else(|| user_home.map(|home| home.join(".dsh")))
}
