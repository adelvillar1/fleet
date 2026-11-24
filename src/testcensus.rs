//! The census: every module file the crate compiles, in one checked table.
//!
//! The beat guards (judge, delegate, explain) used to scan a hand-written
//! list of modules. That list could only ever be as complete as the last
//! session remembered to make it: a module added to `lib.rs` was scanned
//! by nobody until someone edited three arrays. This module is the fix —
//! one table of every module file, asserted equal to the crate's own
//! module tree, so a new module is accounted for the moment it lands.
//!
//! The table is `include_str!`-literal because that is the only way a
//! source scan can be built at compile time (and the only way a missing
//! file is a compile error rather than a silent skip). The assertion that
//! the table matches the tree is [`the_census_matches_the_crates_module_tree`]
//! below: it walks `lib.rs` and every nested `mod` declaration, resolves
//! each to `src/<path>.rs` or `src/<path>/mod.rs`, and compares the two
//! sets in both directions — a file in the crate but not the table names
//! itself in the failure, and so does a stale table entry.

#![cfg(test)]

/// Every module file under `src/`, by path relative to it — the crate's
/// own tree, walked from `lib.rs`, flattened into the literal list
/// `include_str!` requires.
pub(crate) const MODULES: &[(&str, &str)] = &[
    ("agent.rs", include_str!("agent.rs")),
    ("anomalies.rs", include_str!("anomalies.rs")),
    ("api.rs", include_str!("api.rs")),
    ("apply_patch.rs", include_str!("apply_patch.rs")),
    ("approvals.rs", include_str!("approvals.rs")),
    ("armada/authority.rs", include_str!("armada/authority.rs")),
    ("armada/control.rs", include_str!("armada/control.rs")),
    ("armada/deploy.rs", include_str!("armada/deploy.rs")),
    ("armada/join.rs", include_str!("armada/join.rs")),
    ("armada/mcp.rs", include_str!("armada/mcp.rs")),
    ("armada/mod.rs", include_str!("armada/mod.rs")),
    ("armada/roster.rs", include_str!("armada/roster.rs")),
    ("boards.rs", include_str!("boards.rs")),
    ("cache_report.rs", include_str!("cache_report.rs")),
    ("cards.rs", include_str!("cards.rs")),
    ("calib.rs", include_str!("calib.rs")),
    ("canvas.rs", include_str!("canvas.rs")),
    ("capabilities.rs", include_str!("capabilities.rs")),
    ("catalog.rs", include_str!("catalog.rs")),
    ("channels/held.rs", include_str!("channels/held.rs")),
    ("channels/mod.rs", include_str!("channels/mod.rs")),
    ("channels/outbox.rs", include_str!("channels/outbox.rs")),
    ("channels/service.rs", include_str!("channels/service.rs")),
    ("channels/telegram.rs", include_str!("channels/telegram.rs")),
    ("channels/webhook.rs", include_str!("channels/webhook.rs")),
    ("checkpoints.rs", include_str!("checkpoints.rs")),
    ("cleanup.rs", include_str!("cleanup.rs")),
    ("cli.rs", include_str!("cli.rs")),
    ("clipboard.rs", include_str!("clipboard.rs")),
    ("compaction.rs", include_str!("compaction.rs")),
    ("contract.rs", include_str!("contract.rs")),
    ("corpus.rs", include_str!("corpus.rs")),
    ("cron.rs", include_str!("cron.rs")),
    ("deck.rs", include_str!("deck.rs")),
    ("delegate.rs", include_str!("delegate.rs")),
    ("delegate/tests.rs", include_str!("delegate/tests.rs")),
    ("demand.rs", include_str!("demand.rs")),
    ("desk.rs", include_str!("desk.rs")),
    ("dictate.rs", include_str!("dictate.rs")),
    ("doctor.rs", include_str!("doctor.rs")),
    ("documents.rs", include_str!("documents.rs")),
    ("dream.rs", include_str!("dream.rs")),
    ("drift.rs", include_str!("drift.rs")),
    ("engine.rs", include_str!("engine.rs")),
    ("eval.rs", include_str!("eval.rs")),
    ("experiments.rs", include_str!("experiments.rs")),
    ("explain.rs", include_str!("explain.rs")),
    ("explain/tests.rs", include_str!("explain/tests.rs")),
    ("extract.rs", include_str!("extract.rs")),
    ("filelock.rs", include_str!("filelock.rs")),
    ("fileops.rs", include_str!("fileops.rs")),
    ("footprint.rs", include_str!("footprint.rs")),
    ("footprint_history.rs", include_str!("footprint_history.rs")),
    ("fusion.rs", include_str!("fusion.rs")),
    ("generate.rs", include_str!("generate.rs")),
    ("graph.rs", include_str!("graph.rs")),
    ("graph/tests.rs", include_str!("graph/tests.rs")),
    ("gui/bridge.rs", include_str!("gui/bridge.rs")),
    (
        "gui/face_declarations.rs",
        include_str!("gui/face_declarations.rs"),
    ),
    ("gui/mod.rs", include_str!("gui/mod.rs")),
    ("gui/related.rs", include_str!("gui/related.rs")),
    ("gui/server.rs", include_str!("gui/server.rs")),
    ("gui/spine.rs", include_str!("gui/spine.rs")),
    ("gui/themes.rs", include_str!("gui/themes.rs")),
    ("gui/usage.rs", include_str!("gui/usage.rs")),
    ("health.rs", include_str!("health.rs")),
    ("home.rs", include_str!("home.rs")),
    ("hybrid.rs", include_str!("hybrid.rs")),
    ("identity.rs", include_str!("identity.rs")),
    ("improve.rs", include_str!("improve.rs")),
    ("install.rs", include_str!("install.rs")),
    ("journal.rs", include_str!("journal.rs")),
    ("judge.rs", include_str!("judge.rs")),
    ("kanzo.rs", include_str!("kanzo.rs")),
    ("laya.rs", include_str!("laya.rs")),
    ("keepalive.rs", include_str!("keepalive.rs")),
    ("judge/tests.rs", include_str!("judge/tests.rs")),
    ("lane_client.rs", include_str!("lane_client.rs")),
    ("lanes.rs", include_str!("lanes.rs")),
    ("layerfs_tools.rs", include_str!("layerfs_tools.rs")),
    ("layerfs_view.rs", include_str!("layerfs_view.rs")),
    ("lesson.rs", include_str!("lesson.rs")),
    ("lexical.rs", include_str!("lexical.rs")),
    ("lib.rs", include_str!("lib.rs")),
    ("lifecycle.rs", include_str!("lifecycle.rs")),
    ("mcp.rs", include_str!("mcp.rs")),
    ("mcp_meta.rs", include_str!("mcp_meta.rs")),
    ("mcp_registry.rs", include_str!("mcp_registry.rs")),
    ("memory.rs", include_str!("memory.rs")),
    ("memory_pull.rs", include_str!("memory_pull.rs")),
    ("net.rs", include_str!("net.rs")),
    ("nomenclature.rs", include_str!("nomenclature.rs")),
    (
        "nomenclature/tests.rs",
        include_str!("nomenclature/tests.rs"),
    ),
    ("onboard.rs", include_str!("onboard.rs")),
    ("onboard/tests.rs", include_str!("onboard/tests.rs")),
    ("outline.rs", include_str!("outline.rs")),
    ("perception.rs", include_str!("perception.rs")),
    ("plan_graph.rs", include_str!("plan_graph.rs")),
    ("plugins/eval.rs", include_str!("plugins/eval.rs")),
    ("plugins/face.rs", include_str!("plugins/face.rs")),
    ("plugins/mod.rs", include_str!("plugins/mod.rs")),
    (
        "plugins/second_brain.rs",
        include_str!("plugins/second_brain.rs"),
    ),
    ("processes.rs", include_str!("processes.rs")),
    ("recall.rs", include_str!("recall.rs")),
    ("recall_graph.rs", include_str!("recall_graph.rs")),
    ("reload.rs", include_str!("reload.rs")),
    ("remote/broker.rs", include_str!("remote/broker.rs")),
    ("remote/listener.rs", include_str!("remote/listener.rs")),
    ("remote/mod.rs", include_str!("remote/mod.rs")),
    ("remote/pairing.rs", include_str!("remote/pairing.rs")),
    ("remote/qr.rs", include_str!("remote/qr.rs")),
    (
        "remote/kanzo_proxy.rs",
        include_str!("remote/kanzo_proxy.rs"),
    ),
    ("remote/relay.rs", include_str!("remote/relay.rs")),
    ("repo_map.rs", include_str!("repo_map.rs")),
    ("roles.rs", include_str!("roles.rs")),
    ("route.rs", include_str!("route.rs")),
    ("route_cache.rs", include_str!("route_cache.rs")),
    ("search.rs", include_str!("search.rs")),
    ("security.rs", include_str!("security.rs")),
    ("session.rs", include_str!("session.rs")),
    ("session_recall.rs", include_str!("session_recall.rs")),
    ("settings.rs", include_str!("settings.rs")),
    ("shellpath.rs", include_str!("shellpath.rs")),
    ("skills.rs", include_str!("skills.rs")),
    ("subagents.rs", include_str!("subagents.rs")),
    ("tool_advisor.rs", include_str!("tool_advisor.rs")),
    ("tool_audit.rs", include_str!("tool_audit.rs")),
    ("tool_coverage.rs", include_str!("tool_coverage.rs")),
    ("tool_router.rs", include_str!("tool_router.rs")),
    ("tools.rs", include_str!("tools.rs")),
    ("tuios.rs", include_str!("tuios.rs")),
    ("tui/app.rs", include_str!("tui/app.rs")),
    ("tui/events.rs", include_str!("tui/events.rs")),
    ("tui/mod.rs", include_str!("tui/mod.rs")),
    ("testcensus.rs", include_str!("testcensus.rs")),
    ("testsupport.rs", include_str!("testsupport.rs")),
    ("tui/panels.rs", include_str!("tui/panels.rs")),
    ("tui/ui/board_view.rs", include_str!("tui/ui/board_view.rs")),
    ("tui/ui/command.rs", include_str!("tui/ui/command.rs")),
    ("tui/ui/knowledge.rs", include_str!("tui/ui/knowledge.rs")),
    ("tui/ui/lanes.rs", include_str!("tui/ui/lanes.rs")),
    ("tui/ui/mod.rs", include_str!("tui/ui/mod.rs")),
    ("tui/ui/run.rs", include_str!("tui/ui/run.rs")),
    ("tui/ui/trust.rs", include_str!("tui/ui/trust.rs")),
    ("tui/ui/widgets.rs", include_str!("tui/ui/widgets.rs")),
    ("update.rs", include_str!("update.rs")),
    ("vault.rs", include_str!("vault.rs")),
    ("verify.rs", include_str!("verify.rs")),
    ("web.rs", include_str!("web.rs")),
    ("web_cache.rs", include_str!("web_cache.rs")),
    ("work_brief.rs", include_str!("work_brief.rs")),
    ("workspace.rs", include_str!("workspace.rs")),
];

