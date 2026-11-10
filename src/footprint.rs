//! Resource awareness: what fleet is holding right now, and whether that is
//! responsible.
//!
//! The owner's ruling (2026-11-09): fleet must know what it is using at all
//! times — disk, memory, the processes it spawned — and decide whether that
//! is responsible. The split is deliberate:
//!
//! - **Deterministic for the bounds.** A store over its budget, a process
//!   tree over its memory ceiling, more children than the cap: these are
//!   facts with a declared line, and arithmetic decides them. A model adds
//!   nothing to "17 GiB against a 20 GiB budget".
//! - **A judgment for the ambiguous.** "Is this run using 3 GiB because it is
//!   working or because it is stuck?" has no arithmetic answer; that is a
//!   sys1 call, made where the run already asks for judgments, never in this
//!   module — the code here only gathers and grades the numbers.
//!
//! Everything is a pure function over facts so the grader runs hermetically.

/// The memory a process tree may hold, in kibibytes (2 GiB).
pub const MEMORY_CEILING_KIB: u64 = 2 * 1024 * 1024;
/// How many live processes the tree may carry: the face, a run, its lanes,
/// and the shells a run spawns. Four times the cap is the runaway's shape.
pub const PROCESS_CEILING: usize = 80;

/// One process as `ps` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    /// Resident set size in kibibytes.
    pub rss_kib: u64,
}

/// Parse `ps -o pid=,ppid=,rss=` output. Blank and malformed lines are
/// skipped rather than guessed at: a reading that cannot be read is not a
/// reading of zero.
pub fn parse_ps(text: &str) -> Vec<Proc> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            let rss_kib = fields.next()?.parse().ok()?;
            Some(Proc { pid, ppid, rss_kib })
        })
        .collect()
}

/// The process tree rooted at `root`: the root and every descendant.
pub fn tree_of(root: u32, procs: &[Proc]) -> Vec<Proc> {
    let mut chosen: Vec<Proc> = procs.iter().filter(|p| p.pid == root).cloned().collect();
    let mut frontier = vec![root];
    while let Some(parent) = frontier.pop() {
        for proc in procs.iter().filter(|p| p.ppid == parent) {
            if !chosen.iter().any(|c| c.pid == proc.pid) {
                chosen.push(proc.clone());
                frontier.push(proc.pid);
            }
        }
    }
    chosen
}

/// The tree's total resident memory, in kibibytes.
pub fn memory_kib(tree: &[Proc]) -> u64 {
    tree.iter().map(|p| p.rss_kib).sum()
}

/// One resource's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The resource, e.g. `memory`, `processes`, `disk`.
    pub name: &'static str,
    /// What is held, in the resource's own unit.
    pub used: u64,
    /// What it may hold.
    pub budget: u64,
    /// The unit, for the report.
    pub unit: &'static str,
    /// A detail the numbers do not carry (a named over-budget store).
    pub note: String,
}

impl Line {
    pub fn ok(&self) -> bool {
        self.used <= self.budget
    }
}

/// The process-tree lines: memory and process count, both deterministic.
pub fn evaluate_tree(tree: &[Proc]) -> Vec<Line> {
    let mut lines = vec![
        Line {
            name: "memory",
            used: memory_kib(tree) / 1024,
            budget: MEMORY_CEILING_KIB / 1024,
            unit: "MiB",
            note: format!("{} process(es) in the tree", tree.len()),
        },
        Line {
            name: "processes",
            used: tree.len() as u64,
            budget: PROCESS_CEILING as u64,
            unit: "",
            note: String::new(),
        },
    ];
    // A subtree that is most of the memory is worth naming, not counting.
    let mut heaviest = tree.iter().max_by_key(|p| p.rss_kib);
    if let Some(top) = heaviest.take() {
        if top.rss_kib * 2 > memory_kib(tree) && tree.len() > 1 {
            lines[0].note = format!(
                "{} of {} in pid {}",
                top.rss_kib / 1024,
                memory_kib(tree) / 1024,
                top.pid
            );
        }
    }
    lines
}

