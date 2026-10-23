//! Tool selection as a decision plus geometry. sys1 names the capability
//! class from a closed set; sem1 ranks the candidates by meaning; a
//! declared floor decides whether anything fits at all.
//!
//! No language model is in this path. Every function here is pure, so
//! the whole selection can be tested without a lane, a network, or a
//! model call.

/// The cosine below which no candidate is trusted, measured on this
/// machine's embeddinggemma against a realistic tool catalog: real
/// intents scored 0.691–0.802, no-tool requests 0.602–0.668. 0.68 keeps
/// all sixteen true matches and abstains on all six no-tool ones.
pub const FLOOR: f64 = 0.68;

/// The lexical ranker's floor. Its scores are token-overlap fractions on
/// a different scale (a good match lands near 0.5–0.7), so it cannot
/// share the cosine floor; this is the degraded path, not the calibrated
/// one.
pub const LEXICAL_FLOOR: f64 = 0.30;

/// How close the top two may be before the call is ambiguous. The
/// near-duplicate read tools measured 0.009–0.010 apart, and clear
/// winners 0.024 and up.
pub const MARGIN: f64 = 0.02;

/// How many candidates one search activates.
pub const ACTIVATE_MAX: usize = 5;

/// One deferred candidate: its declared name and its description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub description: String,
}

/// One ranked candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub name: String,
    pub score: f64,
}

/// What the ranking decided.
#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    /// The best candidate clears the floor and the margin.
    Confident(Vec<Ranked>),
    /// The top two are too close to call; both are named.
    Ambiguous(Vec<Ranked>),
    /// Nothing clears the floor. The nearest two ride along as context.
    None(Vec<Ranked>),
}

impl Selection {
    /// The names a caller should activate for the next request.
    pub fn activate(&self) -> Vec<String> {
        match self {
            Selection::Confident(ranked) | Selection::Ambiguous(ranked) => {
                ranked.iter().map(|rank| rank.name.clone()).collect()
            }
            Selection::None(_) => Vec::new(),
        }
    }

    pub fn shown(&self) -> &[Ranked] {
        match self {
            Selection::Confident(ranked)
            | Selection::Ambiguous(ranked)
            | Selection::None(ranked) => ranked,
        }
    }
}

/// Read sys1's capability head out of a `sys1_classify` answer: the label
/// and its confidence, or `None` when the lane gave no usable answer.
pub fn parse_capability(text: &str) -> Option<(String, f64)> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let answers = value.get("answers")?.as_object()?;
    for heads in answers.values() {
        if let Some(answer) = heads.get("capability").and_then(|head| head.as_object()) {
            if let Some(label) = answer
                .get("label")
                .and_then(|label| label.as_str())
                .filter(|label| !label.is_empty())
            {
                let confidence = answer
                    .get("confidence")
                    .and_then(|confidence| confidence.as_f64())
                    .unwrap_or(0.0);
                return Some((label.to_string(), confidence));
            }
        }
    }
    None
}

/// The vectors in a `sem1_embed` answer. `embed` reports `vectors`; the
/// alternate spelling is accepted so a provider change does not silently
/// turn every search into a lexical fallback.
pub fn parse_vectors(text: &str) -> Option<Vec<Vec<f32>>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let array = value
        .get("vectors")
        .or_else(|| value.get("embeddings"))?
        .as_array()?;
    let mut out = Vec::with_capacity(array.len());
    for vector in array {
        let values = vector.as_array()?;
        out.push(
            values
                .iter()
                .map(|value| value.as_f64().unwrap_or(0.0) as f32)
                .collect(),
        );
    }
    Some(out)
}

/// Cosine similarity, 0.0 when the shapes differ.
pub fn cosine(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (a, b) in left.iter().zip(right) {
        let (a, b) = (*a as f64, *b as f64);
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

/// Rank candidates by the vectors `[query, candidate…]`.
pub fn rank_by_vectors(
    vectors: &[Vec<f32>],
    candidates: &[Candidate],
    class: Option<&str>,
) -> Vec<Ranked> {
    let Some(query) = vectors.first() else {
        return Vec::new();
    };
    let mut ranked: Vec<Ranked> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let score = vectors
                .get(index + 1)
                .map(|vector| cosine(query, vector))
                .unwrap_or(0.0)
                + class_boost(class, candidate);
            Ranked {
                name: candidate.name.clone(),
                score,
            }
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });
    ranked
}