/// The face's served assets, included so a guard reads the artifact that
/// actually ships rather than a copy that can drift. `MODULES` is the
/// Rust module tree — `the_census_matches_the_crates_module_tree` walks
/// `src/` for `mod` declarations, so a stylesheet in that table would be
/// a stale entry by definition. Assets live here instead.
pub(crate) const ASSETS: &[(&str, &str)] =
    &[("gui/assets/app.css", include_str!("gui/assets/app.css"))];

/// The module tree, walked at run time: `lib.rs`'s `mod` declarations,
/// then each file's own, resolving `x` to `x.rs` or `x/mod.rs`.
fn module_tree() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut seen: Vec<String> = Vec::new();
    let mut stack: Vec<String> = vec!["lib.rs".to_string()];
    while let Some(file) = stack.pop() {
        if seen.contains(&file) {
            continue;
        }
        let path = root.join(&file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        seen.push(file.clone());
        let base = if file == "lib.rs" || file == "main.rs" {
            String::new()
        } else if file.ends_with("mod.rs") {
            file.trim_end_matches("mod.rs")
                .trim_end_matches('/')
                .to_string()
        } else {
            file.trim_end_matches(".rs").to_string()
        };
        for name in declared_modules(&source) {
            let joined = |leaf: &str| {
                if base.is_empty() {
                    leaf.to_string()
                } else {
                    format!("{base}/{leaf}")
                }
            };
            let flat = joined(&format!("{name}.rs"));
            let nested = joined(&format!("{name}/mod.rs"));
            if root.join(&flat).exists() {
                stack.push(flat);
            } else if root.join(&nested).exists() {
                stack.push(nested);
            }
        }
    }
    seen.sort();
    seen
}

