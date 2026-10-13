//! `fleet doctor` checks: credentials, home, API reachability, shell.
//! Evaluators take facts the caller gathered, so every evaluator runs
//! hermetically in tests.

use std::path::Path;

/// One probe's outcome.
pub struct Check {
    /// Short check name, e.g. `credentials`.
    pub name: &'static str,
    /// Whether the check passed.
    pub ok: bool,
    /// Human-readable detail, e.g. the masked key.
    pub detail: String,
}

/// Render one check as a report line.
pub fn render(check: &Check) -> String {
    let verdict = if check.ok { "ok  " } else { "FAIL" };
    format!("{:12} {}  {}", check.name, verdict, check.detail)
}

/// The credentials check from a resolved `DEEPSEEK_API_KEY` (after the
/// `.env` and Harness credential-store fallbacks). The detail masks
/// everything but the last four characters.
pub fn evaluate_credentials(key: Option<&str>) -> Check {
    match key {
        None => Check {
            name: "credentials",
            ok: false,
            detail: "DEEPSEEK_API_KEY is not set; export it, add it to ~/.fleet/.env, or store it in ~/.dsh/.credentials.yaml".to_string(),
        },
        Some(key) => Check {
            name: "credentials",
            ok: true,
            detail: format!("set ({})", mask(key)),
        },
    }
}

/// `…last4` for anything longer than a plausible short prefix; short values
/// are masked entirely. Operates on chars, not bytes: a key is arbitrary text
/// until the API says otherwise, and byte-slicing 4-from-the-end panics when
/// the index straddles a multi-byte char (logged as KI-010).
fn mask(key: &str) -> String {
    if key.chars().count() > 8 {
        let last_four: String =
            key.chars().rev().take(4).collect::<Vec<char>>().into_iter().rev().collect();
        format!("\u{2026}{last_four}")
    } else {
        "\u{2026}".to_string()
    }
}

/// The home check from an `ensure_sessions_dir` outcome with its error
/// already rendered.
pub fn evaluate_home(home: Option<&Path>, outcome: Result<(), String>) -> Check {
    match (home, outcome) {
        (None, _) => Check {
            name: "home",
            ok: false,
            detail: "no home directory known; set FLEET_HOME or HOME".to_string(),
        },
        (Some(home), Ok(())) => Check {
            name: "home",
            ok: true,
            detail: format!("writable ({})", home.display()),
        },
        (Some(home), Err(error)) => Check {
            name: "home",
            ok: false,
            detail: format!("{}: {error}", home.display()),
        },
    }
}

/// The API reachability check from a probe outcome against the parsed
/// endpoint.
pub fn evaluate_api(endpoint: Option<(&str, u16)>, reachable: bool) -> Check {
    match (endpoint, reachable) {
        (None, _) => Check {
            name: "api",
            ok: false,
            detail: "DEEPSEEK_BASE_URL is not a host[:port] URL".to_string(),
        },
        (Some((host, port)), true) => Check {
            name: "api",
            ok: true,
            detail: format!("reachable ({host}:{port})"),
        },
        (Some((host, port)), false) => Check {
            name: "api",
            ok: false,
            detail: format!("{host}:{port} unreachable; check network or DEEPSEEK_BASE_URL"),
        },
    }
}

/// Parse an API base URL into `(host, port)`, defaulting the port by scheme.
pub fn api_endpoint(base_url: &str) -> Option<(&str, u16)> {
    let (scheme, rest) = base_url.split_once("://")?;
    let authority = rest.split(['/']).next()?;
    if authority.is_empty() {
        return None;
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host, port.parse().ok()?)),
        None => match scheme {
            "https" => Some((authority, 443)),
            "http" => Some((authority, 80)),
            _ => None,
        },
    }
}

/// The shell check from a probe outcome.
pub fn evaluate_shell(probe: Result<&'static str, String>) -> Check {
    match probe {
        Ok(shell) => Check { name: "shell", ok: true, detail: format!("{shell} available") },
        Err(error) => Check { name: "shell", ok: false, detail: error },
    }
}

/// The calibration-stream line, informational and never a check: the
/// resolved root and whether today's file exists. It is the cheapest
/// guard against dev-decisions moving its `LOG_DIR` literal and silently
/// re-routing Fleet's stream — the root is named where an operator looks.
/// It never touches the exit code.
pub fn render_calib(root: Option<&Path>, today_file_exists: bool) -> String {
    let detail = match root {
        Some(root) => {
            let status = if today_file_exists { "present" } else { "absent" };
            format!("{} (today's file: {})", root.display(), status)
        }
        None => "no home directory known; set FLEET_HOME or HOME".to_string(),
    };
    format!("{:12} {}  {}", "calib", "info", detail)
}

