//! The run loop's half of the resource contract (the plan's C3): admit before
//! the first turn, measure at the turn boundary, and stop with the numbers in
//! the transcript when the run outgrows its own budget.
//!
//! The split is the same one `footprint.rs` draws, and the reason this is a
//! module of its own rather than code inside the agent loop:
//!
//! - **The arithmetic is here, and it is pure.** [`Work::admitting`],
//!   [`Oversight::measure`] and [`Oversight::apply`] take readings and return
//!   standings. Nothing in this file spawns a lane, reads a clock twice, or
//!   asks a model — so the whole contract is testable without a home, a stream,
//!   or a socket.
//! - **The judgment is not here.** When a reading is abnormal the loop asks the
//!   judge once and hands the answer back in as [`Judgment`]; the judge's own
//!   module owns that call, and the socket guards stay satisfied because this
//!   file never names it.
//! - **The agent loop only wires.** It calls `admitting` before the first turn,
//!   `measure` at the boundary, and acts on [`Standing`]. A run with no folder,
//!   or with the contract off, pays one branch and nothing else.
//!
//! The cadence is deliberate: a full reading is a `statvfs` call plus a `ps`
//! fork, so measuring every turn would spend more than it saves on a chatty
//! run. The plan's own instruction is to measure the cadence rather than assume
//! it, so [`CADENCE`] is one named number and the tests state what it buys.

use std::path::{Path, PathBuf};
use std::time::Instant;

pub use crate::footprint::Env;
use crate::footprint::{self, Admission, Appetite, Reading, Settlement};

/// How many turns between full readings. The plan's guess was "every fifth";
/// the reasoning is that a runaway large enough to matter moves free space by
/// gigabytes, and a five-turn window is minutes of work at most — while the
/// reading itself forks `ps`, which is not free.
pub const CADENCE: u64 = 5;

/// What the run is doing, as the contract needs to know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    /// The class this run files under: `run:<folder>`, or `run` when it has no
    /// folder to name.
    pub class: String,
    /// The folder the run works in, when it has one. Its absence is why a
    /// folderless chat is not admitted against a volume it cannot fill.
    pub folder: Option<PathBuf>,
    /// The budget this run may write, in kibibytes.
    pub budget_kib: u64,
}

impl Work {
    /// The work one run represents, from the folder it works in.
    pub fn for_run(folder: Option<&Path>, budget_kib: u64) -> Work {
        let class = match folder {
            Some(path) => format!("{}:{}", footprint::CLASS_RUN, path.display()),
            None => footprint::CLASS_RUN.to_string(),
        };
        Work {
            class,
            folder: folder.map(Path::to_path_buf),
            budget_kib,
        }
    }

    /// Whether the contract governs this run at all. A run with no folder writes
    /// into no volume fleet chose, so there is nothing to admit it against and
    /// nothing to curtail — the ordinary folderless chat stays untouched.
    pub fn governed(&self) -> bool {
        self.folder.is_some()
    }

    /// The admission for this run against a reading of the machine: the headroom
    /// above the reserve against the run's own budget.
    pub fn admitting(&self, reading: &Reading) -> Admission {
        footprint::admit(reading.available_kib, reading.reserve_kib, self.budget_kib)
    }
}

/// One look at the volume, and when it was taken — the interval is part of the
/// fact, because a reading during a heavy run is a moving target.
#[derive(Debug, Clone)]
pub struct Readout {
    pub available_kib: u64,
    pub volume_kib: u64,
    pub taken: Instant,
}

impl Readout {
    pub fn new(available_kib: u64, volume_kib: u64) -> Readout {
        Readout {
            available_kib,
            volume_kib,
            taken: Instant::now(),
        }
    }

    /// Seconds since this look was taken — recorded with every judgment so a
    /// reader can tell a five-second reading from a five-minute one.
    pub fn age_secs(&self) -> u64 {
        self.taken.elapsed().as_secs()
    }
}

/// What the judge said about an abnormal reading, when one fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgment {
    /// The growth was the work: the run continues and its recorded budget is
    /// raised so the same shape does not ask twice.
    Expected { words: String },
    /// The growth was not the work: the run is curtailed now.
    Curtailed { words: String },
    /// The lane was asked and did not answer. Read as `expected` for this
    /// interval and recorded as such — never a silent stop on a lane hiccup,
    /// which is the plan's own rule.
    Unavailable { reason: String },
}