/// Render one line the way the doctor does: the verdict, then the numbers.
pub fn render(line: &Line) -> String {
    let verdict = if line.ok() { "ok  " } else { "FAIL" };
    let sep = if line.unit.is_empty() { "" } else { " " };
    let detail = if line.note.is_empty() {
        format!(
            "{}{sep}{} of {}{sep}{}",
            line.used, line.unit, line.budget, line.unit
        )
    } else {
        format!(
            "{}{sep}{} of {}{sep}{} ({})",
            line.used, line.unit, line.budget, line.unit, line.note
        )
    };
    format!("{:12} {}  {}", line.name, verdict, detail)
}

// ---- the contract: admit, settle, and what the environment has shown -------

/// What the environment's history says about its margin (the owner's ruling,
/// 2026-11-09: the appetite is inferable from the environment itself).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Env {
    /// One sample per day: the free share of the volume, newest last.
    pub free_fractions: Vec<f64>,
    /// How fast free space falls, GiB per day (0 when it is not filling).
    pub fill_gib_per_day: f64,
}

impl Env {
    /// The standing free share: the median, not the latest reading — one good
    /// day after a cleanup must not read as a roomy machine.
    pub fn standing_free(&self) -> Option<f64> {
        if self.free_fractions.is_empty() {
            return None;
        }
        let mut sorted = self.free_fractions.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        Some(if sorted.len().is_multiple_of(2) {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        })
    }
}

/// How tight the machine is, and therefore how tight fleet must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appetite {
    /// Habitually under ~10% free: the owner tolerates risk, which means there
    /// is no margin to absorb a runaway. Fleet is tightest here.
    Tight,
    Balanced,
    Roomy,
}

impl Appetite {
    /// The reserve as a share of the volume. Fractions, never absolutes: a
    /// smaller disk must not wave through the same number of gigabytes.
    pub fn reserve_share(self) -> f64 {
        match self {
            Appetite::Tight => 0.12,
            Appetite::Balanced => 0.08,
            Appetite::Roomy => 0.04,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Appetite::Tight => "tight",
            Appetite::Balanced => "balanced",
            Appetite::Roomy => "roomy",
        }
    }
}

/// The appetite from the environment's standing free share. A habitually full
/// machine tightens fleet — it never relaxes it: the tolerance is evidence of
/// no margin, not a budget grant.
pub fn appetite(env: &Env) -> Appetite {
    match env.standing_free() {
        Some(share) if share < 0.10 => Appetite::Tight,
        Some(share) if share < 0.25 => Appetite::Balanced,
        Some(_) => Appetite::Roomy,
        // No history is not room: a fresh machine starts balanced.
        None => Appetite::Balanced,
    }
}

/// The reserve in kibibytes, and which of the two inputs produced it: the
/// owner's declared appetite wins where it exists, the inferred one otherwise.
pub fn reserve_kib(total_kib: u64, env: &Env, declared: Option<Appetite>) -> (u64, &'static str) {
    let (appetite, source) = match declared {
        Some(appetite) => (appetite, "declared"),
        None => (appetite(env), "inferred"),
    };
    ((total_kib as f64 * appetite.reserve_share()) as u64, source)
}

/// One unit of work's admission: the headroom it needs against what it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub ok: bool,
    pub headroom_kib: u64,
    pub needed_kib: u64,
}

/// Admit work only against the space strictly above the reserve. Deterministic:
/// "3 GiB of room, 8 GiB of work" needs no judgment.
///
/// The comparison is strict on purpose, and it is the contract's one boundary
/// that reads backwards at first glance: work whose estimate exactly equals the
/// headroom is **refused**. Work that costs exactly what is spare consumes the
/// reserve down to its floor and leaves nothing for the next unit, which is the
/// pattern the owner rejected ("using resources until near exhaustion"). A
/// settlement exactly at its own budget is the mirror: not a runaway, because
/// the budget was the estimate plus the slack it declared. One seam admits with
/// margin to spare; the other settles without inventing a breach.
pub fn admit(available_kib: u64, reserve_kib: u64, estimate_kib: u64) -> Admission {
    let headroom = available_kib.saturating_sub(reserve_kib);
    Admission {
        ok: headroom > estimate_kib,
        headroom_kib: headroom,
        needed_kib: estimate_kib,
    }
}

/// What the work actually took, against the budget it declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub delta_kib: u64,
    /// Over the declared budget: the runaway arm.
    pub runaway: bool,
}

