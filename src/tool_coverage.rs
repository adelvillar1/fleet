//! The owner-intent coverage pass: which tools the archive's owner
//! turns *should* have reached and did not.
//!
//! The complement of the call counts the advisor reads. A tool can be
//! uncalled because nothing needed it, or because the model worked
//! around it — the log alone cannot tell those apart. Owner turns and
//! tool cards can, by similarity: a turn whose nearest card clears the
//! calibrated floor describes work a tool exists for.
//!
//! The pass is a review signal, never a gate. Similarity is noisy
//! (near-duplicate tools sit hundredths apart on this embedder), so
//! every row names its score and its method, and the disable gate reads
//! exact usage only. Disabled tools are included on purpose: a
//! recurring need is a reactivation suggestion, which is the mirror
//! that makes disabling reversible by evidence rather than by memory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::session::Record;

/// The embedder the pass runs on: a batch of texts in, one vector each.
/// The production implementation drives the sem1 lane; tests drive a
/// deterministic double, which is why the seam exists.
pub trait TurnEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
}

/// One coverage miss: a turn, the tool it was nearest, and the score.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageMiss {
    pub session: String,
    /// The turn, capped for the card.
    pub turn: String,
    pub tool: String,
    pub score: f64,
}

/// What the pass found, with its own method stated beside the numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CoverageReport {
    pub sessions: usize,
    pub turns: usize,
    /// Active tools the archive's owner turns reached for by intent but
    /// no call named. Review material for the prompt, not the gate.
    pub misses: Vec<CoverageMiss>,
    /// Disabled tools whose intent recurred: reactivation suggestions.
    pub reactivations: Vec<CoverageMiss>,
    /// Vectors the cache answered, and vectors the embedder computed.
    pub cached: usize,
    pub embedded: usize,
}

/// Where the message-hash vector cache lives: beside the recall index,
/// on its own `__pycache__` discipline (absent or malformed reads as
/// empty; a write failure costs the cache, never the pass).
fn cache_path(home: &Path) -> PathBuf {
    home.join(".coverage-cache.json")
}

/// A 128-bit text key: two independent FNV words, so a collision needs
/// two hashes to agree — the cache serves vectors by content, and a
/// wrong vector is a silent wrong answer.
fn text_key(text: &str) -> String {
    let mut first: u64 = 0xcbf29ce484222325;
    let mut second: u64 = 0x9e3779b97f4a7c15;
    for byte in text.bytes() {
        first ^= byte as u64;
        first = first.wrapping_mul(0x100000001b3);
        second ^= (byte as u64)
            .wrapping_add(second << 6)
            .wrapping_add(second >> 2);
    }
    format!("{first:016x}{second:016x}")
}

/// One owner turn with the tools its session actually called.
struct OwnerTurn {
    session: String,
    text: String,
    called: Vec<String>,
}

/// Read the archive's owner turns: every `User` record in the window,
/// with the session's called tool names beside it. The first user
/// message is the task; later ones are steers — both are intent.
fn owner_turns(home: &Path, window: usize) -> Vec<OwnerTurn> {
    let mut turns = Vec::new();
    let Ok(summaries) = crate::session::list(home) else {
        return turns;
    };
    for summary in summaries.iter().rev().take(window.max(1)) {
        let Ok(records) = crate::session::replay(&summary.path) else {
            continue;
        };
        let mut called: Vec<String> = Vec::new();
        for record in &records {
            if let Record::Assistant { tool_calls, .. } = record {
                for call in tool_calls {
                    called.push(call.function.name.clone());
                }
            }
        }
        for record in &records {
            if let Record::User { content } = record {
                if content.trim().is_empty() {
                    continue;
                }
                turns.push(OwnerTurn {
                    session: summary.id.clone(),
                    text: content.clone(),
                    called: called.clone(),
                });
            }
        }
    }
    turns
}

