//! `fleet doctor` probes. Each check evaluates facts the caller gathered, so
//! every evaluator runs hermetically in tests.

use std::path::Path;

use crate::discovery::{self, HARNESS_ENTRY};
use crate::nodever::{self, NODE_FLOOR};

/// One probe's outcome.
pub struct Check {
    /// Short check name, e.g. `node`.
    pub name: &'static str,
    /// Whether the check passed.
    pub ok: bool,
    /// Human-readable detail, e.g. the observed version.
    pub detail: String,
}

/// Evaluates the node check from a `node --version` outcome.
pub fn evaluate_node(version: Result<(u64, u64, u64), String>) -> Check {
    match version {
        Err(error) => Check {
            name: "node",
            ok: false,
            detail: error,
        },
        Ok(version) => {
            let (major, minor, patch) = version;
            if nodever::meets_floor(version) {
                Check {
                    name: "node",
                    ok: true,
                    detail: format!("v{major}.{minor}.{patch} satisfies ^{}.{:02} || >=24", NODE_FLOOR.0, NODE_FLOOR.1),
                }
            } else {
                Check {
                    name: "node",
                    ok: false,
                    detail: format!(
                        "v{major}.{minor}.{patch} is out of range; Fleet needs ^{}.{:02} || >=24",
                        NODE_FLOOR.0, NODE_FLOOR.1
                    ),
                }
            }
        }
    }
}

/// Evaluates the checkout check from a resolved root, when discovery found one.
pub fn evaluate_harness(root: Option<&Path>) -> Check {
    match root {
        Some(root) if discovery::looks_like_harness(root) => Check {
            name: "harness",
            ok: true,
            detail: root.display().to_string(),
        },
        Some(root) => Check {
            name: "harness",
            ok: false,
            detail: format!("{root:?} does not carry {HARNESS_ENTRY}"),
        },
        None => Check {
            name: "harness",
            ok: false,
            detail: "no checkout found; set --home or FLEET_HOME, or run inside the repository"
                .to_string(),
        },
    }
}

/// Evaluates the installed-dependencies check: `node_modules` at the checkout root.
pub fn evaluate_deps(root: &Path) -> Check {
    let node_modules = root.join("node_modules");
    if node_modules.is_dir() {
        Check {
            name: "dependencies",
            ok: true,
            detail: "node_modules present".to_string(),
        }
    } else {
        Check {
            name: "dependencies",
            ok: false,
            detail: format!("{} is missing; run `pnpm install` in the checkout", node_modules.display()),
        }
    }
}

/// Evaluates the dsh home check: the profiles directory and the profiles it holds.
pub fn evaluate_profiles(dsh_home: &Path) -> Check {
    let profiles = dsh_home.join("profiles");
    let entries = match std::fs::read_dir(&profiles) {
        Err(_) => {
            return Check {
                name: "dsh home",
                ok: false,
                detail: format!("{} does not exist; boot the harness once to create it", profiles.display()),
            }
        }
        Ok(entries) => entries,
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        // A profile carries cordis.yml; other directories (pnpm's
        // node_modules farm, for instance) are not profiles.
        .filter(|entry| entry.path().join("cordis.yml").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    if names.is_empty() {
        Check {
            name: "dsh home",
            ok: false,
            detail: format!("{} holds no profiles", profiles.display()),
        }
    } else {
        Check {
            name: "dsh home",
            ok: true,
            detail: format!("{} ({}: {})", profiles.display(), names.len(), names.join(", ")),
        }
    }
}

/// Evaluates the web UI check from a [`crate::net::listening`] probe. Doctor
/// reports it as informational; it never fails the run on its own.
pub fn evaluate_web(listening: bool, host: &str, port: u16) -> Check {
    if listening {
        Check {
            name: "web ui",
            ok: true,
            detail: format!("listening on http://{host}:{port}"),
        }
    } else {
        Check {
            name: "web ui",
            ok: false,
            detail: format!("nothing listening on {host}:{port}"),
        }
    }
}

/// Renders a check as ` ok  name - detail` / `FAIL  name - detail`.
pub fn render(check: &Check) -> String {
    let verdict = if check.ok { " ok " } else { "FAIL" };
    format!("{verdict}  {} - {}", check.name, check.detail)
}