/// Work that grew past its own declared budget is a runaway; the caller stops
/// it, keeps the diagnosis window, then cleans what it made.
pub fn settle(before_kib: u64, after_kib: u64, budget_kib: u64) -> Settlement {
    let delta = before_kib.saturating_sub(after_kib);
    Settlement {
        delta_kib: delta,
        runaway: delta > budget_kib,
    }
}

// ---- the work classes, and what each one is admitted against --------------

/// A release gate: the four-leg run in its own lane (`scripts/release.sh tag`).
/// A cold gate lane is a full 1050-crate build, so the prior is the measured
/// shape of one, not a guess at the app's size.
pub const CLASS_RELEASE_GATE: &str = "release-gate";
/// A release assets cut: the two builds and the bundle (`release.sh assets`).
pub const CLASS_RELEASE_ASSETS: &str = "release-assets";
/// A lane bootstrap: one vendored Python service's venv (`fleet install --lanes`).
pub const CLASS_LANE_BOOTSTRAP: &str = "lane-bootstrap";
/// A run working in a granted folder, named `run:<folder>` on its row.
pub const CLASS_RUN: &str = "run";

/// The declared estimate for a work class, before any of its own settlements
/// exist — the prior C8 replaces once three settles have landed. These are
/// **proposals** until the owner confirms them (the plan's own warning): they
/// live here and nowhere else, so confirming them is one edit.
pub const ESTIMATE_RELEASE_GATE_KIB: u64 = 8 * 1024 * 1024;
pub const ESTIMATE_RELEASE_ASSETS_KIB: u64 = 8 * 1024 * 1024;
pub const ESTIMATE_LANE_BOOTSTRAP_KIB: u64 = 2 * 1024 * 1024;
/// A run's own budget: what one conversation may write before it is abnormal.
/// A run's write volume is unknowable in advance (the plan puts predicting it
/// out of scope), so this is the bound its per-turn measurement grades against.
pub const RUN_BUDGET_KIB: u64 = 2 * 1024 * 1024;

/// The prior estimate for a work class, in kibibytes. `None` for a class with
/// no declarable estimate — a plain run, whose cost is its own budget.
pub fn estimate_kib(class: &str) -> Option<u64> {
    match class {
        CLASS_RELEASE_GATE => Some(ESTIMATE_RELEASE_GATE_KIB),
        CLASS_RELEASE_ASSETS => Some(ESTIMATE_RELEASE_ASSETS_KIB),
        CLASS_LANE_BOOTSTRAP => Some(ESTIMATE_LANE_BOOTSTRAP_KIB),
        _ => None,
    }
}

// ---- the whole reading: the facts one process tree plus the volume give ---

/// Everything the contract derives from the machine in one pass: the live
/// process tree graded against its ceilings, the volume's free space, the
/// reserve the history implies, and the appetite that produced it.
///
/// This is the shape `fleet footprint --json` publishes and the shape every
/// caller of [`admit`] reads, so the release path and the run loop cannot
/// disagree about what "the machine has room" means.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// The process-tree lines, memory and processes, already graded.
    pub lines: Vec<Line>,
    /// The volume's size, in kibibytes.
    pub volume_kib: u64,
    /// The volume's free space at this reading, in kibibytes.
    pub available_kib: u64,
    /// The floor work may never eat into, in kibibytes.
    pub reserve_kib: u64,
    /// `declared` when the owner's setting produced the reserve, `inferred`
    /// when the environment's history did.
    pub reserve_source: &'static str,
    /// How tight the machine is.
    pub appetite: Appetite,
    /// The standing free share the appetite was read from, when there was
    /// history to read.
    pub standing_free: Option<f64>,
    /// GiB per day the volume is filling, from the same history.
    pub fill_gib_per_day: f64,
    /// How many history records the inference saw.
    pub records: usize,
}

impl Reading {
    /// Build the reading from one measured volume and one history.
    pub fn new(
        lines: Vec<Line>,
        volume_kib: u64,
        available_kib: u64,
        env: &Env,
        declared: Option<Appetite>,
    ) -> Reading {
        let (reserve_kib, reserve_source) = reserve_kib(volume_kib, env, declared);
        Reading {
            lines,
            volume_kib,
            available_kib,
            reserve_kib,
            reserve_source,
            appetite: appetite(env),
            standing_free: env.standing_free(),
            fill_gib_per_day: env.fill_gib_per_day,
            records: env.free_fractions.len(),
        }
    }

