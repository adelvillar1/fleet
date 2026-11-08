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

/// The teacher role's name — the file under `<home>/roles/` the lesson's
/// teaching run rides as its overlay.
pub const TEACHER_ROLE: &str = "teacher";

/// The repair rounds one step may take before the lesson stops rather
/// than building on ground that is not held — the house's rounds
/// discipline, applied to a confusion the learner cannot yet step past.
pub const MAX_REPAIRS: usize = 3;

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

/// One step's check: the learner's own words after the explanation,
/// and whether the step is held. A confusion is the learner's own
/// account of where the step did not land — the repair's whole target.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// The learner's words, taken whole.
    pub words: String,
    /// Whether the step reads as held — the learner can state it and
    /// step on it.
    pub held: bool,
    /// The confusion the learner named, when the step did not land.
    pub confusion: Option<String>,
}

/// The lesson's terminal state: the learner's own words at the goal,
/// and whether they read as the goal held as obviously true. The
/// theory's terminal condition is not recall or coverage — it is the
/// felt experience of "obviously true", read from the learner's words.
#[derive(Debug, Clone, PartialEq)]
pub struct SelfEvidence {
    pub words: String,
    pub obviously_true: bool,
}

/// One taught step's record: the prediction, the check, and the repairs
/// a confusion took before the step was held.
#[derive(Debug, Clone, PartialEq)]
pub struct TeachStep {
    /// The proposition's node id.
    pub node: String,
    /// The proposition the step taught.
    pub proposition: String,
    /// The learner's prediction before the explanation.
    pub prediction: String,
    /// The check after the explanation.
    pub check: Check,
    /// The repairs: each confusion and the check that followed it.
    pub repairs: Vec<(String, Check)>,
}

/// A lesson's whole record: the steps in the teaching order, and the
/// goal's self-evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct LessonOutcome {
    pub steps: Vec<TeachStep>,
    pub self_evidence: SelfEvidence,
    /// True when every step is held and the goal feels obviously true —
    /// a lesson that stopped on an unrepaired confusion is not complete.
    pub complete: bool,
}

/// The conversation seam the teaching flow drives. A run implements it
/// with the lesson session's own turns — the teacher's overlay asks,
/// the learner answers — and a test scripts the replies.
pub trait TeachingConversation {
    /// Ask the learner to predict the step before it is explained: the
    /// prediction attaches the step to what is already held, which is
    /// what makes the explanation land as obviously true.
    fn predict(&mut self, proposition: &str) -> String;

    /// Offer the step's explanation and read the learner's check.
    fn check(&mut self, proposition: &str) -> Check;

    /// Repair a confusion the check surfaced, and read the check that
    /// follows the repair. Nothing is built on a step that is not held.
    fn repair(&mut self, confusion: &str, proposition: &str) -> Check;

    /// Read the goal's self-evidence in the learner's own words.
    fn self_evidence(&mut self, goal: &str) -> SelfEvidence;
}

/// The teaching loop (the learning plan's C4): move one reasoning step
/// at a time through the plan's teaching order — predict, explain,
/// check — repairing a confusion before anything is built on it, and
/// stop at the goal's self-evidence.
///
/// The theory's own discipline: a foundation is what the learner
/// already holds and is never re-taught; a step whose confusion will
/// not repair within [`MAX_REPAIRS`] stops the lesson rather than
/// building on ground that is not held; and the terminal condition is
/// the learner's own "obviously true", never the lesson's coverage.
pub fn teach(
    plan: &crate::graph::LessonPlan,
    conversation: &mut dyn TeachingConversation,
) -> LessonOutcome {
    let mut steps: Vec<TeachStep> = Vec::new();
    let mut complete = true;
    for node in plan.teaching_order() {
        // A foundation is what the learner holds — the minimal tree
        // re-teaches nothing.
        if plan.is_foundation(&node) {
            continue;
        }
        let proposition = plan.proposition(&node).unwrap_or_default().to_string();
        let prediction = conversation.predict(&proposition);
        let mut check = conversation.check(&proposition);
        let mut repairs: Vec<(String, Check)> = Vec::new();
        // A confusion is repaired before continuing — the error
        // surfaces the moment it is stepped on, and nothing is built
        // on it until it is repaired.
        let mut round = 0;
        while !check.held && round < MAX_REPAIRS {
            let confusion = check
                .confusion
                .clone()
                .unwrap_or_else(|| "the learner named no confusion".to_string());
            check = conversation.repair(&confusion, &proposition);
            repairs.push((confusion, check.clone()));
            round += 1;
        }
        if !check.held {
            // The ground is not held: the lesson stops here rather than
            // teaching a step that rests on a confusion.
            complete = false;
            steps.push(TeachStep {
                node,
                proposition,
                prediction,
                check,
                repairs,
            });
            return LessonOutcome {
                steps,
                self_evidence: SelfEvidence {
                    words: String::new(),
                    obviously_true: false,
                },
                complete,
            };
        }
        steps.push(TeachStep {
            node,
            proposition,
            prediction,
            check,
            repairs,
        });
    }
    let self_evidence = conversation.self_evidence(&plan.goal);
    if !self_evidence.obviously_true {
        complete = false;
    }
    LessonOutcome {
        steps,
        self_evidence,
        complete,
    }
}

