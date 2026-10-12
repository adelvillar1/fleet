//! Node binary resolution and the supported-version floor.

use std::process::Command;

/// Supported node range, mirroring the repository's `engines` field: `^22.19 || >=24`.
pub const NODE_FLOOR: (u64, u64) = (22, 19);

/// Parses `node --version` output (`v22.19.0`) into (major, minor, patch).
/// Tolerates prerelease and build suffixes on the patch component.
pub fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim();
    let trimmed = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0");
    let patch = patch.split(['-', '+']).next().unwrap_or("0");
    let patch = patch.parse().ok()?;
    Some((major, minor, patch))
}

/// True when `version` satisfies `^22.19 || >=24`; node 23 and older lines are out.
pub fn meets_floor(version: (u64, u64, u64)) -> bool {
    let (major, minor, _) = version;
    if major > NODE_FLOOR.0 {
        major >= 24
    } else if major == NODE_FLOOR.0 {
        minor >= NODE_FLOOR.1
    } else {
        false
    }
}

/// Runs `<node> --version` and parses the result.
pub fn node_version(node: &str) -> Result<(u64, u64, u64), String> {
    let output = Command::new(node)
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run {node}: {error}"))?;
    if !output.status.success() {
        let code = output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "a signal".to_string());
        return Err(format!("{node} --version exited with {code}"));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    parse_version(&raw)
        .ok_or_else(|| format!("{node} --version printed an unrecognized version: {raw:?}"))
}