impl Judgment {
    /// The verdict word this judgment lands on the calibration row.
    pub fn verdict(&self) -> &'static str {
        match self {
            Judgment::Curtailed { .. } => crate::calib::FOOTPRINT_RUNAWAY,
            Judgment::Expected { .. } | Judgment::Unavailable { .. } => {
                crate::calib::FOOTPRINT_EXPECTED
            }
        }
    }

    /// How the verdict was reached, for the row.
    pub fn decided_by(&self) -> &'static str {
        match self {
            Judgment::Unavailable { .. } => "judge-unavailable",
            _ => "judged",
        }
    }

    /// The judge's own words (or the lane's reason), for the row.
    pub fn words(&self) -> &str {
        match self {
            Judgment::Expected { words } | Judgment::Curtailed { words } => words,
            Judgment::Unavailable { reason } => reason,
        }
    }
}

/// What the resource judge is shown, when an overage asks it: the numbers it
/// must reason about, and the work the run was doing when it grew.
///
/// The shape lives here rather than in the beat's module so the agent can carry
/// an ask to an injected capability without naming the beat — and so the number
/// of fields the judge sees is one decision in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverageAsk {
    /// The work class, as the contract files it.
    pub class: String,
    /// What the run has settled since it was admitted, in kibibytes.
    pub settled_kib: u64,
    /// The budget it was admitted against, in kibibytes.
    pub budget_kib: u64,
    /// Free space at this reading, and the reserve it is measured against.
    pub available_kib: u64,
    pub reserve_kib: u64,
    /// The run's recent tool calls, as `name: the first argument's first line`.
    /// The evidence that separates "writing a checkpoint tree" from "stuck in a
    /// loop", and the only context the ask carries beyond the numbers.
    pub recent_calls: Vec<String>,
}

/// Where one measurement landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// Inside the budget: the run carries on.
    Inside,
    /// Past the budget and not yet judged: the caller asks sys1 once.
    Overage { settled_kib: u64, budget_kib: u64 },
    /// The reserve was breached outright — the disk is nearly full and no
    /// judgment is worth the wait. Deterministic, and it stops the run.
    Encroached {
        available_kib: u64,
        reserve_kib: u64,
    },
}

/// The run's live oversight: the look it was admitted against, the baseline
/// every later measurement is compared to, and the turns since the last reading.
#[derive(Debug, Clone)]
pub struct Oversight {
    /// The run this state belongs to.
    pub work: Work,
    /// The look at admission: the baseline the slide is measured from.
    pub baseline: Readout,
    /// The reserve in force for this run, and which input produced it.
    pub reserve_kib: u64,
    pub reserve_source: &'static str,
    /// How tight the machine is, carried so a stop reads as a decision.
    pub appetite: Appetite,
    /// The run's budget, which a judged `expected` may raise.
    pub budget_kib: u64,
    /// How many turns since the last reading.
    since_reading: u64,
    /// How many times this run has asked the judge.
    pub asks: u64,
    /// The floor this run was granted by a yes at a breach, when one was
    /// given. `Some` means the owner said continue: the run may write down to
    /// that floor, and it is asked again only when a reading falls below it —
    /// so a grant buys one more run's budget of room, not a licence on the
    /// disk. The number is `reserve − budget`, which derives the grant from a
    /// figure the owner already settled rather than inventing a second one.
    pub past_reserve: Option<u64>,
}

impl Oversight {
    /// Start the contract for one run from the reading that admitted it.
    pub fn start(work: Work, reading: &Reading, available_kib: u64, volume_kib: u64) -> Oversight {
        Oversight {
            budget_kib: work.budget_kib,
            work,
            baseline: Readout::new(available_kib, volume_kib),
            reserve_kib: reading.reserve_kib,
            reserve_source: reading.reserve_source,
            appetite: reading.appetite,
            since_reading: 0,
            asks: 0,
            past_reserve: None,
        }
    }

    /// Whether this turn is a measuring turn. The first turn after admission is
    /// not (the baseline was just taken), and every [`CADENCE`]th is.
    pub fn due(&mut self) -> bool {
        self.since_reading += 1;
        if self.since_reading < CADENCE {
            return false;
        }
        self.since_reading = 0;
        true
    }