/// Seed the teacher role into the home's roles (the learning plan's C4).
/// A seed only fills a home that lacks the role — an owner-edited file
/// is the owner's and is left exactly as it stands.
pub fn seed_teacher(home: &Path) -> Result<(), String> {
    let path = crate::roles::roles_dir(home).join(format!("{TEACHER_ROLE}.md"));
    if path.exists() {
        return Ok(());
    }
    let fields = crate::roles::RoleFields {
        description: "The learning teacher — moves one reasoning step at a time, from what the learner holds to the goal.".to_string(),
        tools: Vec::new(),
        skills: Vec::new(),
        shapes: Vec::new(),
        overlay: TEACHER_OVERLAY.to_string(),
    };
    crate::roles::write(home, TEACHER_ROLE, &fields)
}

/// The teacher's overlay — the standing instruction the teaching run
/// carries beside the harness prompt. The theory's demands are its
/// rules: one step at a time, predict before explaining, a check after
/// every step, the confusion repaired first, and the goal's
/// self-evidence the only terminal condition.
pub const TEACHER_OVERLAY: &str = r#"You are the teacher in a learning session. You move one reasoning
step at a time, from what the learner already holds to the goal they
pinned. Never teach two steps at once: a step the learner cannot hold
is not a step, and a lecture is a pile of them.

Before you explain a step, ask the learner to predict it. The
prediction attaches the step to what they hold, which is what makes
the explanation land as obviously true. Then explain it — the clearest
form you have, no meta-narration, every sentence installing
understanding or cut as overhead. Then check: ask the learner to state
the step back or to use it, and take their words whole.

When the learner names a confusion, repair it before anything is built
on it. A confusion is the exact edge of their understanding, and the
repair is the lesson's most important work. If a step will not hold
after honest repair, stop there and say so — never build on ground
that is not held.

You never teach the plan's own reasoning back to the learner, and you
never name the goal as reached. The lesson ends when the learner's own
words say the goal feels obviously true. Ask them, and take their
answer."#;

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

/// Record a lesson on the substrate (the learning plan's C4): one
/// `check` fact per taught step — the prediction, the check, and every
/// repair with the confusion it answered — and one `lesson` fact
/// carrying the goal's self-evidence and whether the lesson completed.
/// The confusions ride the facts whole: they are the friction the
/// self-improvement feed reads.
pub fn record_lesson(
    memory: &Memory,
    session: &str,
    plan: &crate::graph::LessonPlan,
    outcome: &LessonOutcome,
) -> Result<(), String> {
    for step in &outcome.steps {
        let mut body = format!(
            "prediction: {}\n\ncheck: {}\n\nheld: {}",
            step.prediction, step.check.words, step.check.held
        );
        if let Some(confusion) = &step.check.confusion {
            body.push_str(&format!("\n\nconfusion: {confusion}"));
        }
        for (confusion, check) in &step.repairs {
            body.push_str(&format!(
                "\n\nrepair of `{confusion}`: {}\nheld: {}",
                check.words, check.held
            ));
        }
        let request = memory::Remember {
            name: format!("check-{session}-{}", step.node),
            description: step.proposition.clone(),
            fact_type: "check".to_string(),
            shape: Vec::new(),
            phrases: Vec::new(),
            missing: Vec::new(),
            premises: Vec::new(),
            basis: Basis::ConfirmedBy,
            tier: Tier::Durable,
            valid_until: String::new(),
            source: session.to_string(),
            body,
        };
        memory
            .remember(&request)
            .map_err(|error| format!("the check fact did not land: {error}"))?;
    }
    let confusions: Vec<String> = outcome
        .steps
        .iter()
        .filter_map(|step| step.check.confusion.clone())
        .collect();
    let request = memory::Remember {
        name: format!("lesson-{session}"),
        description: plan
            .goal_proposition()
            .unwrap_or(plan.goal.as_str())
            .to_string(),
        fact_type: "lesson".to_string(),
        shape: Vec::new(),
        phrases: Vec::new(),
        missing: confusions.clone(),
        premises: Vec::new(),
        basis: Basis::ConfirmedBy,
        tier: Tier::Durable,
        valid_until: String::new(),
        source: session.to_string(),
        body: format!(
            "complete: {}\n\nself-evidence: {}\n\nconfusions: {}",
            outcome.complete,
            outcome.self_evidence.words,
            if confusions.is_empty() {
                "(none)".to_string()
            } else {
                confusions.join("; ")
            }
        ),
    };
    memory
        .remember(&request)
        .map_err(|error| format!("the lesson fact did not land: {error}"))?;
    Ok(())
}

/// Record a lesson plan on the substrate (the learning plan's C3): one
/// `lesson-plan` fact naming the goal, the teaching order, and the
/// coarse view the learner reads. The plan is project knowledge, not a
/// silo — the recall, the dream pass, and the face's Learning pane read
/// it like any other fact.
pub fn record_plan(
    memory: &Memory,
    session: &str,
    plan: &crate::graph::LessonPlan,
) -> Result<(), String> {
    // The description is the goal's own proposition — what the lesson
    // teaches toward — never the node's id, which is a join and not a
    // sentence the recall or the learner can read.
    let description = plan
        .goal_proposition()
        .unwrap_or(plan.goal.as_str())
        .to_string();
    let request = memory::Remember {
        name: format!("lesson-plan-{session}"),
        description,
        fact_type: "lesson-plan".to_string(),
        shape: Vec::new(),
        phrases: Vec::new(),
        missing: Vec::new(),
        premises: Vec::new(),
        basis: Basis::ConfirmedBy,
        tier: Tier::Durable,
        valid_until: String::new(),
        source: session.to_string(),
        body: format!(
            "teaching order: {}\n\n{}",
            plan.teaching_order().join(" \u{2192} "),
            plan.coarse_view()
        ),
    };
    memory
        .remember(&request)
        .map_err(|error| format!("the lesson plan did not land: {error}"))?;
    Ok(())
}

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
            premises: Vec::new(),
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
        premises: Vec::new(),
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