/// The lexical fallback: token overlap over the name and description.
pub fn lexical_rank(query: &str, candidates: &[Candidate]) -> Vec<Ranked> {
    let query = tokens(query);
    let mut ranked: Vec<Ranked> = candidates
        .iter()
        .map(|candidate| {
            let text = tokens(&format!("{} {}", candidate.name, candidate.description));
            let shared = query.intersection(&text).count() as f64;
            let score = if query.is_empty() || text.is_empty() {
                0.0
            } else {
                shared / (query.len() as f64).sqrt() / (text.len() as f64).sqrt()
            };
            Ranked {
                name: candidate.name.clone(),
                score,
            }
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });
    ranked
}

/// Apply the floor and the margin to a ranking.
pub fn select(ranked: Vec<Ranked>, floor: f64) -> Selection {
    // Only candidates above the floor are eligible; the nearest two ride
    // along only when nothing clears it.
    let nearest: Vec<Ranked> = ranked.iter().take(2).cloned().collect();
    let above: Vec<Ranked> = ranked
        .into_iter()
        .filter(|rank| rank.score >= floor)
        .take(ACTIVATE_MAX)
        .collect();
    if above.is_empty() {
        return Selection::None(nearest);
    }
    let close = above
        .get(1)
        .is_some_and(|second| above[0].score - second.score < MARGIN);
    if close {
        return Selection::Ambiguous(above.into_iter().take(2).collect());
    }
    Selection::Confident(above)
}

/// The text that rides into an embedding: the request plus the class
/// sys1 chose, so the class is part of the geometry rather than a
/// threshold applied afterwards.
pub fn query_text(query: &str, class: Option<&str>) -> String {
    match class {
        Some(class) if class != "none" => {
            format!("{query}\ncapability: {class} — {}", class_gloss(class))
        }
        _ => query.to_string(),
    }
}

/// A candidate description, short enough to embed and to read.
pub fn card(description: &str) -> String {
    let line = description.lines().next().unwrap_or("").trim();
    let mut text: String = line.chars().take(400).collect();
    if line.chars().count() > 400 {
        text.push('\u{2026}');
    }
    text
}

/// A stable hash of the catalog and class, so cached vectors are reused
/// only for the catalog that produced them.
pub fn catalog_hash(candidates: &[Candidate], class: Option<&str>) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    for candidate in candidates {
        feed(candidate.name.as_bytes());
        feed(b"\0");
        feed(candidate.description.as_bytes());
        feed(b"\0");
    }
    if let Some(class) = class {
        feed(class.as_bytes());
    }
    hash
}

/// The words that make a candidate belong to a class. A small
/// deterministic boost, never a filter: a candidate whose text carries
/// the class's vocabulary ranks a little higher than geometry found it.
fn class_boost(class: Option<&str>, candidate: &Candidate) -> f64 {
    let Some(class) = class else {
        return 0.0;
    };
    let text = format!("{} {}", candidate.name, candidate.description).to_lowercase();
    let hit = class_terms(class).iter().any(|term| text.contains(term));
    if hit {
        0.03
    } else {
        0.0
    }
}

fn class_terms(class: &str) -> &'static [&'static str] {
    match class {
        "read" => &["read", "get", "view", "list", "stat", "open", "file"],
        "search" => &["search", "find", "grep", "query", "glob", "match"],
        "edit" => &[
            "edit", "write", "create", "replace", "patch", "update", "modify",
        ],
        "execute" => &[
            "run", "exec", "command", "shell", "bash", "build", "test", "terminal",
        ],
        "web" => &["web", "fetch", "http", "url", "browse", "search", "page"],
        "memory" => &["memory", "remember", "recall", "fact", "knowledge"],
        "vision" => &["image", "vision", "screenshot", "look", "photo", "ocr"],
        "media" => &[
            "image",
            "audio",
            "speech",
            "render",
            "deck",
            "presentation",
            "video",
        ],
        "delegate" => &["agent", "delegate", "subagent", "task"],
        _ => &[],
    }
}

