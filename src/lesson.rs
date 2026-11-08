//! The learning loop (the LearnHarness design): the lesson session kind,
//! the prober and the probe loop (the measure half), and the records on
//! the substrate.
//!
//! The theory's demands are the acceptance criteria: the prober produces
//! precise states (the bracketed current understanding and the pinned
//! goal), the teaching moves in single checked steps, the plan's
//! resolution does not leak into the lesson, the logistics are the
//! system's, and the goal is the learner's own "obviously true".
//!
//! This module holds the measure half's host: the prober's role and the
//! probe loop, and the records — the probe answers and the bracket as
//! facts (the `probe` type) and one calibration row (the `learn_probe`
//! op). The plan (the minimal tree), the teaching, the knowledge graph,
//! the practice, and the face surface follow in the same module as the
//! loop grows.

use std::path::Path;

use crate::calib::{self, Row, Stream};
use crate::memory::{self, Basis, Memory, Tier};

/// The prober role's name — the file under `<home>/roles/` the lesson's
/// run rides as its overlay.
pub const PROBER_ROLE: &str = "prober";

/// One probe answer: the learner's words, their thinking, and their
/// confidence — the record the bracket is drawn from. The thinking and
/// the confidence are the theory's richest data; the answer is the
/// surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeAnswer {
    /// The question the prober asked.
    pub question: String,
    /// The learner's answer, in their own words.
    pub answer: String,
    /// The learner's reasoning, uncertainty, and reactions.
    pub thinking: String,
    /// The self-reported confidence, when the learner stated one.
    pub confidence: Option<f64>,
}

/// The probe's bracket: the current understanding (what the learner
/// holds, with its brackets) and the pinned goal.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeBracket {
    /// What the learner holds, in their words — the bracket's inside.
    pub current: String,
    /// Where the understanding ends, in the learner's words — the
    /// bracket's edge.
    pub edge: String,
    /// The goal the lesson teaches toward, pinned from the learner's
    /// own articulation.
    pub goal: String,
}

/// A probe session's records: the answers and the bracket. The lesson
/// writes them to the substrate — the answers as facts (the `probe`
/// type) and one `learn_probe` calibration row — and the plan reads the
/// bracket.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeRecord {
    pub session: String,
    pub answers: Vec<ProbeAnswer>,
    pub bracket: ProbeBracket,
}

/// The conversation seam the probe loop drives. A run implements it
/// with the lesson session's own turn machinery — the prober's overlay
/// asks, the learner's reply is read from the conversation; a test
/// scripts the replies, which is what makes the loop's own discipline
/// (one question at a time, the answers captured whole, the bracket
/// drawn from them) testable without a model.
pub trait ProbeConversation {
    /// Ask one question and read the learner's answer: their words,
    /// their thinking, and the confidence they stated. The question
    /// reaches the learner exactly as the prober asked it — the seam
    /// never restates it.
    fn ask(&mut self, question: &str) -> ProbeAnswer;

    /// Draw the bracket from the answers: what the learner holds, where
    /// it ends, and the goal, pinned from the learner's own
    /// articulation rather than supplied by the prober.
    fn bracket(&mut self, answers: &[ProbeAnswer]) -> ProbeBracket;
}

/// The probe loop (the learning plan's C2): ask each question through
/// the conversation seam, capture the learner's answers with their
/// thinking and confidence, then draw the bracket. One question at a
/// time — the loop never batches, because the prober's next question
/// follows the learner's last answer.
pub fn probe_loop(
    session_id: &str,
    conversation: &mut dyn ProbeConversation,
    questions: &[String],
) -> ProbeRecord {
    let mut answers = Vec::with_capacity(questions.len());
    for question in questions {
        answers.push(conversation.ask(question));
    }
    let bracket = conversation.bracket(&answers);
    ProbeRecord {
        session: session_id.to_string(),
        answers,
        bracket,
    }
}

/// Seed the prober role into the home's roles (the learning plan's C2).
/// The role is the lesson run's overlay: it tells the prober to ask,
/// to take the learner's words whole, and never to teach. A seed only
/// fills a home that lacks the role — an owner-edited file is the
/// owner's and is left exactly as it stands.
pub fn seed_prober(home: &Path) -> Result<(), String> {
    let path = crate::roles::roles_dir(home).join(format!("{PROBER_ROLE}.md"));
    if path.exists() {
        return Ok(());
    }
    let fields = crate::roles::RoleFields {
        description: "The learning prober — asks what the learner holds, where it ends, and what they are reaching for.".to_string(),
        tools: Vec::new(),
        skills: Vec::new(),
        shapes: Vec::new(),
        overlay: PROBER_OVERLAY.to_string(),
    };
    crate::roles::write(home, PROBER_ROLE, &fields)
}

/// The prober's overlay — the standing instruction the lesson's run
/// carries beside the harness prompt. The theory's demands are its
/// rules: ask, take the learner's words whole, follow the confusion,
/// and never teach.
pub const PROBER_OVERLAY: &str = r#"You are the prober in a learning session. Your whole job is to
find what the learner already holds and where it ends, and to pin the
goal they are reaching for. You do not teach, explain, or correct —
the teaching is a later session's work, and a prober who explains
steals the lesson's ground.