/// Record one proposition on the substrate (the learning plan's C5): a
/// fact of the `proposition` type whose `premises` field carries the
/// chain's joint-premise structure — the propositions the claim rests
/// on TOGETHER, which the pairwise recall graph cannot carry. The
/// claim is the fact's sentence; the premises ride the frontmatter.
pub fn record_proposition(
    memory: &Memory,
    name: &str,
    claim: &str,
    premises: &[String],
    source: &str,
) -> Result<memory::Fact, String> {
    // The write goes through `Remember::parse`, the tool's own
    // validator — a proposition whose premise is not a proposition name
    // is refused rather than written as a chain that cannot resolve.
    let arguments = serde_json::json!({
        "name": name,
        "description": claim,
        "type": "proposition",
        "premises": premises,
        "basis": "confirmed-by",
        "source": source,
    });
    let request = memory::Remember::parse(&arguments)
        .map_err(|error| format!("the proposition is not writable: {error}"))?;
    memory
        .remember(&request)
        .map_err(|error| format!("the proposition did not land: {error}"))
}

/// One proposition the extraction proposes from the teaching prose.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedProposition {
    /// The proposition's name — a fact name, the chain's join.
    pub name: String,
    /// The claim, in one sentence.
    pub claim: String,
    /// The propositions this one rests on, by name.
    pub premises: Vec<String>,
    /// The prose the claim came from — the evidence the extraction
    /// read, kept so the audit can say what the claim accounts for.
    pub evidence: Vec<String>,
}

/// The extraction's coverage gate: the fraction of the teaching prose's
/// significant tokens the propositions must account for. Below it the
/// extraction left the teaching's own content behind, and the graph is
/// only as good as the extract — so the merge is refused rather than
/// quietly incomplete.
pub const MIN_COVERAGE: f64 = 0.8;

/// The audit's result: the prose's tokens, how many the extraction
/// covers, and the tokens nothing accounted for.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractionAudit {
    pub prose_tokens: usize,
    pub covered_tokens: usize,
    /// The covered fraction, in `[0, 1]`.
    pub coverage: f64,
    /// Whether the gate holds.
    pub passed: bool,
    /// The tokens no proposition accounted for.
    pub uncovered: Vec<String>,
}

/// The words that carry no content of their own — the audit counts the
/// prose's significant tokens, and a function word is not one.
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "been", "but", "by", "for", "from", "has", "have",
    "how", "i", "in", "is", "it", "its", "of", "on", "or", "so", "that", "the", "their", "there",
    "this", "to", "was", "we", "were", "what", "when", "where", "which", "who", "why", "will",
    "with", "would", "you", "your",
];

/// The prose's significant tokens: the words, lowercased, the function
/// words dropped. The audit's denominator.
fn significant_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    for word in text.split(|c: char| !c.is_alphanumeric()) {
        let word = word.to_ascii_lowercase();
        if word.is_empty() || STOPWORDS.contains(&word.as_str()) {
            continue;
        }
        if !tokens.contains(&word) {
            tokens.push(word);
        }
    }
    tokens
}

/// Audit the extraction against the teaching prose: how much of the
/// prose's own content the propositions account for. The gate the
/// graph's quality rides on — an extraction that leaves the teaching
/// uncovered is not the graph, and the audit names what it left.
pub fn audit(prose: &str, extracted: &[ExtractedProposition]) -> ExtractionAudit {
    let prose_tokens = significant_tokens(prose);
    let mut claimed: Vec<String> = Vec::new();
    for proposition in extracted {
        claimed.extend(significant_tokens(&proposition.claim));
        for premise in &proposition.premises {
            claimed.extend(significant_tokens(premise));
        }
    }
    let uncovered: Vec<String> = prose_tokens
        .iter()
        .filter(|token| !claimed.contains(token))
        .cloned()
        .collect();
    let covered_tokens = prose_tokens.len() - uncovered.len();
    let coverage = if prose_tokens.is_empty() {
        1.0
    } else {
        covered_tokens as f64 / prose_tokens.len() as f64
    };
    ExtractionAudit {
        prose_tokens: prose_tokens.len(),
        covered_tokens,
        coverage,
        passed: coverage >= MIN_COVERAGE,
        uncovered,
    }
}

/// Merge the extraction into the substrate, deterministically: a
/// proposition the corpus already holds gains the new premises — the
/// ordered union — and keeps its own sentence, because the first
/// writer's claim stands and a later extraction adds to the chain
/// rather than rewording it. A new proposition is written whole.
///
/// Returns how many propositions landed: the new and the joined.
pub fn merge_extraction(
    memory: &Memory,
    extracted: &[ExtractedProposition],
) -> Result<usize, String> {
    let existing = memory.facts();
    let mut landed = 0;
    for proposition in extracted {
        let held = existing.iter().find(|fact| fact.name == proposition.name);
        match held {
            Some(fact) => {
                let mut premises = fact.premises.clone();
                for premise in &proposition.premises {
                    if !premises.contains(premise) {
                        premises.push(premise.clone());
                    }
                }
                if premises != fact.premises {
                    memory::rewrite_field(&fact.path, "premises", &memory::flow_list(&premises))
                        .map_err(|error| format!("the chain did not land: {error}"))?;
                }
            }
            None => {
                record_proposition(
                    memory,
                    &proposition.name,
                    &proposition.claim,
                    &proposition.premises,
                    "",
                )?;
            }
        }
        landed += 1;
    }
    Ok(landed)
}