/// The vectors the pass needs, cache first: every distinct text once,
/// whatever many turns share it. Returns the vectors in `texts` order
/// and counts who answered.
fn vectors_through_cache(
    home: &Path,
    texts: &[String],
    embedder: &dyn TurnEmbedder,
) -> Result<(Vec<Vec<f32>>, usize, usize), String> {
    let cache_file = cache_path(home);
    let mut cache: BTreeMap<String, Vec<f32>> = std::fs::read_to_string(&cache_file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let mut missing: Vec<usize> = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        if !cache.contains_key(&text_key(text)) {
            missing.push(index);
        }
    }
    let mut embedded = 0usize;
    if !missing.is_empty() {
        let batch: Vec<String> = missing.iter().map(|index| texts[*index].clone()).collect();
        let computed = embedder.embed(&batch)?;
        for (offset, index) in missing.iter().enumerate() {
            if let Some(vector) = computed.get(offset) {
                cache.insert(text_key(&texts[*index]), vector.clone());
                embedded += 1;
            }
        }
        // Best effort: the pass answers even when the cache cannot be
        // written, and the next run simply embeds again.
        if let Ok(text) = serde_json::to_string(&cache) {
            let _ = std::fs::write(&cache_file, text);
        }
    }
    let cached = texts.len() - embedded;
    let vectors = texts
        .iter()
        .map(|text| {
            cache
                .get(&text_key(text))
                .cloned()
                .ok_or_else(|| "the cache lost a vector it just wrote".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((vectors, cached, embedded))
}

/// Run the pass. `cards` are the (name, description) pairs to rank
/// against; `disabled` names are reported as reactivations rather than
/// misses. The floor is the calibrated cosine from the tool router.
pub fn coverage(
    home: &Path,
    window: usize,
    embedder: &dyn TurnEmbedder,
    cards: &[(String, String)],
    disabled: &[String],
) -> Result<CoverageReport, String> {
    let turns = owner_turns(home, window);
    let mut report = CoverageReport {
        sessions: turns
            .iter()
            .map(|turn| turn.session.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        turns: turns.len(),
        ..CoverageReport::default()
    };
    if turns.is_empty() || cards.is_empty() {
        return Ok(report);
    }
    // Cards first, then the distinct turn texts in first-seen order: one
    // embed per text, and one order both the embed and the lookup read.
    let card_texts: Vec<String> = cards
        .iter()
        .map(|(name, description)| {
            format!("{name} \u{2014} {}", crate::tool_router::card(description))
        })
        .collect();
    let mut distinct: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for turn in &turns {
        if seen.insert(turn.text.clone()) {
            distinct.push(turn.text.clone());
        }
    }
    let mut texts = card_texts.clone();
    texts.extend(distinct.iter().cloned());
    let (vectors, cached, embedded) = vectors_through_cache(home, &texts, embedder)?;
    report.cached = cached;
    report.embedded = embedded;
    let card_vectors = &vectors[..cards.len()];
    let by_text: std::collections::BTreeMap<&str, &Vec<f32>> = distinct
        .iter()
        .enumerate()
        .map(|(index, text)| (text.as_str(), &vectors[cards.len() + index]))
        .collect();
    for turn in &turns {
        let Some(vector) = by_text.get(turn.text.as_str()) else {
            continue;
        };
        let mut best: Option<(f64, &str)> = None;
        for (offset, (name, _)) in cards.iter().enumerate() {
            let score = crate::tool_router::cosine(vector, &card_vectors[offset]);
            if best.is_none_or(|(current, _)| score > current) {
                best = Some((score, name));
            }
        }
        let Some((score, tool)) = best else { continue };
        if score < crate::tool_router::FLOOR {
            continue;
        }
        // The turn named its intent; did the session call the tool, or
        // any alias of it?
        let resolved_called = turn.called.iter().any(|called| {
            called == tool
                || crate::tools::resolve_name(called)
                    .map(|(candidate, _)| candidate.name() == tool)
                    .unwrap_or(false)
        });
        if resolved_called {
            continue;
        }
        let miss = CoverageMiss {
            session: turn.session.clone(),
            turn: turn.text.chars().take(120).collect(),
            tool: tool.to_string(),
            score,
        };
        if disabled.iter().any(|name| name == tool) {
            report.reactivations.push(miss);
        } else {
            report.misses.push(miss);
        }
    }
    Ok(report)
}

/// The report as a card, with the method and its limits stated beside
/// every number: this is a review signal, and it says so.
pub fn render(report: &CoverageReport, window: usize) -> String {
    let mut out = format!(
        "fleet tools coverage \u{2014} {} owner turn(s) over {} session(s), floor {}\n",
        report.turns,
        report.sessions,
        crate::tool_router::FLOOR
    );
    out.push_str(
        "  method: nearest tool card by sem1 cosine over the owner's turns; review-only \u{2014} the \
         disable gate reads exact usage, never this\n",
    );
    out.push_str(&format!(
        "  vectors: {} embedded, {} from the cache\n",
        report.embedded, report.cached
    ));
    if !report.misses.is_empty() {
        out.push_str("\n  active tools the turns reached for but no call named\n");
        for miss in report.misses.iter().take(20) {
            out.push_str(&format!(
                "  {:.2}  {}  \u{2014} {} [{}]\n",
                miss.score, miss.tool, miss.turn, miss.session
            ));
        }
        if report.misses.len() > 20 {
            out.push_str(&format!(
                "  \u{2026} and {} more\n",
                report.misses.len() - 20
            ));
        }
    } else {
        out.push_str("\n  no coverage misses over the window\n");
    }
    if !report.reactivations.is_empty() {
        out.push_str(
            "\n  disabled tools whose need recurred \u{2014} `fleet tools enable <name>`\n",
        );
        for miss in report.reactivations.iter().take(10) {
            out.push_str(&format!(
                "  {:.2}  {}  \u{2014} {} [{}]\n",
                miss.score, miss.tool, miss.turn, miss.session
            ));
        }
    }
    let _ = window;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic embedder: one hot dimension per keyword, so a
    /// turn shares a dimension with the card that names its work.
    struct KeywordEmbedder;

    impl TurnEmbedder for KeywordEmbedder {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            let vocabulary = [
                "search", "file", "edit", "shell", "image", "web", "joke", "flight",
            ];
            Ok(texts
                .iter()
                .map(|text| {
                    let lowered = text.to_lowercase();
                    vocabulary
                        .iter()
                        .map(|word| if lowered.contains(word) { 1.0 } else { 0.0 })
                        .collect()
                })
                .collect())
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fleet-coverage-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(crate::home::ensure_sessions_dir(&dir).unwrap()).unwrap();
        dir
    }

    fn session_with(home: &Path, _id: &str, turns: &[&str], calls: &[&str]) {
        let mut session = crate::session::Session::create(home, "test-model").unwrap();
        for turn in turns {
            session
                .record(&Record::User {
                    content: (*turn).to_string(),
                })
                .unwrap();
        }
        let tool_calls: Vec<crate::api::ToolCall> = calls
            .iter()
            .enumerate()
            .map(|(index, name)| crate::api::ToolCall {
                id: format!("c{index}"),
                kind: "function".to_string(),
                function: crate::api::FunctionCall {
                    name: (*name).to_string(),
                    arguments: "{}".to_string(),
                },
            })
            .collect();
        if !tool_calls.is_empty() {
            session
                .record(&Record::Assistant {
                    content: None,
                    tool_calls,
                    persona: None,
                })
                .unwrap();
        }
        session
            .record(&Record::End {
                stop_reason: "completed".to_string(),
                turns: 1,
                served_model: None,
            })
            .unwrap();
    }

    #[test]
    fn a_turn_whose_tool_was_not_called_is_a_miss_and_a_called_one_is_not() {
        let home = scratch("miss");
        session_with(
            &home,
            "s1",
            &["search the whole file for the pins"],
            &["bash"],
        );
        session_with(&home, "s2", &["edit the line"], &["edit"]);
        let cards = vec![
            ("grep".to_string(), "search file contents".to_string()),
            ("edit".to_string(), "edit a passage".to_string()),
        ];
        let report = coverage(&home, 10, &KeywordEmbedder, &cards, &[]).unwrap();
        assert_eq!(report.turns, 2);
        assert_eq!(report.misses.len(), 1, "{report:?}");
        assert_eq!(report.misses[0].tool, "grep");
        assert!(report.misses[0].score >= crate::tool_router::FLOOR);
        // The second turn called its nearest tool: no miss.
        assert!(!report
            .misses
            .iter()
            .any(|miss| miss.turn.contains("edit the line")));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_disabled_tool_whose_need_recurs_is_a_reactivation() {
        let home = scratch("reactivation");
        session_with(&home, "s1", &["search the file"], &[]);
        let cards = vec![
            ("grep".to_string(), "search file contents".to_string()),
            ("bash".to_string(), "run a shell command".to_string()),
        ];
        let report = coverage(&home, 10, &KeywordEmbedder, &cards, &["grep".to_string()]).unwrap();
        assert!(report.misses.is_empty(), "{report:?}");
        assert_eq!(report.reactivations.len(), 1);
        assert_eq!(report.reactivations[0].tool, "grep");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_unrelated_turn_abstains_below_the_floor() {
        let home = scratch("abstain");
        session_with(&home, "s1", &["tell me a joke about flights"], &[]);
        let cards = vec![("grep".to_string(), "search file contents".to_string())];
        let report = coverage(&home, 10, &KeywordEmbedder, &cards, &[]).unwrap();
        assert!(report.misses.is_empty(), "{report:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_cache_answers_the_second_run_without_the_embedder() {
        let home = scratch("cache");
        session_with(&home, "s1", &["search the file"], &[]);
        let cards = vec![("grep".to_string(), "search file contents".to_string())];
        let first = coverage(&home, 10, &KeywordEmbedder, &cards, &[]).unwrap();
        assert_eq!(first.embedded, 2);
        struct Refuses;
        impl TurnEmbedder for Refuses {
            fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
                Err("never called".to_string())
            }
        }
        let second = coverage(&home, 10, &Refuses, &cards, &[]).unwrap();
        assert_eq!(second.embedded, 0, "the cache answered everything");
        assert_eq!(second.misses.len(), first.misses.len());
        let _ = std::fs::remove_dir_all(&home);
    }
}