    /// Grade one reading. The reserve breach is checked first and always: a
    /// machine at its floor has no room for a judgment to be wrong about.
    pub fn measure(&self, now: &Readout) -> Standing {
        // The floor this run is held to: the declared reserve, lowered to the
        // one the owner granted by a yes at a breach. A run permitted past the
        // reserve is not asked again until it reaches the floor it was
        // granted — otherwise every reading past the line would prompt, which
        // is a nag rather than a pause.
        let floor = self.past_reserve.unwrap_or(self.reserve_kib);
        if now.available_kib < floor {
            return Standing::Encroached {
                available_kib: now.available_kib,
                reserve_kib: floor,
            };
        }
        let settlement: Settlement = footprint::settle(
            self.baseline.available_kib,
            now.available_kib,
            self.budget_kib,
        );
        if settlement.runaway {
            return Standing::Overage {
                settled_kib: settlement.delta_kib,
                budget_kib: self.budget_kib,
            };
        }
        Standing::Inside
    }

    /// Record the owner's yes at a breach: this run may keep working for one
    /// more of its own budgets below the reserve, and it is asked again after
    /// that. A no is not recorded — the caller stops — and the grant does not
    /// compound: a second yes moves the floor by another budget, and no further,
    /// so a run cannot talk its way to the edge of the disk one pause at a time.
    pub fn granted_past_reserve(&mut self) {
        let floor = self.past_reserve.unwrap_or(self.reserve_kib);
        self.past_reserve = Some(floor.saturating_sub(self.budget_kib));
    }

    /// Apply the judge's answer. A curtailment leaves the ending to the caller
    /// (the run stops); an `Expected` raises the recorded budget to what the run
    /// has actually taken, so the identical slide does not ask again.
    pub fn apply(&mut self, judgment: &Judgment, settled_kib: u64) {
        self.asks += 1;
        if matches!(
            judgment,
            Judgment::Expected { .. } | Judgment::Unavailable { .. }
        ) {
            // The raise is the measured slide plus a quarter of the budget as
            // slack: enough that the next interval is silent, not so much that a
            // genuinely runaway run is waved through on one favorable answer.
            let raised = settled_kib.saturating_add(self.budget_kib / 4);
            if raised > self.budget_kib {
                self.budget_kib = raised;
            }
        }
    }

    /// The clause the transcript carries when the run stops, in the words the
    /// owner reads: the numbers, the reserve, and what produced it.
    pub fn stop_clause(&self, now: &Readout, settled_kib: u64, why: &str) -> String {
        format!(
            "the resource contract stopped this run {why}: {:.2} GiB settled against a {:.2} GiB budget, with {:.2} GiB free against a {:.2} GiB reserve ({}, {})",
            settled_kib as f64 / 1024.0 / 1024.0,
            self.budget_kib as f64 / 1024.0 / 1024.0,
            now.available_kib as f64 / 1024.0 / 1024.0,
            self.reserve_kib as f64 / 1024.0 / 1024.0,
            self.appetite.word(),
            self.reserve_source,
        )
    }
}

/// One look at the volume that holds `folder`, in kibibytes, from one `statvfs`
/// — the same call the `footprint` verb makes, so a run and the report it prints
/// cannot disagree.
pub fn reading_of(folder: &Path) -> Option<Readout> {
    let (free, total) = volume_facts(folder)?;
    Some(Readout::new(free, total))
}

#[cfg(unix)]
fn volume_facts(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: `path` is a valid NUL-terminated name and `stats` a valid
    // out-pointer; statvfs writes only into the struct it is given.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let available = (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64) / 1024;
    let total = (stats.f_blocks as u64).saturating_mul(stats.f_frsize as u64) / 1024;
    Some((available, total))
}

#[cfg(not(unix))]
fn volume_facts(_path: &Path) -> Option<(u64, u64)> {
    None
}

