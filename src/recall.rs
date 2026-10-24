//! `recall`: the harness's own holdings, read on demand.
//!
//! A run already carries memory and a skills library, but it carries
//! them pre-emptively: the assembly spends budget on facts it guessed
//! would matter, and a skill's body arrives only when `use_skill` names
//! it. `recall` is the other direction — the model says what it is
//! about to do, and this returns what the harness already holds.
//!
//! Three rules make it cheap enough to call often:
//!
//! * **No model call.** Ranking is the memory corpus's own scoring and a
//!   lexical match over the loaded skill library. A tool whose answer
//!   costs a generation is a tool the model learns not to call.
//! * **Read-only.** Nothing is bumped, nothing is recorded, no file is
//!   rewritten. A query is not a hit: only what actually entered a turn
//!   is, and that is the assembly's own accounting (KI-068).
//! * **Bounded.** Facts, skills and the bytes of each are capped, and the
//!   result says what it left out.

use crate::memory::ScoredFact;
use crate::skills::SkillRef;

/// How many facts one recall returns.
pub const MAX_FACTS: usize = 8;
/// How many skills one recall returns.
pub const MAX_SKILLS: usize = 6;
/// How many characters of a fact's description are quoted.
const FACT_CHARS: usize = 400;
/// How many characters of a skill's description are quoted.
const SKILL_CHARS: usize = 240;

/// One recalled fact, flattened for rendering: the corpus's own score,
/// where it came from, and the sentence itself.
#[derive(Debug, Clone, PartialEq)]
pub struct RecalledFact {
    pub name: String,
    pub description: String,
    pub fact_type: String,
    pub date: String,
    /// The fact's provenance: `confirmed-by` when the owner said it,
    /// `inferred` when a session produced it. It is the difference
    /// between a rule and a guess, so it sits beside the score.
    pub basis: String,
    pub score: f64,
    /// True when the fact entered through the spillover rather than the
    /// shape's own pool.
    pub off_shape: bool,
}

/// One skill the instruction's words match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecalledSkill {
    pub name: String,
    pub description: String,
}

/// What one recall found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recall {
    pub facts: Vec<RecalledFact>,
    pub skills: Vec<RecalledSkill>,
    /// Facts the ranking produced before the cap.
    pub facts_found: usize,
    /// Skills whose text matched before the cap.
    pub skills_found: usize,
}

impl Recall {
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.skills.is_empty()
    }
}

/// Rank the facts the memory already holds against `instruction`.
///
/// The memory's own `recall` is the ranking — the same score, the same
/// shape pool, the same freshness decay — so the tool cannot drift from
/// what the assembly would have chosen.
pub fn facts(
    memory: Option<&crate::memory::Memory>,
    instruction: &str,
) -> (Vec<RecalledFact>, usize) {
    let Some(memory) = memory else {
        return (Vec::new(), 0);
    };
    let scored: Vec<ScoredFact> = memory.recall(instruction);
    let found = scored.len();
    let facts = scored
        .into_iter()
        .take(MAX_FACTS)
        .map(|hit| RecalledFact {
            name: hit.fact.name.clone(),
            description: clip(&hit.fact.description, FACT_CHARS),
            fact_type: hit.fact.fact_type.clone(),
            date: hit.fact.date.clone(),
            basis: hit.fact.basis.label().to_string(),
            score: hit.score.total,
            off_shape: hit.off_shape,
        })
        .collect();
    (facts, found)
}

/// The skills whose name, description or tags the instruction's words
/// touch.
///
/// Lexical on purpose: the library is loaded in memory and small, an
/// instruction is a sentence, and an embedding per call would make this
/// the expensive tool it exists to replace. The order is by how many of
/// the instruction's words a skill matches, then by name, so two runs of
/// one instruction read the same list.
pub fn skills(library: &[SkillRef], instruction: &str) -> (Vec<RecalledSkill>, usize) {
    let words: Vec<String> = instruction
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.len() >= 4)
        .map(str::to_string)
        .collect();
    if words.is_empty() {
        return (Vec::new(), 0);
    }
    let mut scored: Vec<(usize, RecalledSkill)> = Vec::new();
    for skill in library {
        // The name's separators become spaces so `debug-issue` matches
        // an instruction that says "debug".
        let haystack = format!(
            "{} {}",
            skill.name.replace(['-', '_'], " "),
            skill.description
        )
        .to_lowercase();
        let hits = words.iter().filter(|word| haystack.contains(*word)).count();
        if hits == 0 {
            continue;
        }
        scored.push((
            hits,
            RecalledSkill {
                name: skill.name.clone(),
                description: clip(&skill.description, SKILL_CHARS),
            },
        ));
    }
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.name.cmp(&right.1.name))
    });
    let found = scored.len();
    let skills = scored
        .into_iter()
        .take(MAX_SKILLS)
        .map(|(_, skill)| skill)
        .collect();
    (skills, found)
}