/// The extraction pipeline (the learning plan's C5): extract → audit
/// → merge, over the teaching prose. The audit gates the merge — an
/// extraction that leaves the teaching's own content uncovered is
/// refused by name, with the uncovered tokens, rather than merged
/// into the graph as a silently incomplete map.
pub fn extract_pipeline(
    memory: &Memory,
    prose: &str,
    extracted: &[ExtractedProposition],
) -> Result<ExtractionAudit, String> {
    let audit = audit(prose, extracted);
    if !audit.passed {
        return Err(format!(
            "the extraction covers {:.0}% of the teaching prose's {} significant tokens — below              the {:.0}% gate. Uncovered: {}",
            audit.coverage * 100.0,
            audit.prose_tokens,
            MIN_COVERAGE * 100.0,
            audit.uncovered.join(", ")
        ));
    }
    merge_extraction(memory, extracted)?;
    Ok(audit)
}

/// The practitioner role's name — the file under `<home>/roles/` the
/// practice session's run rides as its overlay.
pub const PRACTITIONER_ROLE: &str = "practitioner";

/// One practice problem: a varied setting that requires the knowledge
/// to come to mind **unnamed**. The setting never names the
/// propositions it exercises — a problem that names the answer is a
/// recall test, not a practice, and the theory's whole point is that
/// the knowledge arrives without the name attached.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeProblem {
    /// The problem's own setting, in the practitioner's words.
    pub setting: String,
    /// The propositions the problem exercises, by name — the
    /// practitioner's own record, never shown to the learner.
    pub exercises: Vec<String>,
}

/// One problem's outcome: the learner's answer and whether it held.
/// Session evidence, never a mastery rating — the graph stores the
/// ideas and their connections, not how well the learner did.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeOutcome {
    pub problem: PracticeProblem,
    pub answer: String,
    pub held: bool,
}

/// The conversation seam the practice loop drives. A run implements it
/// with the practice session's own turns; a test scripts the answers.
pub trait PracticeConversation {
    /// Pose one problem and read the learner's answer.
    fn pose(&mut self, problem: &PracticeProblem) -> String;

    /// Judge this answer against the problem's own shape — whether it
    /// held, never a grade.
    fn judge(&mut self, problem: &PracticeProblem, answer: &str) -> bool;
}

/// The practice loop (the learning plan's C6): varied problems, one
/// active per turn, each requiring the knowledge to come to mind
/// unnamed. The loop is sequential by construction — a problem is
/// posed, answered, and judged before the next begins, because two
/// active problems at once is a test, not a practice.
///
/// A problem whose setting names one of the propositions it exercises
/// is refused rather than posed: the unnamed retrieval is the whole
/// mechanism, and a setting that names the answer teaches the name
/// instead of the knowledge.
pub fn practice(
    memory: &Memory,
    problems: &[PracticeProblem],
    conversation: &mut dyn PracticeConversation,
) -> Result<Vec<PracticeOutcome>, String> {
    let facts = memory.facts();
    let mut outcomes = Vec::with_capacity(problems.len());
    for problem in problems {
        for exercise in &problem.exercises {
            // The proposition's own claim, when the corpus holds it —
            // the knowledge the problem must not name.
            let claim = facts
                .iter()
                .find(|fact| fact.name == *exercise)
                .map(|fact| fact.description.clone())
                .unwrap_or_else(|| exercise.clone());
            if names_the_knowledge(&problem.setting, &claim) {
                return Err(format!(
                    "the problem's setting names `{exercise}`, which it exercises — a practice \
                     problem requires the knowledge to come to mind unnamed"
                ));
            }
        }
        let answer = conversation.pose(problem);
        let held = conversation.judge(problem, &answer);
        outcomes.push(PracticeOutcome {
            problem: problem.clone(),
            answer,
            held,
        });
    }
    Ok(outcomes)
}

/// Whether a problem's setting names the knowledge it exercises: the
/// claim's significant tokens, covered by the setting's own. A setting
/// that carries the claim is a recall test — it hands the learner the
/// name — and the practice's whole mechanism is the retrieval without
/// it. The same token rule the extraction's audit uses, pointed the
/// other way.
fn names_the_knowledge(setting: &str, claim: &str) -> bool {
    let claim_tokens = significant_tokens(claim);
    if claim_tokens.is_empty() {
        return false;
    }
    let setting_tokens = significant_tokens(setting);
    let covered = claim_tokens
        .iter()
        .filter(|token| setting_tokens.contains(token))
        .count();
    covered as f64 / claim_tokens.len() as f64 >= MIN_COVERAGE
}

/// Record the practice outcomes on the substrate (the learning plan's
/// C6): one `practice` fact per problem, carrying the setting, the
/// answer, and whether it held. The outcomes are session evidence —
/// the fact records what happened, never a mastery rating.
pub fn record_practice(
    memory: &Memory,
    session: &str,
    outcomes: &[PracticeOutcome],
) -> Result<(), String> {
    for (index, outcome) in outcomes.iter().enumerate() {
        let request = memory::Remember {
            name: format!("practice-{session}-{index}"),
            description: outcome.problem.setting.clone(),
            fact_type: "practice".to_string(),
            shape: Vec::new(),
            phrases: Vec::new(),
            missing: Vec::new(),
            premises: outcome.problem.exercises.clone(),
            basis: Basis::ConfirmedBy,
            tier: Tier::Durable,
            valid_until: String::new(),
            source: session.to_string(),
            body: format!(
                "answer: {}\n\nheld: {}\n\nexercises: {}",
                outcome.answer,
                outcome.held,
                if outcome.problem.exercises.is_empty() {
                    "(none)".to_string()
                } else {
                    outcome.problem.exercises.join(", ")
                }
            ),
        };
        memory
            .remember(&request)
            .map_err(|error| format!("the practice fact did not land: {error}"))?;
    }
    Ok(())
}