/// `mod x;` and `pub mod x;` declarations — an inline `mod tests { }`
/// has no semicolon and is not a file, so it is not a module of the tree.
fn declared_modules(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source.lines() {
        let line = line.trim_start();
        let line = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(rest) = line.strip_prefix("mod ") else {
            continue;
        };
        let Some(name) = rest.split(';').next() else {
            continue;
        };
        let name = name.trim();
        if !rest.contains(';')
            || name.is_empty()
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        found.push(name.to_string());
    }
    found
}

/// Does a module name match an accounting pattern?
///
/// A pattern is an exact name (`agent.rs`), a directory (`armada/*`), a
/// suffix (`*/tests.rs`), or a directory subtree (`tui/**`). Patterns exist
/// so a guard can exempt a *class* with one reason instead of enumerating
/// eighty files one at a time — but every class still has to be written
/// down with its reason, so nothing is silently exempt.
pub(crate) fn matches(pattern: &str, module: &str) -> bool {
    if pattern == module {
        return true;
    }
    match pattern.strip_suffix("/**") {
        Some(dir) => module.starts_with(&format!("{dir}/")),
        None => match pattern.strip_prefix("*/") {
            Some(suffix) => module.ends_with(&format!("/{suffix}")),
            None => match pattern.strip_suffix("/*") {
                // `armada/*` covers files directly in that directory, not
                // deeper subtrees — `armada/**` is the recursive form.
                Some(dir) => module
                    .strip_prefix(&format!("{dir}/"))
                    .is_some_and(|rest| !rest.contains('/')),
                None => false,
            },
        },
    }
}

