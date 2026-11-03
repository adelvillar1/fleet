//! The plugin evaluation suite (the face-plugin-host plan's C8): two
//! layers, deterministic first. The **static pass** — the manifest's
//! conformance, the bundle's credential sweep (the face's own payload
//! sweep class: a bundle naming a key shape is refused), and the
//! asset's basic sanity — runs always and never spends. The **declared
//! pass** drives each of the manifest's queries once through the
//! bridge's enforcement, so the sandbox's data path is exercised, not
//! asserted. The suite grades; it never installs.
//!
//! The rendered half — the browser lane loading the bundle and naming
//! what painted and what errored — is the plan's live leg, exercised
//! once against a real bundle; the route's network-dead CSP is the
//! canvas's own tested constant.

use std::path::Path;

use super::face::{FaceManifest, READ_ONLY_QUERIES};

/// One check's verdict: pass, fail (naming the reason), or skip (the
/// check does not apply to this tier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail(String),
    Skip,
}

/// One named check in the report.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub verdict: Verdict,
}

/// The suite's report over one bundle.
#[derive(Debug, Clone)]
pub struct Report {
    pub name: String,
    pub checks: Vec<Check>,
}

impl Report {
    /// Every non-skipped check passed.
    pub fn passed(&self) -> bool {
        self.checks
            .iter()
            .all(|check| matches!(check.verdict, Verdict::Pass | Verdict::Skip))
    }

    /// The pass count over the checks that ran.
    pub fn tally(&self) -> (usize, usize) {
        let ran = self
            .checks
            .iter()
            .filter(|check| !matches!(check.verdict, Verdict::Skip))
            .count();
        let passed = self
            .checks
            .iter()
            .filter(|check| matches!(check.verdict, Verdict::Pass))
            .count();
        (passed, ran)
    }
}

/// The credential-sweep class, applied to a bundle's text: the face's
/// own payload sweep refuses any payload naming these shapes, and a
/// plugin bundle is a payload the face serves.
const SECRET_SHAPES: [&str; 3] = ["sk-", "Authorization", "Bearer"];

/// The static pass over an installed bundle. The manifest's own parse
/// covers the schema; this adds the sweep and the asset sanity.
pub fn static_pass(home: &Path, name: &str) -> Report {
    let dir = super::face::plugin_dir(home, name);
    let mut checks = Vec::new();
    let manifest_path = FaceManifest::path(&dir);
    let manifest = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|text| FaceManifest::parse(&text).ok());
    let Some(manifest) = manifest else {
        return Report {
            name: name.to_string(),
            checks: vec![Check {
                name: "manifest".to_string(),
                verdict: Verdict::Fail(
                    "the manifest does not parse — the schema's refusal names the field"
                        .to_string(),
                ),
            }],
        };
    };
    checks.push(Check {
        name: "manifest".to_string(),
        verdict: Verdict::Pass,
    });
    // The credential sweep: every file in the bundle, scanned.
    let mut swept = Vec::new();
    sweep_files(&dir, &mut swept);
    let mut secret_hits = Vec::new();
    for path in &swept {
        if let Ok(text) = std::fs::read_to_string(path) {
            for shape in SECRET_SHAPES {
                if text.contains(shape) {
                    secret_hits.push(format!(
                        "{} names `{shape}`",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
    }
    checks.push(Check {
        name: "the credential sweep".to_string(),
        verdict: if secret_hits.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Fail(secret_hits.join("; "))
        },
    });
    // The asset's sanity: an index.html, bounded.
    let index = dir.join("index.html");
    match std::fs::read(&index) {
        Ok(bytes) if bytes.is_empty() => checks.push(Check {
            name: "the asset".to_string(),
            verdict: Verdict::Fail("index.html is empty".to_string()),
        }),
        Ok(bytes) if bytes.len() > 2_000_000 => checks.push(Check {
            name: "the asset".to_string(),
            verdict: Verdict::Fail(format!(
                "index.html is {} bytes — past the 2 MB bound",
                bytes.len()
            )),
        }),
        Ok(_) => checks.push(Check {
            name: "the asset".to_string(),
            verdict: Verdict::Pass,
        }),
        Err(_) => checks.push(Check {
            name: "the asset".to_string(),
            verdict: Verdict::Fail("the bundle carries no index.html".to_string()),
        }),
    }
    // The declared queries resolve — the manifest's parse already
    // refused an unknown name; this records the count as the sandbox's
    // breadth.
    checks.push(Check {
        name: "the declared queries".to_string(),
        verdict: if manifest.queries.is_empty() {
            Verdict::Skip
        } else {
            Verdict::Pass
        },
    });
    let _ = READ_ONLY_QUERIES;
    Report {
        name: name.to_string(),
        checks,
    }
}

fn sweep_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sweep_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The declared pass: each of the manifest's queries driven once
/// through the bridge's enforcement. The answers are the bridge's own
/// reads; the point is that every declared query RESOLVES — a
/// declared query the bridge cannot answer is a find, not a promise.
pub async fn declared_pass(bridge: &crate::gui::bridge::Bridge, home: &Path, name: &str) -> Report {
    let dir = super::face::plugin_dir(home, name);
    let manifest_path = FaceManifest::path(&dir);
    let mut checks = Vec::new();
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return Report {
            name: name.to_string(),
            checks: vec![Check {
                name: "the manifest".to_string(),
                verdict: Verdict::Fail("the manifest does not parse".to_string()),
            }],
        };
    };
    let Ok(manifest) = FaceManifest::parse(&text) else {
        return Report {
            name: name.to_string(),
            checks: vec![Check {
                name: "the manifest".to_string(),
                verdict: Verdict::Fail("the manifest does not parse".to_string()),
            }],
        };
    };
    for query in &manifest.queries {
        let check = match bridge.plugin_query(name, query).await {
            Ok(_) => Check {
                name: format!("the query `{query}` answers"),
                verdict: Verdict::Pass,
            },
            Err(message) => Check {
                name: format!("the query `{query}` answers"),
                verdict: Verdict::Fail(message),
            },
        };
        checks.push(check);
    }
    if manifest.queries.is_empty() {
        checks.push(Check {
            name: "the declared queries".to_string(),
            verdict: Verdict::Skip,
        });
    }
    Report {
        name: name.to_string(),
        checks,
    }
}

/// The report as the CLI renders it: one line per check, the tally
/// closing.
pub fn render(report: &Report) -> String {
    let mut out = format!("fleet plugins eval — {}\n", report.name);
    for check in &report.checks {
        let word = match &check.verdict {
            Verdict::Pass => "pass",
            Verdict::Fail(reason) => {
                out.push_str(&format!("  fail  {} — {reason}\n", check.name));
                continue;
            }
            Verdict::Skip => "skip",
        };
        out.push_str(&format!("  {word}  {}\n", check.name));
    }
    let (passed, ran) = report.tally();
    out.push_str(&format!(
        "{passed}/{ran} checks pass{}\n",
        if report.passed() {
            ""
        } else {
            " — the promotion is refused"
        }
    ));
    out
}