/// The whole answer, rendered: what was found, with the numbers and the
/// sources a reader can check.
pub fn render(instruction: &str, recall: &Recall) -> String {
    if recall.is_empty() {
        return format!(
            "nothing in this harness's memory or skill library matches {instruction:?}. That is an answer: the fact is not written down here, so check the tree or the live web rather than assuming."
        );
    }
    let mut text = String::new();
    if !recall.facts.is_empty() {
        text.push_str(&format!(
            "facts ({} of {} ranked; score, kind, date):\n",
            recall.facts.len(),
            recall.facts_found
        ));
        for fact in &recall.facts {
            text.push_str(&format!(
                "- {} \u{2014} {}{} [{:.2} {} {} {}]\n",
                fact.name,
                fact.description,
                if fact.off_shape { " (off-shape)" } else { "" },
                fact.score,
                fact.fact_type,
                fact.basis,
                fact.date
            ));
        }
    }
    if !recall.skills.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!(
            "skills ({} of {} matched; call one with use_skill):\n",
            recall.skills.len(),
            recall.skills_found
        ));
        for skill in &recall.skills {
            text.push_str(&format!("- {}: {}\n", skill.name, skill.description));
        }
    }
    text.trim_end().to_string()
}

/// Cut a sentence to `chars` characters, saying it was cut.
fn clip(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(chars).collect();
    clipped.push('\u{2026}');
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(name: &str, description: &str) -> SkillRef {
        SkillRef {
            name: name.to_string(),
            description: description.to_string(),
        }
    }

    #[test]
    fn skills_rank_by_words_and_break_ties_by_name() {
        let library = vec![
            skill("alpha-search", "search a codebase quickly"),
            skill("beta-search", "search a codebase quickly"),
            skill("unrelated", "bake bread"),
        ];
        let (found, total) = skills(&library, "how do I search this codebase");
        assert_eq!(total, 2);
        assert_eq!(found[0].name, "alpha-search");
        assert_eq!(found[1].name, "beta-search");
        // Two runs of one instruction read the same list.
        let again = skills(&library, "how do I search this codebase").0;
        assert_eq!(found, again);
    }

    #[test]
    fn short_words_do_not_match_everything() {
        let library = vec![skill("x", "the a of to")];
        let (found, total) = skills(&library, "do it");
        assert_eq!(total, 0);
        assert!(found.is_empty());
    }

    #[test]
    fn nothing_found_is_an_answer_not_an_empty_string() {
        let recall = Recall::default();
        let text = render("who owns the billing service", &recall);
        assert!(text.contains("nothing in this harness's memory"), "{text}");
        assert!(text.contains("check the tree"), "{text}");
    }

    #[test]
    fn the_rendering_names_its_numbers_and_sources() {
        let recall = Recall {
            facts: vec![RecalledFact {
                name: "the-owner-ruling".to_string(),
                description: "the owner ruled X".to_string(),
                fact_type: "feedback".to_string(),
                date: "2026-10-20".to_string(),
                basis: "confirmed-by".to_string(),
                score: 0.81,
                off_shape: true,
            }],
            skills: vec![RecalledSkill {
                name: "debug-issue".to_string(),
                description: "find a bug".to_string(),
            }],
            facts_found: 12,
            skills_found: 2,
        };
        let text = render("what did the owner say", &recall);
        assert!(text.contains("1 of 12 ranked"), "{text}");
        assert!(
            text.contains("0.81 feedback confirmed-by 2026-10-20"),
            "{text}"
        );
        assert!(text.contains("off-shape"), "{text}");
        assert!(text.contains("1 of 2 matched"), "{text}");
        assert!(text.contains("use_skill"), "{text}");
    }

    #[test]
    fn a_long_description_is_clipped_and_says_so() {
        let long = "x".repeat(FACT_CHARS + 10);
        let clipped = clip(&long, FACT_CHARS);
        assert_eq!(clipped.chars().count(), FACT_CHARS + 1);
        assert!(clipped.ends_with('\u{2026}'));
        assert_eq!(clip("short", FACT_CHARS), "short");
    }
}