/// The census modules matched by neither the guard's scan set nor its
/// exemptions. Empty is the only healthy answer: anything else is a module
/// the guard cannot say whether it should look at.
pub(crate) fn unaccounted(scanned: &[&str], exempt: &[(&str, &str)]) -> Vec<&'static str> {
    MODULES
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| {
            !scanned.contains(name) && !exempt.iter().any(|(pattern, _)| matches(pattern, name))
        })
        .collect()
}

#[test]
fn the_census_matches_the_crates_module_tree() {
    let tree = module_tree();
    let mut table: Vec<&str> = MODULES.iter().map(|(name, _)| *name).collect();
    table.sort();
    assert!(
        tree.len() > 100,
        "the tree walk found only {} modules — the walk is broken, not the crate",
        tree.len()
    );
    let missing: Vec<&String> = tree
        .iter()
        .filter(|m| !table.contains(&m.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "module file(s) in the crate but not in the census table — add each to MODULES: {missing:?}"
    );
    let stale: Vec<&str> = table
        .iter()
        .copied()
        .filter(|m| !tree.contains(&m.to_string()))
        .collect();
    assert!(
        stale.is_empty(),
        "census entries naming no module file — remove or rename each: {stale:?}"
    );
}

/// The accounting helper's own proof: an incomplete guard is named, a
/// complete one is silent, and every pattern form expands.
#[test]
fn the_accounting_names_an_uncovered_module() {
    assert!(matches("armada/*", "armada/join.rs"));
    assert!(!matches("armada/*", "armada/deep/join.rs"));
    assert!(matches("armada/**", "armada/deep/join.rs"));
    assert!(matches("*/tests.rs", "judge/tests.rs"));
    assert!(matches("agent.rs", "agent.rs"));

    let everything: Vec<&str> = MODULES.iter().map(|(n, _)| *n).collect();
    assert!(unaccounted(&everything, &[]).is_empty());

    let uncovered = unaccounted(&["agent.rs"], &[("armada/**", "runs off a turn")]);
    assert!(
        uncovered.contains(&"calib.rs"),
        "a gap is named: {uncovered:?}"
    );
    assert!(
        !uncovered.contains(&"agent.rs"),
        "the scan set is accounted"
    );
    assert!(
        !uncovered.iter().any(|m| m.starts_with("armada/")),
        "the exemption covers its class"
    );
}

/// Strip `/* … */` comments, preserving newlines so line numbers survive.
fn without_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut comment = false;
    while let Some(c) = chars.next() {
        if comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                comment = false;
            } else if c == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            comment = true;
            out.push_str("  ");
            continue;
        }
        out.push(c);
    }
    out
}