/// The environment's history, read from `<home>/footprint-history.jsonl` the way
/// the report reads it: one record per `scripts/footprint.sh --record`.
pub fn env_from_history(home: &Path) -> Env {
    let history = std::fs::read_to_string(home.join("footprint-history.jsonl")).unwrap_or_default();
    let mut env = Env::default();
    for line in history.lines() {
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let volume = doc.get("volume_kib").and_then(|v| v.as_u64()).unwrap_or(0);
        let free = doc.get("free_kib").and_then(|v| v.as_u64()).unwrap_or(0);
        if volume == 0 {
            continue;
        }
        env.free_fractions.push(free as f64 / volume as f64);
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIB: u64 = 1024;
    const GIB: u64 = 1024 * 1024;

    fn reading(available_gib: u64, reserve_gib: u64) -> Reading {
        Reading {
            lines: Vec::new(),
            volume_kib: 460 * GIB,
            available_kib: available_gib * GIB,
            reserve_kib: reserve_gib * GIB,
            reserve_source: "inferred",
            appetite: Appetite::Roomy,
            standing_free: Some(0.25),
            fill_gib_per_day: 0.0,
            records: 1,
        }
    }

    fn live(available_gib: u64, reserve_gib: u64, budget_gib: u64) -> Oversight {
        let work = Work::for_run(Some(Path::new("/tmp/scratch")), budget_gib * GIB);
        Oversight::start(
            work,
            &reading(available_gib, reserve_gib),
            available_gib * GIB,
            460 * GIB,
        )
    }

    #[test]
    fn a_run_without_a_folder_is_not_governed() {
        let bare = Work::for_run(None, 2 * GIB);
        assert!(!bare.governed());
        assert_eq!(bare.class, footprint::CLASS_RUN);
        let funded = Work::for_run(Some(Path::new("/tmp/scratch")), 2 * GIB);
        assert!(funded.governed());
        assert_eq!(funded.class, "run:/tmp/scratch");
    }

    #[test]
    fn the_run_is_admitted_against_its_own_budget_above_the_reserve() {
        let work = Work::for_run(Some(Path::new("/tmp/scratch")), 2 * GIB);
        assert!(work.admitting(&reading(116, 18)).ok);
        let admission = work.admitting(&reading(19, 18));
        assert!(!admission.ok, "{admission:?}");
        assert_eq!(admission.headroom_kib, GIB);
        assert_eq!(admission.needed_kib, 2 * GIB);
    }

    #[test]
    fn the_cadence_measures_on_the_fifth_turn_and_not_before() {
        let mut state = live(116, 18, 2);
        assert_eq!(
            (1..CADENCE).filter(|_| state.due()).count(),
            0,
            "no reading before the cadence"
        );
        assert!(state.due(), "the cadence's own turn reads");
        assert_eq!(
            (1..CADENCE).filter(|_| state.due()).count(),
            0,
            "the counter reset: the next run of turns starts over"
        );
        let mut state = live(116, 18, 2);
        let reads = (1..=20).filter(|_| state.due()).count();
        assert_eq!(reads, 4, "20 turns is four readings at a cadence of five");
    }

    #[test]
    fn a_slide_inside_the_budget_is_inside_and_past_it_is_an_overage() {
        let state = live(116, 18, 2);
        assert_eq!(
            state.measure(&Readout::new(115 * GIB, 460 * GIB)),
            Standing::Inside
        );
        assert_eq!(
            state.measure(&Readout::new(114 * GIB, 460 * GIB)),
            Standing::Inside,
            "exactly the budget is inside: settle is inclusive"
        );
        let standing = state.measure(&Readout::new(114 * GIB - KIB, 460 * GIB));
        assert_eq!(
            standing,
            Standing::Overage {
                settled_kib: 2 * GIB + KIB,
                budget_kib: 2 * GIB
            }
        );
    }

    #[test]
    fn a_breached_reserve_stops_the_run_without_asking_anyone() {
        let state = live(116, 18, 2);
        let standing = state.measure(&Readout::new(17 * GIB, 460 * GIB));
        assert_eq!(
            standing,
            Standing::Encroached {
                available_kib: 17 * GIB,
                reserve_kib: 18 * GIB
            }
        );
        let hard = state.measure(&Readout::new(10 * GIB, 460 * GIB));
        assert!(matches!(hard, Standing::Encroached { .. }), "{hard:?}");
    }

    #[test]
    fn an_expected_judgment_raises_the_budget_and_a_curtailment_does_not() {
        let mut state = live(116, 18, 2);
        state.apply(
            &Judgment::Expected {
                words: "the run is writing a checkpoint tree".to_string(),
            },
            4 * GIB,
        );
        assert!(state.budget_kib > 4 * GIB, "budget {:?}", state.budget_kib);
        assert_eq!(state.asks, 1);
        assert_eq!(
            state.measure(&Readout::new(112 * GIB, 460 * GIB)),
            Standing::Inside
        );
        let mut state = live(116, 18, 2);
        state.apply(
            &Judgment::Unavailable {
                reason: "the lane timed out".to_string(),
            },
            3 * GIB,
        );
        assert!(state.budget_kib >= 3 * GIB);
        let mut state = live(116, 18, 2);
        state.apply(
            &Judgment::Curtailed {
                words: "nothing in its tool calls writes".to_string(),
            },
            9 * GIB,
        );
        assert_eq!(state.budget_kib, 2 * GIB);
    }

    #[test]
    fn a_breach_pauses_and_a_yes_buys_one_more_budget_of_room() {
        // The owner's 2026-11-10 ruling: a deterministic breach is a pause, not
        // a stop. The breach itself is unchanged — arithmetic still decides it.
        let mut state = live(116, 18, 2);
        let breach = state.measure(&Readout::new(17 * GIB, 460 * GIB));
        assert_eq!(
            breach,
            Standing::Encroached {
                available_kib: 17 * GIB,
                reserve_kib: 18 * GIB
            }
        );
        // The owner says continue: the run may work one more of its own budgets
        // below the reserve — 18 - 2 = 16 GiB — and is asked again after that.
        state.granted_past_reserve();
        assert_eq!(state.past_reserve, Some(16 * GIB));
        // The grant lowers the FLOOR; it does not excuse the budget. At 17 GiB
        // the reserve no longer trips, but the run has settled 99 GiB against a
        // 2 GiB budget, so the overage arm is what speaks — an ask, not a pause.
        // The point of the grant is that the BREACH prompt does not repeat on
        // every reading; it is not a blanket permission.
        assert!(
            matches!(
                state.measure(&Readout::new(17 * GIB, 460 * GIB)),
                Standing::Overage { .. }
            ),
            "above the granted floor the breach arm stays quiet; the budget still grades"
        );
        // Below the granted floor, the pause comes back — with the GRANTED
        // floor named, so the prompt describes the line in force.
        let again = state.measure(&Readout::new(15 * GIB, 460 * GIB));
        assert_eq!(
            again,
            Standing::Encroached {
                available_kib: 15 * GIB,
                reserve_kib: 16 * GIB
            }
        );
        // A second yes moves it by another budget and no further: the grant
        // never compounds to a licence on the disk.
        state.granted_past_reserve();
        assert_eq!(state.past_reserve, Some(14 * GIB));
        // And a run that never breached has no grant at all.
        assert_eq!(live(116, 18, 2).past_reserve, None);
    }

    #[test]
    fn the_stop_clause_names_the_numbers_and_where_the_reserve_came_from() {
        let state = live(116, 18, 2);
        let clause = state.stop_clause(
            &Readout::new(113 * GIB, 460 * GIB),
            3 * GIB,
            "after the judge curtailed it",
        );
        assert!(clause.contains("3.00 GiB settled"), "{clause}");
        assert!(clause.contains("2.00 GiB budget"), "{clause}");
        assert!(clause.contains("113.00 GiB free"), "{clause}");
        assert!(clause.contains("18.00 GiB reserve"), "{clause}");
        assert!(clause.contains("roomy"), "{clause}");
        assert!(clause.contains("inferred"), "{clause}");
        assert!(clause.contains("after the judge curtailed it"), "{clause}");
    }

    #[test]
    fn a_curtailed_verdict_is_reached_end_to_end_through_the_reading_and_the_ask() {
        // C6's decision path, without a model: a run whose writes outgrow its
        // budget is measured, found in overage, asked once, and — when the
        // answer is a curtailment — left with the clause that ends it and the
        // numbers in it.
        let mut state = live(116, 18, 2);
        // Advance to a measuring turn: the cadence reads on the fifth.
        for _ in 1..CADENCE {
            assert!(!state.due(), "no reading before the cadence");
        }
        assert!(state.due(), "the cadence's own turn is a measuring turn");
        // The run wrote 4 GiB: past its 2 GiB budget, above the reserve.
        let now = Readout::new(112 * GIB, 460 * GIB);
        let standing = state.measure(&now);
        let Standing::Overage {
            settled_kib,
            budget_kib,
        } = standing
        else {
            panic!("expected an overage, got {standing:?}");
        };
        assert_eq!(settled_kib, 4 * GIB);
        assert_eq!(budget_kib, 2 * GIB);
        // The judge's answer, applied.
        let judgment = Judgment::Curtailed {
            words: "nothing in its tool calls writes four gigabytes".to_string(),
        };
        state.apply(&judgment, settled_kib);
        // The curtailment raised nothing, and the clause names the numbers.
        assert_eq!(state.budget_kib, 2 * GIB);
        let clause = state.stop_clause(&now, settled_kib, "after the judge curtailed it");
        assert!(clause.contains("4.00 GiB settled"), "{clause}");
        assert!(clause.contains("112.00 GiB free"), "{clause}");
        assert!(clause.contains("18.00 GiB reserve"), "{clause}");
        // The row this ending lands carries the judge's own words and the
        // `judged` provenance — which is what C9 later grades.
        assert_eq!(judgment.verdict(), crate::calib::FOOTPRINT_RUNAWAY);
        assert_eq!(judgment.decided_by(), "judged");
        assert!(judgment.words().contains("four gigabytes"));
    }

    #[test]
    fn the_contract_asks_the_judge_once_per_reading_and_never_per_turn() {
        // A run with a hundred turns and no overage asks nothing: the ask is
        // tied to an abnormal reading, not to the loop.
        let mut state = live(116, 18, 2);
        for turn in 1..=100 {
            if state.due() {
                // Every measuring turn reads a clean slide.
                assert_eq!(
                    state.measure(&Readout::new((115 * GIB) - (turn * KIB), 460 * GIB)),
                    Standing::Inside
                );
            }
        }
        assert_eq!(state.asks, 0, "a clean run never asks");
        // A run that reads abnormal on three separate readings asks three
        // times — once each, which is the bound the plan names.
        let mut state = live(116, 18, 2);
        for _ in 0..3 {
            state.apply(
                &Judgment::Expected {
                    words: "the work".to_string(),
                },
                5 * GIB,
            );
        }
        assert_eq!(state.asks, 3);
    }

    #[test]
    fn the_answer_that_raises_the_budget_does_not_raise_it_twice_for_one_shape() {
        // One `expected` on a 5 GiB slide raises the budget past the slide, so
        // the identical slide on the next reading reads clean — the plan's
        // "the second identical shape does not pay twice".
        let mut state = live(116, 18, 2);
        let settled = 5 * GIB;
        state.apply(
            &Judgment::Expected {
                words: "checkpointing a large tree".to_string(),
            },
            settled,
        );
        let raised = state.budget_kib;
        assert!(raised > settled, "raised {raised} vs settled {settled}");
        // The same slide again, measured against the raised budget: clean.
        assert_eq!(
            state.measure(&Readout::new(116 * GIB - raised, 460 * GIB)),
            Standing::Inside,
            "the same slide must not ask twice"
        );
        // And one byte past the raised budget is an overage again, so the
        // raise bounds the shape rather than silencing the contract.
        assert!(matches!(
            state.measure(&Readout::new(116 * GIB - raised - KIB, 460 * GIB)),
            Standing::Overage { .. }
        ));
    }

    #[test]
    fn a_history_file_becomes_the_environment_the_reserve_is_read_from() {
        let dir = std::env::temp_dir().join(format!("contract-env-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("footprint-history.jsonl"),
            "{\"at\":\"2026-11-01T00:00:00Z\",\"volume_kib\":1000,\"free_kib\":90}\n\
             {\"at\":\"2026-11-02T00:00:00Z\",\"volume_kib\":1000,\"free_kib\":80}\n\
             not a record\n\
             {\"volume_kib\":0,\"free_kib\":5}\n",
        )
        .unwrap();
        let env = env_from_history(&dir);
        assert_eq!(env.free_fractions.len(), 2);
        assert!((env.free_fractions[0] - 0.09).abs() < 1e-9);
        assert_eq!(footprint::appetite(&env), Appetite::Tight);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