    /// Whether every deterministic line is inside its ceiling.
    pub fn ok(&self) -> bool {
        self.lines.iter().all(Line::ok)
    }

    /// The admission one unit of this work class gets against this reading.
    pub fn admission(&self, class: &str) -> Admission {
        admit(
            self.available_kib,
            self.reserve_kib,
            estimate_kib(class).unwrap_or(RUN_BUDGET_KIB),
        )
    }

    /// The reading as one JSON document — the release scripts and the face read
    /// this, never the human table.
    pub fn to_json(&self, class: Option<&str>) -> serde_json::Value {
        let mut doc = serde_json::json!({
            "measured": crate::calib::stamp(),
            "ok": self.ok(),
            "volume_kib": self.volume_kib,
            "available_kib": self.available_kib,
            "reserve_kib": self.reserve_kib,
            "reserve_source": self.reserve_source,
            "appetite": self.appetite.word(),
            "standing_free": self.standing_free,
            "fill_gib_per_day": self.fill_gib_per_day,
            "records": self.records,
            "memory_kib": self.lines.first().map(|line| line.used).unwrap_or(0),
            "processes": self.lines.get(1).map(|line| line.used).unwrap_or(0),
            "lines": self
                .lines
                .iter()
                .map(|line| serde_json::json!({
                    "name": line.name,
                    "used": line.used,
                    "budget": line.budget,
                    "unit": line.unit,
                    "ok": line.ok(),
                }))
                .collect::<Vec<_>>(),
        });
        if let Some(class) = class {
            let admission = self.admission(class);
            doc["work_class"] = serde_json::Value::String(class.to_string());
            doc["estimate_kib"] = serde_json::Value::from(admission.needed_kib);
            doc["headroom_kib"] = serde_json::Value::from(admission.headroom_kib);
            doc["admitted"] = serde_json::Value::Bool(admission.ok);
        }
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: &str = "  100     1 20480
  101   100 10240
  102   101  5120
  103     1  4096
";

    #[test]
    fn a_ps_reading_is_parsed_and_garbage_skipped() {
        let procs = parse_ps(&format!("{PS}not a row\n   \n  200 x 3\n"));
        assert_eq!(procs.len(), 4, "{procs:?}");
        assert_eq!(
            procs[0],
            Proc {
                pid: 100,
                ppid: 1,
                rss_kib: 20480
            }
        );
    }

    #[test]
    fn the_tree_is_the_root_and_its_descendants_only() {
        let procs = parse_ps(PS);
        let tree = tree_of(100, &procs);
        let pids: Vec<u32> = tree.iter().map(|p| p.pid).collect();
        assert_eq!(pids, vec![100, 101, 102], "a sibling is not ours");
        assert_eq!(memory_kib(&tree), 20480 + 10240 + 5120);
    }

    #[test]
    fn the_tree_grades_against_the_declared_ceilings() {
        // A small tree is inside both caps and says how many it carries.
        let small = evaluate_tree(&parse_ps(PS)[..1]);
        assert!(small.iter().all(|line| line.ok()), "{small:?}");
        assert!(small[0].note.contains("1 process"), "{:?}", small[0]);
        // The memory line names where the weight is when one pid holds it.
        let heavy = evaluate_tree(&parse_ps("  1  0 3000000\n  2  1 100000\n")[..2]);
        assert!(!heavy[0].ok());
        assert!(render(&heavy[0]).contains("FAIL"), "{}", render(&heavy[0]));
        assert!(heavy[0].note.contains("in pid 1"), "{:?}", heavy[0]);
        // Too many children is its own failure even at no memory.
        let mut many = String::new();
        for pid in 1..=PROCESS_CEILING + 5 {
            many.push_str(&format!("{pid} {} 1024\n", if pid == 1 { 0 } else { 1 }));
        }
        let crowded = evaluate_tree(&parse_ps(&many));
        let procs = crowded.iter().find(|l| l.name == "processes").unwrap();
        assert!(!procs.ok());
        assert!(
            render(procs).contains(&format!("of {PROCESS_CEILING}")),
            "{}",
            render(procs)
        );
    }

    #[test]
    fn the_reserve_never_learns_downward_and_fractions_scale() {
        // A habitually full machine: 9% free, day in and day out.
        let tightish = Env {
            free_fractions: vec![0.07, 0.09, 0.11, 0.09, 0.08],
            fill_gib_per_day: 1.2,
        };
        assert_eq!(appetite(&tightish), Appetite::Tight);
        // The fraction, not the absolute: the same share on a smaller disk.
        let (small, _) = reserve_kib(100 * 1024 * 1024, &tightish, None);
        let (big, _) = reserve_kib(1000 * 1024 * 1024, &tightish, None);
        assert_eq!(small, (100.0 * 1024.0 * 1024.0 * 0.12) as u64);
        assert!(big > small * 9);
        // The owner's word wins, and says so.
        let (declared, source) = reserve_kib(100 * 1024 * 1024, &tightish, Some(Appetite::Roomy));
        assert_eq!(source, "declared");
        assert!(declared < small);
        // No history is not room.
        assert_eq!(appetite(&Env::default()), Appetite::Balanced);
    }

    #[test]
    fn work_is_admitted_only_above_the_reserve() {
        // 30 GiB free, 25 GiB reserve, 8 GiB of work: refused, both numbers said.
        let refused = admit(30 * 1024 * 1024, 25 * 1024 * 1024, 8 * 1024 * 1024);
        assert!(!refused.ok);
        assert_eq!(refused.headroom_kib, 5 * 1024 * 1024);
        assert_eq!(refused.needed_kib, 8 * 1024 * 1024);
        // 40 GiB free, the same work: admitted.
        assert!(admit(40 * 1024 * 1024, 25 * 1024 * 1024, 8 * 1024 * 1024).ok);
    }

    #[test]
    fn the_seam_at_exactly_the_headroom_refuses_and_at_exactly_the_budget_does_not() {
        // C1's boundary pair. Work costing exactly the spare room consumes the
        // reserve to its floor and leaves the next unit nothing — refused.
        let exact = admit(33 * 1024 * 1024, 25 * 1024 * 1024, 8 * 1024 * 1024);
        assert_eq!(exact.headroom_kib, exact.needed_kib);
        assert!(!exact.ok, "equal-to-headroom must refuse: {exact:?}");
        // One kibibyte under the estimate is admitted; one over is not.
        assert!(admit(33 * 1024 * 1024, 25 * 1024 * 1024, 8 * 1024 * 1024 - 1).ok);
        assert!(!admit(33 * 1024 * 1024, 25 * 1024 * 1024, 8 * 1024 * 1024 + 1).ok);
        // The mirror seam: a settle exactly at its budget is clean, not a runaway.
        let at_budget = settle(100 * 1024 * 1024, 92 * 1024 * 1024, 8 * 1024 * 1024);
        assert_eq!(at_budget.delta_kib, 8 * 1024 * 1024);
        assert!(!at_budget.runaway, "equal-to-budget must not be a runaway");
        // One kibibyte past the budget is.
        assert!(settle(100 * 1024 * 1024, 92 * 1024 * 1024 - 1, 8 * 1024 * 1024).runaway);
        // A reserve larger than what is available leaves no headroom at all,
        // rather than a wrapped number.
        let none = admit(1024, 2048, 1);
        assert_eq!(none.headroom_kib, 0);
        assert!(!none.ok);
    }

    #[test]
    fn work_past_its_own_budget_settles_as_a_runaway() {
        let clean = settle(100 * 1024 * 1024, 95 * 1024 * 1024, 8 * 1024 * 1024);
        assert!(!clean.runaway);
        assert_eq!(clean.delta_kib, 5 * 1024 * 1024);
        let runaway = settle(100 * 1024 * 1024, 80 * 1024 * 1024, 8 * 1024 * 1024);
        assert!(runaway.runaway);
        assert_eq!(runaway.delta_kib, 20 * 1024 * 1024);
    }

    #[test]
    fn a_reading_carries_the_reserve_its_source_and_its_admission() {
        // A roomy machine — 460 GiB, 40% standing free — with the tree inside
        // both ceilings.
        let env = Env {
            free_fractions: vec![0.40, 0.38, 0.41],
            fill_gib_per_day: 0.5,
        };
        let lines = evaluate_tree(&parse_ps(PS)[..1]);
        let volume = 460 * 1024 * 1024;
        let reading = Reading::new(lines, volume, 116 * 1024 * 1024, &env, None);
        assert_eq!(reading.appetite, Appetite::Roomy);
        assert_eq!(reading.reserve_source, "inferred");
        assert_eq!(reading.reserve_kib, (volume as f64 * 0.04) as u64);
        assert_eq!(reading.records, 3);
        assert!(reading.standing_free.unwrap() > 0.39);
        assert!(reading.ok());
        // 116 GiB free, 18.4 GiB reserve, an 8 GiB gate: admitted with room.
        let gate = reading.admission(CLASS_RELEASE_GATE);
        assert!(gate.ok, "{gate:?}");
        assert_eq!(gate.needed_kib, ESTIMATE_RELEASE_GATE_KIB);
        // The same machine holding 25 GiB free: the gate is refused, and the
        // numbers say why. 25 - 18.4 = 6.6 GiB of headroom against 8 GiB.
        let tight = Reading::new(reading.lines.clone(), volume, 25 * 1024 * 1024, &env, None);
        let refused = tight.admission(CLASS_RELEASE_GATE);
        assert!(!refused.ok, "{refused:?}");
        assert!(refused.headroom_kib < refused.needed_kib);
        // A plain run grades against its own budget, not a class prior.
        assert_eq!(
            reading.admission(CLASS_RUN).needed_kib,
            RUN_BUDGET_KIB,
            "a run with no declared estimate is admitted against its own budget"
        );
        // The owner's declaration wins and the reading says so.
        let declared = Reading::new(
            reading.lines.clone(),
            volume,
            116 * 1024 * 1024,
            &env,
            Some(Appetite::Tight),
        );
        assert_eq!(declared.reserve_source, "declared");
        assert!(declared.reserve_kib > reading.reserve_kib);
    }

    #[test]
    fn the_reading_publishes_its_numbers_as_one_document() {
        let env = Env {
            free_fractions: vec![0.08, 0.09, 0.07],
            fill_gib_per_day: 1.2,
        };
        let volume = 460 * 1024 * 1024;
        let reading = Reading::new(
            evaluate_tree(&parse_ps(PS)[..1]),
            volume,
            100 * 1024 * 1024,
            &env,
            None,
        );
        let doc = reading.to_json(Some(CLASS_RELEASE_GATE));
        assert_eq!(doc["appetite"], "tight");
        assert_eq!(doc["reserve_source"], "inferred");
        assert_eq!(doc["volume_kib"].as_u64().unwrap(), volume);
        assert_eq!(doc["reserve_kib"].as_u64().unwrap(), reading.reserve_kib);
        assert_eq!(doc["work_class"], CLASS_RELEASE_GATE);
        assert_eq!(
            doc["estimate_kib"].as_u64().unwrap(),
            ESTIMATE_RELEASE_GATE_KIB
        );
        assert!(doc["admitted"].is_boolean(), "{doc}");
        // The document carries every line, graded — the reader never has to
        // re-derive a verdict from a number.
        let lines = doc["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line["ok"].is_boolean()));
        // With no class asked for, no admission is claimed.
        let bare = reading.to_json(None);
        assert!(bare.get("admitted").is_none(), "{bare}");
    }

    #[test]
    fn every_class_prior_is_declared_once_and_a_plain_run_has_none() {
        assert_eq!(
            estimate_kib(CLASS_RELEASE_GATE),
            Some(ESTIMATE_RELEASE_GATE_KIB)
        );
        assert_eq!(
            estimate_kib(CLASS_RELEASE_ASSETS),
            Some(ESTIMATE_RELEASE_ASSETS_KIB)
        );
        assert_eq!(
            estimate_kib(CLASS_LANE_BOOTSTRAP),
            Some(ESTIMATE_LANE_BOOTSTRAP_KIB)
        );
        // A run declares no estimate: its cost is unknowable in advance, and
        // its budget is what it is measured against.
        assert_eq!(estimate_kib(CLASS_RUN), None);
        assert_eq!(estimate_kib("run:/tmp/scratch"), None);
        // The run budget is the bound work with no declared estimate keeps, so
        // it must be a real bound: a zero would admit nothing ever.
        assert_eq!(RUN_BUDGET_KIB, 2 * 1024 * 1024);
    }
}