/// The lines that belong to a theme block: `:root {` and every
/// `:root[data-theme="…"] { … }`. The token block is the one place a
/// literal colour may be declared; everything else names a token.
fn token_block_lines(source: &str) -> std::collections::HashSet<usize> {
    let mut inside = false;
    let mut lines = std::collections::HashSet::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let opener = trimmed == ":root {"
            || (trimmed.starts_with(":root[data-theme=\"") && trimmed.ends_with("\"] {"));
        if opener {
            inside = true;
            lines.insert(index);
            continue;
        }
        if inside {
            lines.insert(index);
            if trimmed == "}" {
                inside = false;
            }
        }
    }
    lines
}

/// One `#rgb`/`#rrggbb`/`#rrggbbaa` at the start of `text`, if it is a
/// complete literal and not the prefix of a longer identifier (`#face`
/// as a selector must never read as a colour).
fn hex_literal(text: &str) -> Option<String> {
    let run: String = text[1..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    if !matches!(run.len(), 3 | 4 | 6 | 8) {
        return None;
    }
    let after = text[1 + run.len()..].chars().next();
    if after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(format!("#{run}"))
}

/// Every hex literal in a declaration value, outside the token block.
/// Selectors are never scanned: a value is the text after the last `{`
/// of its `;`-delimited chunk and before any `}`.
fn hexes_outside_the_token_block(source: &str) -> Vec<(usize, String)> {
    let stripped = without_comments(source);
    let allowed = token_block_lines(&stripped);
    let mut found = Vec::new();
    let mut offset = 0usize;
    for chunk in stripped.split(';') {
        let chunk_start = offset;
        offset += chunk.len() + 1;
        let body = match chunk.rfind('{') {
            Some(at) => &chunk[at + 1..],
            None => chunk,
        };
        let body = match body.find('}') {
            Some(at) => &body[..at],
            None => body,
        };
        let Some(colon) = body.find(':') else {
            continue;
        };
        let value = &body[colon + 1..];
        let value_at = chunk_start + (body.as_ptr() as usize - chunk.as_ptr() as usize) + colon + 1;
        let mut cursor = 0;
        while let Some(at) = value[cursor..].find('#') {
            let start = cursor + at;
            let Some(literal) = hex_literal(&value[start..]) else {
                cursor = start + 1;
                continue;
            };
            let line = stripped[..value_at + start].matches('\n').count();
            if !allowed.contains(&line) {
                found.push((line + 1, literal));
            }
            cursor = start + 1;
        }
    }
    found
}

/// P7's AC: a grep guard finds no hex outside the token block. The
/// scanner reads the shipped stylesheet (the `include_str!` artifact,
/// not a copy), so a component that spells `#4B2E2A` fails by name.
#[test]
fn no_hex_lives_outside_the_token_block() {
    let mut offenders = Vec::new();
    for (name, source) in ASSETS {
        for (line, literal) in hexes_outside_the_token_block(source) {
            offenders.push(format!("{name}:{line} {literal}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "hex literals outside the token block — name a token instead:\n{}",
        offenders.join("\n")
    );
}

/// The scanner's own proof: the token block may declare a literal, a
/// component rule may not, and a selector that looks like hex is not one.
#[test]
fn the_hex_scanner_reads_declarations_not_selectors() {
    let fixture = ":root {\n  --x: #fff;\n}\nbody { color: #4B2E2A; }\n#face { color: var(--x); }";
    let found = hexes_outside_the_token_block(fixture);
    assert_eq!(found, vec![(4, "#4B2E2A".to_string())]);
}