/// Seed the practitioner role into the home's roles (the learning
/// plan's C6). A seed only fills a home that lacks the role — an
/// owner-edited file is the owner's and is left exactly as it stands.
pub fn seed_practitioner(home: &Path) -> Result<(), String> {
    let path = crate::roles::roles_dir(home).join(format!("{PRACTITIONER_ROLE}.md"));
    if path.exists() {
        return Ok(());
    }
    let fields = crate::roles::RoleFields {
        description: "The learning practitioner — poses varied problems that require the knowledge to come to mind unnamed.".to_string(),
        tools: Vec::new(),
        skills: Vec::new(),
        shapes: Vec::new(),
        overlay: PRACTITIONER_OVERLAY.to_string(),
    };
    crate::roles::write(home, PRACTITIONER_ROLE, &fields)
}

/// The practitioner's overlay — the standing instruction the practice
/// session carries beside the harness prompt. The theory's demands are
/// its rules: the problems are varied, the knowledge is never named,
/// one problem is active at a time, and a hint adapts to the learner's
/// own answer.
pub const PRACTITIONER_OVERLAY: &str = r#"You are the practitioner in a learning session. You pose problems.
The problems are varied — different settings, different surfaces — so
that the knowledge must come to mind WITHOUT being named. Never name
the idea the problem exercises, not in the setting, not in a hint, not
in the answer's framing. A problem that names the answer is a recall
test, and a recall test teaches the name instead of the knowledge.

One problem is active at a time. Pose it, let the learner work, and
read their answer whole. When the answer does not hold, adapt: a hint
that meets the learner where their answer went wrong, never the answer
itself. When it holds, move on.