/// The recall-path check: which retrieval path is live. It passes in
/// **both** states — a keyword-only recall is a working configuration,
/// not a failure, and a clean machine has no local embedding model — and
/// its detail names the path. When the vector path is absent, the one
/// named line says so: that is the live system's failure this check
/// exists to fix, where an absent embedding stack silently degraded
/// recall over 11k entries. The line is never silent.
///
/// `vector` is `(endpoint, dims)` when the probe found a live local
/// model, `None` for the keyword-only degradation.
pub fn evaluate_recall(vector: Option<(&str, usize)>) -> Check {
    let detail = match vector {
        Some((endpoint, dims)) => {
            format!("vector path live \u{2014} {endpoint} ({dims}-dim embeddings serve recall)")
        }
        None => "embeddings unavailable \u{2014} recall is keyword-only".to_string(),
    };
    Check { name: "recall", ok: true, detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_masks_but_never_hides_presence() {
        let ok = evaluate_credentials(Some("sk-1234567890abcdef"));
        assert!(ok.ok);
        assert!(ok.detail.starts_with("set (\u{2026}"));
        assert!(!ok.detail.contains("sk-1234567890"));
        let short = evaluate_credentials(Some("short"));
        assert_eq!(short.detail, "set (\u{2026})");
        let missing = evaluate_credentials(None);
        assert!(!missing.ok);
        assert!(missing.detail.contains("DEEPSEEK_API_KEY"));
    }

    #[test]
    fn credentials_mask_never_panics_on_multibyte_keys() {
        // The byte-slicing mask panicked when the 4-from-end index straddled a
        // multi-byte char (KI-010); a key is arbitrary text until the API says
        // otherwise, and doctor is the last place a panic belongs.
        let crafted = "aaaa\u{1d54f}\u{e9}";
        let check = evaluate_credentials(Some(crafted));
        assert!(check.ok);
        assert!(!check.detail.contains('\u{1d54f}'), "{}", check.detail);
        assert!(check.detail.starts_with("set (\u{2026}"));
        let long = "sk-1234567890\u{e9}";
        let check = evaluate_credentials(Some(long));
        // The wrapper is `set (…)`, so the masked tail sits just inside the paren.
        assert!(check.detail.ends_with("890\u{e9})"), "{}", check.detail);
        assert!(!check.detail.contains("sk-1234567890"));
    }

    #[test]
    fn api_endpoint_parses_hosts_and_ports() {
        assert_eq!(api_endpoint("https://api.deepseek.com"), Some(("api.deepseek.com", 443)));
        assert_eq!(api_endpoint("http://127.0.0.1:4141/v1"), Some(("127.0.0.1", 4141)));
        assert_eq!(api_endpoint("https://host:notaport"), None);
        assert_eq!(api_endpoint("ftp://host"), None);
    }

    #[test]
    fn render_aligns_names_and_verdicts() {
        let line = render(&evaluate_shell(Ok("sh")));
        assert!(line.starts_with("shell        ok    sh available"), "{line}");
        let line = render(&evaluate_shell(Err("no shell".to_string())));
        assert!(line.contains("FAIL"), "{line}");
    }

    #[test]
    fn the_calib_line_names_the_resolved_root() {
        // A scratch home with no config.json and no user home resolves to
        // the <FLEET_HOME>/logs fallback; the informational line names it
        // and says today's file is absent, without touching any check.
        let home = std::path::Path::new("/scratch/fleet-home");
        let root = crate::calib::resolve_root(home, None);
        assert_eq!(root, std::path::Path::new("/scratch/fleet-home/logs"));
        let line = render_calib(Some(&root), false);
        assert!(line.contains("calib"), "{line}");
        assert!(line.contains("/scratch/fleet-home/logs"), "{line}");
        assert!(line.contains("absent"), "{line}");
        // A present file is named present; a missing home says so plainly.
        assert!(render_calib(Some(&root), true).contains("present"));
        assert!(render_calib(None, false).contains("no home directory known"));
    }

    #[test]
    fn the_recall_check_names_the_live_path_both_ways() {
        // The vector path: the endpoint and the dimensionality the probe
        // found are named where an operator looks.
        let live = evaluate_recall(Some(("http://127.0.0.1:8901/v1/embeddings", 768)));
        assert!(live.ok);
        assert!(live.detail.contains("vector path live"), "{}", live.detail);
        assert!(live.detail.contains("768"), "{}", live.detail);
        // The degradation: one named line, never silent.
        let degraded = evaluate_recall(None);
        assert!(degraded.ok);
        assert_eq!(degraded.detail, "embeddings unavailable \u{2014} recall is keyword-only");
        // Both states pass: keyword-only recall is a working
        // configuration, and doctor is the install gate — a clean
        // machine with no local model must stay green.
        assert!(render(&degraded).starts_with("recall"), "{}", render(&degraded));
    }
}