fn class_gloss(class: &str) -> &'static str {
    match class {
        "read" => "inspect a file or directory that is already known",
        "search" => "find something not yet located",
        "edit" => "change an existing file or write a new one",
        "execute" => "run a command, build, or test",
        "web" => "look something up on the live web",
        "memory" => "recall or record durable knowledge",
        "vision" => "look at an image or a screenshot",
        "media" => "generate or judge media",
        "delegate" => "hand a task to another agent",
        _ => "no particular capability",
    }
}

fn tokens(text: &str) -> std::collections::BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| word.len() > 2)
        .map(|word| word.to_lowercase())
        .collect()
}

/// One provider of a capability, as the dispatch policy sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub name: String,
    /// Already joined or loaded, so calling it costs no spawn.
    pub warm: bool,
    /// The last health probe said it answers.
    pub healthy: bool,
}

/// Choose a provider: warm first, then healthy, then declared order.
/// `write` capabilities never fall back — a half-applied write in two
/// places is worse than a refusal — so their choice is the first
/// provider or none.
pub fn choose(providers: &[Provider], write: bool) -> Option<(&Provider, &'static str)> {
    if write {
        return providers
            .first()
            .map(|provider| (provider, "the only provider for a write"));
    }
    if let Some(provider) = providers
        .iter()
        .find(|provider| provider.warm && provider.healthy)
    {
        return Some((provider, "warm and healthy"));
    }
    if let Some(provider) = providers.iter().find(|provider| provider.healthy) {
        return Some((provider, "healthy; a cold spawn is accepted"));
    }
    providers
        .first()
        .map(|provider| (provider, "no healthy provider; the first is tried"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, description: &str) -> Candidate {
        Candidate {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    #[test]
    fn a_capability_answer_is_read_from_the_answers_map() {
        let text = r#"{"verdict":"ok","answers":{"jev":{"capability":{"label":"search","confidence":0.82}}}}"#;
        assert_eq!(parse_capability(text), Some(("search".to_string(), 0.82)));
        assert_eq!(parse_capability("{}"), None);
    }

    #[test]
    fn vectors_and_distance_rank_the_right_candidate_first() {
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.9, 0.1, 0.0],
            vec![0.0, 1.0, 0.0],
        ];
        let candidates = vec![
            candidate("a", "search files"),
            candidate("b", "generate audio"),
        ];
        let ranked = rank_by_vectors(&vectors, &candidates, Some("search"));
        assert_eq!(ranked[0].name, "a");
        assert!(ranked[0].score > ranked[1].score);
    }

    #[test]
    fn the_floor_and_margin_decide() {
        let clear = select(
            vec![
                Ranked {
                    name: "a".into(),
                    score: 0.9,
                },
                Ranked {
                    name: "b".into(),
                    score: 0.4,
                },
            ],
            FLOOR,
        );
        assert!(matches!(clear, Selection::Confident(_)));
        let close = select(
            vec![
                Ranked {
                    name: "a".into(),
                    score: 0.9,
                },
                Ranked {
                    name: "b".into(),
                    score: 0.89,
                },
            ],
            FLOOR,
        );
        assert!(matches!(close, Selection::Ambiguous(_)));
        let nothing = select(
            vec![
                Ranked {
                    name: "a".into(),
                    score: 0.1,
                },
                Ranked {
                    name: "b".into(),
                    score: 0.05,
                },
            ],
            FLOOR,
        );
        assert!(matches!(nothing, Selection::None(_)));
        assert!(nothing.activate().is_empty());
        // The lexical floor is its own scale: a lexical winner that
        // would fail the cosine floor still activates.
        let lexical = select(
            vec![
                Ranked {
                    name: "a".into(),
                    score: 0.55,
                },
                Ranked {
                    name: "b".into(),
                    score: 0.20,
                },
            ],
            LEXICAL_FLOOR,
        );
        assert!(matches!(lexical, Selection::Confident(_)));
        assert_eq!(lexical.activate(), vec!["a".to_string()]);
    }

    #[test]
    fn lexical_ranking_finds_the_tool_that_names_the_work() {
        let candidates = vec![
            candidate("mcp__fs__write_file", "write a file to disk"),
            candidate("mcp__web__fetch", "fetch a URL and return its text"),
            candidate("mcp__db__query", "query the database"),
        ];
        let ranked = lexical_rank("fetch a url", &candidates);
        assert_eq!(ranked[0].name, "mcp__web__fetch", "{ranked:?}");
    }

    #[test]
    fn the_query_carries_the_class_and_the_hash_is_stable() {
        let a = candidate("a", "read a file");
        let b = candidate("b", "delete a file");
        assert!(query_text("look", Some("read")).contains("capability: read"));
        assert_eq!(
            catalog_hash(&[a.clone(), b.clone()], Some("read")),
            catalog_hash(&[a, b], Some("read"))
        );
    }

    fn fake_vector(text: &str) -> Vec<f32> {
        let dims = 128;
        let mut vector = vec![0.0f32; dims];
        for token in text.split(|c: char| !c.is_alphanumeric()) {
            if token.len() < 3 {
                continue;
            }
            let mut hash = 0xcbf29ce484222325u64;
            for byte in token.to_lowercase().bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            vector[(hash as usize) % dims] += 1.0;
        }
        let norm: f32 = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }
        vector
    }

    #[test]
    fn selection_ranks_the_expected_tool_across_the_corpus() {
        // Forty candidates, twenty-four queries, each naming one
        // candidate's distinctive token. The point is the pipeline —
        // query, cards, cosine, floor — not the quality of a fake
        // embedder.
        let candidates: Vec<Candidate> = (0..40)
            .map(|index| {
                candidate(
                    &format!("c{index}"),
                    &format!("capability token{index} work"),
                )
            })
            .collect();
        let card_vectors: Vec<Vec<f32>> = candidates
            .iter()
            .map(|candidate| fake_vector(&format!("{} {}", candidate.name, candidate.description)))
            .collect();
        let mut hits = 0;
        for index in 0..24 {
            let mut vectors = vec![fake_vector(&format!("do token{index} now"))];
            vectors.extend(card_vectors.iter().cloned());
            let ranked = rank_by_vectors(&vectors, &candidates, None);
            let top: Vec<&str> = ranked
                .iter()
                .take(3)
                .map(|rank| rank.name.as_str())
                .collect();
            if top.contains(&format!("c{index}").as_str()) {
                hits += 1;
            }
        }
        assert!(hits >= 20, "only {hits}/24 in the top three");
    }

    #[test]
    fn dispatch_prefers_warm_then_healthy_then_order() {
        let providers = vec![
            Provider {
                name: "cold".into(),
                warm: false,
                healthy: true,
            },
            Provider {
                name: "warm".into(),
                warm: true,
                healthy: true,
            },
            Provider {
                name: "dead".into(),
                warm: true,
                healthy: false,
            },
        ];
        let (chosen, reason) = choose(&providers, false).unwrap();
        assert_eq!(chosen.name, "warm");
        assert_eq!(reason, "warm and healthy");
        // A write never falls back: the first provider is the choice,
        // whatever its health.
        let (chosen, reason) = choose(&providers, true).unwrap();
        assert_eq!(chosen.name, "cold");
        assert!(reason.contains("only provider"));
        // Nothing healthy: the first is tried, and the reason says so.
        let dead = vec![Provider {
            name: "dead".into(),
            warm: false,
            healthy: false,
        }];
        let (chosen, reason) = choose(&dead, false).unwrap();
        assert_eq!(chosen.name, "dead");
        assert!(reason.contains("no healthy"));
    }
}