You never grade. You say whether the answer held, and you record the
setting and the answer as they were. The learner's performance is not a
score and must never become one — the session records what happened,
nothing more."#;

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

    #[test]
    fn the_lesson_plan_lands_as_a_fact_with_its_teaching_order() {
        let root = scratch("plan");
        let memory = Memory::load(&root);
        let nodes = vec![
            crate::graph::LessonNode {
                id: "holds".to_string(),
                proposition: "a request runs until its handler returns".to_string(),
                premises: Vec::new(),
                cost: 1,
            },
            crate::graph::LessonNode {
                id: "goal".to_string(),
                proposition: "why a blocked handler stalls the request".to_string(),
                premises: vec!["holds".to_string()],
                cost: 2,
            },
        ];
        let plan = crate::graph::build_lesson_plan("goal", &nodes).expect("the tree builds");
        record_plan(&memory, "2026-11-08-abc", &plan).expect("the plan lands");
        let facts = memory.facts();
        let fact = facts
            .iter()
            .find(|fact| fact.name == "lesson-plan-2026-11-08-abc")
            .expect("the plan fact");
        assert_eq!(fact.fact_type, "lesson-plan");
        assert_eq!(fact.description, "why a blocked handler stalls the request");
        assert!(
            fact.body.contains("teaching order: holds \u{2192} goal"),
            "{}",
            fact.body
        );
        assert!(fact.body.contains("graph TD"), "{}", fact.body);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The scripted teaching conversation: the test's stand-in for the
    /// lesson session's own turns.
    struct ScriptedTeaching {
        predictions: VecDeque<String>,
        checks: VecDeque<Check>,
        repairs: VecDeque<Check>,
        self_evidence: SelfEvidence,
        taught: Vec<String>,
        repaired: Vec<String>,
    }

    impl TeachingConversation for ScriptedTeaching {
        fn predict(&mut self, proposition: &str) -> String {
            self.taught.push(proposition.to_string());
            self.predictions.pop_front().unwrap_or_default()
        }

        fn check(&mut self, _proposition: &str) -> Check {
            self.checks
                .pop_front()
                .expect("the script answers every check")
        }

        fn repair(&mut self, confusion: &str, _proposition: &str) -> Check {
            self.repaired.push(confusion.to_string());
            self.repairs
                .pop_front()
                .expect("the script answers every repair")
        }

        fn self_evidence(&mut self, _goal: &str) -> SelfEvidence {
            self.self_evidence.clone()
        }
    }

    /// A plan with one foundation and two taught steps.
    fn teaching_plan() -> crate::graph::LessonPlan {
        let nodes = vec![
            crate::graph::LessonNode {
                id: "holds".to_string(),
                proposition: "a request runs until its handler returns".to_string(),
                premises: Vec::new(),
                cost: 1,
            },
            crate::graph::LessonNode {
                id: "blocks".to_string(),
                proposition: "a handler that blocks does not return".to_string(),
                premises: vec!["holds".to_string()],
                cost: 2,
            },
            crate::graph::LessonNode {
                id: "goal".to_string(),
                proposition: "why a blocked handler stalls the request".to_string(),
                premises: vec!["blocks".to_string()],
                cost: 2,
            },
        ];
        crate::graph::build_lesson_plan("goal", &nodes).expect("the tree builds")
    }

    fn held(words: &str) -> Check {
        Check {
            words: words.to_string(),
            held: true,
            confusion: None,
        }
    }

    fn confused(words: &str, confusion: &str) -> Check {
        Check {
            words: words.to_string(),
            held: false,
            confusion: Some(confusion.to_string()),
        }
    }

    #[test]
    fn the_teaching_loop_moves_one_step_and_repairs_before_continuing() {
        let plan = teaching_plan();
        let mut conversation = ScriptedTeaching {
            predictions: VecDeque::from(vec![
                "I think the handler blocks the request".to_string(),
                "I think the request waits for the handler".to_string(),
            ]),
            checks: VecDeque::from(vec![
                confused(
                    "I thought a returning handler was the same as a finished request",
                    "the handler returning and the request finishing are the same in my head",
                ),
                held("the blocked handler does not return, so the request does not finish"),
                held("a blocked handler stalls the whole request"),
            ]),
            repairs: VecDeque::from(vec![held(
                "they are different: the handler returning is one thing, the request finishing another",
            )]),
            self_evidence: SelfEvidence {
                words: "yes — obviously, the request is stuck because the handler never comes back"
                    .to_string(),
                obviously_true: true,
            },
            taught: Vec::new(),
            repaired: Vec::new(),
        };
        let outcome = teach(&plan, &mut conversation);
        // The foundation is never re-taught; the two steps move in the
        // plan's order.
        assert_eq!(
            conversation.taught,
            vec![
                "a handler that blocks does not return",
                "why a blocked handler stalls the request"
            ]
        );
        assert_eq!(outcome.steps.len(), 2);
        assert_eq!(outcome.steps[0].node, "blocks");
        assert_eq!(
            outcome.steps[0].prediction,
            "I think the handler blocks the request"
        );
        // The confusion was repaired before the next step, and the
        // repair's check is recorded.
        assert_eq!(conversation.repaired.len(), 1);
        assert_eq!(outcome.steps[0].repairs.len(), 1);
        assert_eq!(
            outcome.steps[0].repairs[0].0,
            "the handler returning and the request finishing are the same in my head"
        );
        assert!(outcome.steps[0].repairs[0].1.held);
        assert!(outcome.steps[0].check.held, "the repair held the step");
        assert!(outcome.complete, "every step held and the goal felt true");
        assert!(outcome.self_evidence.obviously_true);
    }

    #[test]
    fn the_teaching_loop_stops_on_a_confusion_it_cannot_repair() {
        let plan = teaching_plan();
        let mut conversation = ScriptedTeaching {
            predictions: VecDeque::from(vec!["a prediction".to_string()]),
            checks: VecDeque::from(vec![confused(
                "I do not see it",
                "the blocking step will not land",
            )]),
            repairs: VecDeque::from(vec![
                confused("still not", "the blocking step will not land"),
                confused("still not", "the blocking step will not land"),
                confused("still not", "the blocking step will not land"),
            ]),
            self_evidence: SelfEvidence {
                words: String::new(),
                obviously_true: false,
            },
            taught: Vec::new(),
            repaired: Vec::new(),
        };
        let outcome = teach(&plan, &mut conversation);
        // The repair rounds are bounded, and the lesson stops rather
        // than building on ground that is not held.
        assert_eq!(conversation.repaired.len(), MAX_REPAIRS);
        assert_eq!(outcome.steps.len(), 1, "only the step that failed");
        assert_eq!(outcome.steps[0].node, "blocks");
        assert!(!outcome.steps[0].check.held);
        assert!(
            !outcome.complete,
            "an unrepaired confusion is not a completed lesson"
        );
        assert!(outcome.self_evidence.words.is_empty());
    }

    #[test]
    fn the_teaching_loop_ends_at_the_goals_self_evidence() {
        let plan = teaching_plan();
        let mut conversation = ScriptedTeaching {
            predictions: VecDeque::from(vec!["a prediction".to_string(), "another".to_string()]),
            checks: VecDeque::from(vec![
                held("the blocked handler does not return"),
                held("a blocked handler stalls the request"),
            ]),
            repairs: VecDeque::new(),
            self_evidence: SelfEvidence {
                words: "I can say it, but it does not feel obvious yet".to_string(),
                obviously_true: false,
            },
            taught: Vec::new(),
            repaired: Vec::new(),
        };
        let outcome = teach(&plan, &mut conversation);
        // Every step held, but the goal is not obviously true — the
        // lesson is not complete, because coverage is not the terminal
        // condition.
        assert_eq!(outcome.steps.len(), 2);
        assert!(outcome.steps.iter().all(|step| step.check.held));
        assert!(
            !outcome.complete,
            "the learner's own words say the goal is not obviously true"
        );
    }

    #[test]
    fn the_teacher_role_seeds_into_the_home() {
        let home = scratch("teacher-role");
        seed_teacher(&home).expect("the teacher seeds");
        let role = crate::roles::load(&home, TEACHER_ROLE).expect("the teacher loads");
        assert!(role.overlay.contains("one reasoning"), "{}", role.overlay);
        assert!(role.overlay.contains("predict"), "{}", role.overlay);
        assert!(role.overlay.contains("obviously true"), "{}", role.overlay);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn the_lesson_records_land_as_facts_with_their_confusions() {
        let root = scratch("lesson-record");
        let memory = Memory::load(&root);
        let plan = teaching_plan();
        let outcome = LessonOutcome {
            steps: vec![TeachStep {
                node: "blocks".to_string(),
                proposition: "a handler that blocks does not return".to_string(),
                prediction: "I think the handler blocks the request".to_string(),
                check: held("the blocked handler does not return"),
                repairs: vec![(
                    "the handler returning and the request finishing are the same in my head"
                        .to_string(),
                    held("they are different"),
                )],
            }],
            self_evidence: SelfEvidence {
                words: "obviously true now".to_string(),
                obviously_true: true,
            },
            complete: true,
        };
        record_lesson(&memory, "2026-11-08-abc", &plan, &outcome).expect("the lesson lands");
        let facts = memory.facts();
        let check = facts
            .iter()
            .find(|fact| fact.name == "check-2026-11-08-abc-blocks")
            .expect("the check fact");
        assert_eq!(check.fact_type, "check");
        assert!(check
            .body
            .contains("prediction: I think the handler blocks the request"));
        assert!(check.body.contains(
            "repair of `the handler returning and the request finishing are the same in my head`"
        ));
        let lesson = facts
            .iter()
            .find(|fact| fact.name == "lesson-2026-11-08-abc")
            .expect("the lesson fact");
        assert_eq!(lesson.fact_type, "lesson");
        assert!(lesson.body.contains("complete: true"));
        assert!(lesson.body.contains("self-evidence: obviously true now"));
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the knowledge graph (the learning plan's C5) ----------------------

    fn proposition(name: &str, claim: &str, premises: &[&str]) -> ExtractedProposition {
        ExtractedProposition {
            name: name.to_string(),
            claim: claim.to_string(),
            premises: premises.iter().map(|p| p.to_string()).collect(),
            evidence: vec![claim.to_string()],
        }
    }

    #[test]
    fn a_proposition_lands_as_a_fact_with_its_chain() {
        let root = scratch("proposition");
        let memory = Memory::load(&root);
        let fact = record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &["a-request-runs-until-its-handler-returns".to_string()],
            "2026-11-08-abc",
        )
        .expect("the proposition lands");
        assert_eq!(fact.fact_type, "proposition");
        assert_eq!(
            fact.premises,
            vec!["a-request-runs-until-its-handler-returns"]
        );
        // The chain reads back off the file, not the write's own return.
        let loaded = Memory::load(&root);
        let facts = loaded.facts();
        let fact = facts
            .iter()
            .find(|fact| fact.name == "a-blocked-handler-stalls")
            .expect("the proposition loads");
        assert_eq!(
            fact.premises,
            vec!["a-request-runs-until-its-handler-returns"]
        );
        assert_eq!(fact.source, "2026-11-08-abc");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_premise_that_is_not_a_proposition_name_is_refused() {
        let root = scratch("premise-name");
        let memory = Memory::load(&root);
        let error = record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &["Not A Name".to_string()],
            "",
        )
        .expect_err("a premise that is not a name");
        assert!(error.contains("not a proposition name"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_extraction_audit_gates_the_merge() {
        let root = scratch("audit");
        let memory = Memory::load(&root);
        let prose = "a blocked handler does not return, so the request waits and the \
                     connection is held open until the handler comes back or the \
                     client gives up";
        // The extraction accounts for almost none of the prose.
        let extracted = vec![proposition(
            "a-blocked-handler-stalls",
            "a blocked handler stalls the request",
            &[],
        )];
        let audit = audit(prose, &extracted);
        assert!(!audit.passed, "the extraction left the prose behind");
        assert!(!audit.uncovered.is_empty());
        let error =
            extract_pipeline(&memory, prose, &extracted).expect_err("the gate refuses the merge");
        assert!(error.contains("below"), "{error}");
        // And nothing landed: the graph is not silently incomplete.
        assert!(
            Memory::load(&root)
                .facts()
                .iter()
                .all(|fact| fact.name != "a-blocked-handler-stalls"),
            "a refused extraction writes nothing"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_extraction_merges_deterministically() {
        let root = scratch("merge");
        let memory = Memory::load(&root);
        record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &["a-request-runs-until-its-handler-returns".to_string()],
            "2026-11-08-abc",
        )
        .expect("the first proposition lands");
        let prose = "a blocked handler stalls the whole request and the connection stays open";
        let extracted = vec![
            proposition(
                "a-blocked-handler-stalls",
                "a blocked handler stalls the whole request",
                &["a-connection-stays-open"],
            ),
            proposition(
                "a-connection-stays-open",
                "the connection stays open while the handler is blocked",
                &["a-blocked-handler-stalls"],
            ),
        ];
        let audit = extract_pipeline(&memory, prose, &extracted).expect("the gate holds");
        assert!(audit.passed);
        // The held proposition gained the new premise — the ordered
        // union — and kept its own sentence.
        let loaded = Memory::load(&root);
        let facts = loaded.facts();
        let held = facts
            .iter()
            .find(|fact| fact.name == "a-blocked-handler-stalls")
            .expect("the held proposition");
        assert_eq!(
            held.premises,
            vec![
                "a-request-runs-until-its-handler-returns".to_string(),
                "a-connection-stays-open".to_string()
            ],
            "the chain grew by the ordered union"
        );
        assert_eq!(
            held.description,
            "a blocked handler stalls the whole request"
        );
        // The new proposition landed whole.
        let fresh = facts
            .iter()
            .find(|fact| fact.name == "a-connection-stays-open")
            .expect("the new proposition");
        assert_eq!(fresh.premises, vec!["a-blocked-handler-stalls"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_proposition_recalls_like_project_knowledge() {
        let root = scratch("recall");
        let memory = Memory::load(&root);
        record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &["a-request-runs-until-its-handler-returns".to_string()],
            "2026-11-08-abc",
        )
        .expect("the proposition lands");
        let hits = memory.recall("why does a blocked handler stall the request");
        assert!(
            hits.iter()
                .any(|hit| hit.fact.name == "a-blocked-handler-stalls"),
            "the chain's conclusion recalls like any project knowledge: {:?}",
            hits.iter()
                .map(|hit| hit.fact.name.clone())
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // --- the practice (the learning plan's C6) --------------------------------

    /// The scripted practice conversation.
    struct ScriptedPractice {
        answers: VecDeque<String>,
        held: VecDeque<bool>,
        posed: Vec<String>,
    }

    impl PracticeConversation for ScriptedPractice {
        fn pose(&mut self, problem: &PracticeProblem) -> String {
            self.posed.push(problem.setting.clone());
            self.answers
                .pop_front()
                .expect("the script answers every problem")
        }

        fn judge(&mut self, _problem: &PracticeProblem, _answer: &str) -> bool {
            self.held
                .pop_front()
                .expect("the script judges every answer")
        }
    }

    fn problem(setting: &str, exercises: &[&str]) -> PracticeProblem {
        PracticeProblem {
            setting: setting.to_string(),
            exercises: exercises.iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn the_practice_poses_one_problem_per_turn_and_never_names_the_knowledge() {
        let problems = vec![
            problem(
                "a server's handler stops mid-request and never comes back; the client is \
                 still waiting",
                &["a-blocked-handler-stalls"],
            ),
            problem(
                "a connection sits open long after its request should have finished",
                &["a-connection-stays-open"],
            ),
        ];
        let mut conversation = ScriptedPractice {
            answers: VecDeque::from(vec![
                "the request is stuck because the handler never returns".to_string(),
                "the connection is held open by the stalled request".to_string(),
            ]),
            held: VecDeque::from(vec![true, false]),
            posed: Vec::new(),
        };
        let root = scratch("practice");
        let memory = Memory::load(&root);
        record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &[],
            "2026-11-08-abc",
        )
        .expect("the proposition lands");
        record_proposition(
            &memory,
            "a-connection-stays-open",
            "the connection stays open while the handler is blocked",
            &[],
            "2026-11-08-abc",
        )
        .expect("the second proposition lands");
        let outcomes = practice(&memory, &problems, &mut conversation).expect("the practice runs");
        // One problem per turn: the settings pose in order, each answered
        // and judged before the next begins.
        assert_eq!(
            conversation.posed,
            vec![
                "a server's handler stops mid-request and never comes back; the client is \
                 still waiting",
                "a connection sits open long after its request should have finished"
            ]
        );
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes[0].held);
        assert!(!outcomes[1].held, "the second answer did not hold");
        assert_eq!(
            outcomes[0].problem.exercises,
            vec!["a-blocked-handler-stalls"],
            "the practitioner's own record names what the problem exercised"
        );
    }

    #[test]
    fn the_practice_refuses_a_setting_that_names_what_it_exercises() {
        let root = scratch("practice-refusal");
        let memory = Memory::load(&root);
        record_proposition(
            &memory,
            "a-blocked-handler-stalls",
            "a blocked handler stalls the whole request",
            &[],
            "2026-11-08-abc",
        )
        .expect("the proposition lands");
        let problems = vec![problem(
            "explain why a blocked handler stalls the whole request",
            &["a-blocked-handler-stalls"],
        )];
        let mut conversation = ScriptedPractice {
            answers: VecDeque::new(),
            held: VecDeque::new(),
            posed: Vec::new(),
        };
        let error = practice(&memory, &problems, &mut conversation).expect_err("a named problem");
        assert!(error.contains("unnamed"), "{error}");
        assert!(
            conversation.posed.is_empty(),
            "a refused problem is never posed"
        );
    }

    #[test]
    fn the_practice_outcomes_land_as_session_evidence() {
        let root = scratch("practice-record");
        let memory = Memory::load(&root);
        let outcomes = vec![PracticeOutcome {
            problem: problem(
                "a server's handler stops mid-request and never comes back",
                &["a-blocked-handler-stalls"],
            ),
            answer: "the request is stuck because the handler never returns".to_string(),
            held: true,
        }];
        record_practice(&memory, "2026-11-08-abc", &outcomes).expect("the practice lands");
        let facts = memory.facts();
        let fact = facts
            .iter()
            .find(|fact| fact.name == "practice-2026-11-08-abc-0")
            .expect("the practice fact");
        assert_eq!(fact.fact_type, "practice");
        assert_eq!(fact.source, "2026-11-08-abc");
        // The exercises ride the chain's field: the practice's outcomes
        // name what they exercised, which is the evidence the feeds read.
        assert_eq!(fact.premises, vec!["a-blocked-handler-stalls"]);
        assert!(fact.body.contains("held: true"));
        assert!(fact
            .body
            .contains("answer: the request is stuck because the handler never returns"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_practitioner_role_seeds_into_the_home() {
        let home = scratch("practitioner-role");
        seed_practitioner(&home).expect("the practitioner seeds");
        let role = crate::roles::load(&home, PRACTITIONER_ROLE).expect("the practitioner loads");
        assert!(
            role.overlay.contains("WITHOUT being named"),
            "{}",
            role.overlay
        );
        assert!(role.overlay.contains("One problem"), "{}", role.overlay);
        let _ = std::fs::remove_dir_all(&home);
    }
}