Ask one question at a time. Ask what the learner thinks is true, how
they know, and where they are least sure. When the learner answers,
take their words whole — never restate them into something tidier
than they said. When they name a confusion, follow it: a confusion is
the exact edge of their understanding, and the lesson is built from
that edge.

Never explain the answer, never fill a silence with the right one, and
never move on while the learner's last answer is unclear. The goal you
pin must be the learner's own articulation of what they want to be
true, not a goal you supply for them. When you pin it, say it back in
their words and ask whether you have it right."#;

/// Record a probe session on the substrate: the answers as facts (the
/// `probe` type, each with its thinking and confidence), the bracket
/// as a fact, and one `learn_probe` calibration row naming the session
/// and the goal. The facts are recallable project knowledge with their
/// provenance; the row is the audit trail the later slices read.
pub fn record_probe(
    memory: &Memory,
    stream: Option<&Stream>,
    record: &ProbeRecord,
) -> Result<(), String> {
    for (index, answer) in record.answers.iter().enumerate() {
        let request = memory::Remember {
            name: format!("probe-{}-{}", record.session, index),
            description: answer.question.clone(),
            fact_type: "probe".to_string(),
            shape: Vec::new(),
            phrases: Vec::new(),
            missing: Vec::new(),
            basis: Basis::ConfirmedBy,
            tier: Tier::Durable,
            valid_until: String::new(),
            source: record.session.clone(),
            // The answer and the thinking ride the body; the body is
            // the record's full content, never a summary.
            body: format!(
                "answer: {}\n\nthinking: {}\n\nconfidence: {}",
                answer.answer,
                answer.thinking,
                answer
                    .confidence
                    .map(|c| format!("{c:.2}"))
                    .unwrap_or_else(|| "(none stated)".to_string())
            ),
        };
        memory
            .remember(&request)
            .map_err(|error| format!("the probe fact did not land: {error}"))?;
    }
    // The bracket is the measure's output and the plan's input — it
    // lands as a fact of the same type, so the recall and the dream
    // pass read it beside the answers it was drawn from.
    let request = memory::Remember {
        name: format!("probe-bracket-{}", record.session),
        description: record.bracket.goal.clone(),
        fact_type: "probe".to_string(),
        shape: Vec::new(),
        phrases: Vec::new(),
        missing: Vec::new(),
        basis: Basis::ConfirmedBy,
        tier: Tier::Durable,
        valid_until: String::new(),
        source: record.session.clone(),
        body: format!(
            "current: {}\n\nedge: {}\n\ngoal: {}",
            record.bracket.current, record.bracket.edge, record.bracket.goal
        ),
    };
    memory
        .remember(&request)
        .map_err(|error| format!("the bracket fact did not land: {error}"))?;
    if let Some(stream) = stream {
        stream.record(Row::LearnProbe(calib::LearnProbeRow {
            ts: calib::stamp(),
            session: record.session.clone(),
            answers: record.answers.len() as u64,
            goal: record.bracket.goal.clone(),
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calib::{self, Stream};
    use crate::memory::Memory;
    use std::collections::VecDeque;

    /// A scratch root, unique per call.
    fn scratch(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fleet-lesson-{tag}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The scripted conversation: the test's stand-in for the lesson
    /// session's own turns. It answers every question in order and
    /// draws a fixed bracket — which is what makes the loop's own
    /// discipline (one question at a time, the answers captured whole)
    /// testable without a model.
    struct Scripted {
        replies: VecDeque<ProbeAnswer>,
        asked: Vec<String>,
        bracket: ProbeBracket,
    }

    impl Scripted {
        fn new(replies: Vec<ProbeAnswer>, bracket: ProbeBracket) -> Self {
            Self {
                replies: replies.into(),
                asked: Vec::new(),
                bracket,
            }
        }
    }

    impl ProbeConversation for Scripted {
        fn ask(&mut self, question: &str) -> ProbeAnswer {
            self.asked.push(question.to_string());
            let mut answer = self
                .replies
                .pop_front()
                .expect("the script answers every question the loop asks");
            // The seam names the question the answer belongs to — the
            // loop never restates it.
            answer.question = question.to_string();
            answer
        }

        fn bracket(&mut self, answers: &[ProbeAnswer]) -> ProbeBracket {
            assert!(!answers.is_empty(), "a bracket is drawn from answers");
            self.bracket.clone()
        }
    }

    fn answer(answer: &str, thinking: &str, confidence: Option<f64>) -> ProbeAnswer {
        ProbeAnswer {
            question: String::new(),
            answer: answer.to_string(),
            thinking: thinking.to_string(),
            confidence,
        }
    }

    #[test]
    fn the_probe_loop_asks_one_at_a_time_and_draws_the_bracket() {
        let questions = vec![
            "what do you hold about this?".to_string(),
            "where does it end?".to_string(),
            "what are you reaching for?".to_string(),
        ];
        let bracket = ProbeBracket {
            current: "the caller holds the request open until the handler returns".to_string(),
            edge: "what happens when the handler blocks is not something they have thought about"
                .to_string(),
            goal: "to know why a blocked handler stalls the whole request".to_string(),
        };
        let mut conversation = Scripted::new(
            vec![
                answer(
                    "the request runs until the handler returns",
                    "I have read this in the server's loop, but I could not say why",
                    Some(0.4),
                ),
                answer(
                    "I do not know what happens when the handler blocks",
                    "this is exactly the part I have not worked through",
                    Some(0.1),
                ),
                answer(
                    "I want to know why a blocked handler stalls the whole request",
                    "that is the thing I keep hitting",
                    Some(0.8),
                ),
            ],
            bracket.clone(),
        );
        let record = probe_loop("2026-11-08-abc", &mut conversation, &questions);
        // One at a time, in order, the prober's own questions.
        assert_eq!(conversation.asked, questions, "the loop asks in order");
        // Every answer carries the learner's thinking and confidence —
        // the theory's richest data, not decoration.
        assert_eq!(record.answers.len(), 3);
        assert_eq!(
            record.answers[0].thinking,
            "I have read this in the server's loop, but I could not say why"
        );
        assert_eq!(record.answers[1].confidence, Some(0.1));
        assert_eq!(record.answers[2].confidence, Some(0.8));
        assert_eq!(record.bracket, bracket);
        assert_eq!(record.session, "2026-11-08-abc");
    }

    #[test]
    fn the_probe_records_land_as_facts_and_one_row() {
        let root = scratch("records");
        let memory = Memory::load(&root);
        let stream_root = scratch("stream");
        let stream = Stream::open(&stream_root);
        let record = ProbeRecord {
            session: "2026-11-08-abc".to_string(),
            answers: vec![answer(
                "the request runs until the handler returns",
                "I have read this but could not say why",
                Some(0.4),
            )],
            bracket: ProbeBracket {
                current: "the caller holds the request open".to_string(),
                edge: "a blocked handler".to_string(),
                goal: "why a blocked handler stalls the request".to_string(),
            },
        };
        record_probe(&memory, Some(&stream), &record).expect("the record lands");
        // The answers land as facts of the `probe` type, with the
        // thinking and the confidence riding the body.
        let facts = memory.facts();
        let answer_fact = facts
            .iter()
            .find(|fact| fact.name == "probe-2026-11-08-abc-0")
            .expect("the answer fact");
        assert_eq!(answer_fact.fact_type, "probe");
        assert_eq!(answer_fact.source, "2026-11-08-abc");
        assert!(answer_fact
            .body
            .contains("thinking: I have read this but could not say why"));
        assert!(answer_fact.body.contains("confidence: 0.40"));
        // The bracket lands beside the answers it was drawn from.
        let bracket_fact = facts
            .iter()
            .find(|fact| fact.name == "probe-bracket-2026-11-08-abc")
            .expect("the bracket fact");
        assert_eq!(
            bracket_fact.description,
            "why a blocked handler stalls the request"
        );
        assert!(bracket_fact.body.contains("edge: a blocked handler"));
        // The audit row lands with the session, the count, and the goal.
        stream.flush();
        let day = calib::today();
        let path = calib::day_path(&stream_root, &day);
        let text = std::fs::read_to_string(&path).expect("the day's stream");
        let rows: Vec<serde_json::Value> = text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        let row = rows
            .iter()
            .find(|row| row.get("op").and_then(|op| op.as_str()) == Some(calib::OP_LEARN_PROBE))
            .expect("one learn_probe row");
        assert_eq!(
            row.get("session").and_then(|v| v.as_str()),
            Some("2026-11-08-abc")
        );
        assert_eq!(row.get("answers").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            row.get("goal").and_then(|v| v.as_str()),
            Some("why a blocked handler stalls the request")
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&stream_root);
    }

    #[test]
    fn the_prober_role_seeds_into_the_home_and_never_overwrites() {
        let home = scratch("roles");
        seed_prober(&home).expect("the prober seeds");
        let role = crate::roles::load(&home, PROBER_ROLE).expect("the prober loads");
        assert!(!role.description.is_empty());
        // The overlay carries the theory's own rule: the prober never
        // teaches — a prober who explains steals the lesson's ground.
        assert!(role.overlay.contains("do not teach"), "{}", role.overlay);
        assert!(
            role.overlay.contains("one question at a time"),
            "{}",
            role.overlay
        );
        // A seed never overwrites an owner-edited file.
        let path = crate::roles::roles_dir(&home).join("prober.md");
        std::fs::write(
            &path,
            "---\nname: prober\ndescription: the owner's own\n---\nowner voice\n",
        )
        .unwrap();
        seed_prober(&home).expect("the second seed lands");
        let role = crate::roles::load(&home, PROBER_ROLE).unwrap();
        assert_eq!(role.description, "the owner's own");
        assert_eq!(role.overlay, "owner voice");
        let _ = std::fs::remove_dir_all(&home);
    }
}
