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

/// How many daily readings the standing free fraction is read over: the plan's
/// own thirty, which is long enough that a week of hard work does not reclassify
/// the machine and short enough that a permanently fuller disk does.
pub const STANDING_WINDOW: usize = 30;

/// The free share under which a reading counts as *near the floor*. Not the
/// reserve — this is the history's shape, read before any reserve exists.
pub const NEAR_FLOOR: f64 = 0.10;

/// How far the free share swings around its own median before the machine reads
/// as unsettled. Five points of the volume: a machine that sits at 20% and
/// swings to 8% has no margin to absorb a runaway even though its median looks
/// comfortable, and the plan's rule is that such a machine makes fleet tighter.
pub const VOLATILE_MAD: f64 = 0.05;

/// What the environment's history says about its margin (the owner's ruling,
/// 2026-11-09: the appetite is inferable from the environment itself).
///
/// This is the **profile** half of the plan's C10: the free-share series, the
/// fill rate, the volume it was measured on, and the one fact about the machine
/// itself that changes what a full disk costs its owner — whether it is a laptop
/// (constrained) or a server (roomy). Every field is measured or declared; none
/// is inferred into the struct, so a caller cannot pass a conclusion as a fact.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Env {
    /// One sample per day: the free share of the volume, newest last.
    pub free_fractions: Vec<f64>,
    /// How fast free space falls, GiB per day (0 when it is not filling).
    pub fill_gib_per_day: f64,
    /// The volume's size in kibibytes, as the history measured it.
    pub volume_kib: u64,
    /// Whether the machine runs on a battery — a laptop, and therefore the one
    /// whose owner feels a full disk as a broken machine rather than a slow one.
    pub laptop: bool,
    /// Memory pressure at the last recorded reading: used over total, as a
    /// share. `None` is a machine whose history predates the field or whose
    /// probe could not answer — never a claim of zero, which would read as an
    /// idle machine.
    pub memory_used_share: Option<f64>,
}

impl Env {
    /// The readings the standing share is computed over: the newest
    /// [`STANDING_WINDOW`], so an old roomy month cannot outvote today.
    pub fn window(&self) -> &[f64] {
        let start = self.free_fractions.len().saturating_sub(STANDING_WINDOW);
        &self.free_fractions[start..]
    }

    /// The standing free share: the median, not the latest reading — one good
    /// day after a cleanup must not read as a roomy machine.
    pub fn standing_free(&self) -> Option<f64> {
        median(self.window())
    }

    /// How many of the window's readings sit under [`NEAR_FLOOR`].
    pub fn dips(&self) -> usize {
        self.window()
            .iter()
            .filter(|free| **free < NEAR_FLOOR)
            .count()
    }

    /// How far the free share swings: the median absolute deviation from the
    /// standing median. Zero for a one-record history, which is honest — one
    /// reading has no spread, and the appetite reads only what it measured.
    pub fn volatility(&self) -> f64 {
        let Some(mid) = self.standing_free() else {
            return 0.0;
        };
        let deviations: Vec<f64> = self
            .window()
            .iter()
            .map(|free| (free - mid).abs())
            .collect();
        median(&deviations).unwrap_or(0.0)
    }

    /// Which machine this is, in the contract's two words.
    pub fn machine(&self) -> &'static str {
        if self.laptop {
            "laptop"
        } else {
            "server"
        }
    }
}

/// The median of a series, or `None` when it is empty.
fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    })
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

    /// One step tighter, never looser: what a volatile history buys. `Tight` is
    /// the floor, so the step saturates rather than wrapping.
    pub fn tighter(self) -> Appetite {
        match self {
            Appetite::Tight => Appetite::Tight,
            Appetite::Balanced => Appetite::Tight,
            Appetite::Roomy => Appetite::Balanced,
        }
    }

    /// How many turns between readings. Roomy keeps the plan's guess of five;
    /// a tighter machine reads more often, because the same five turns write
    /// proportionally more of what it has left. The number is turns, not
    /// seconds, because the reading is a `statvfs` plus a `ps` fork and a
    /// chatty run must not pay it per turn.
    pub fn cadence_turns(self) -> u64 {
        match self {
            Appetite::Tight => 2,
            Appetite::Balanced => 5,
            Appetite::Roomy => 5,
        }
    }

    /// What one run may write before it is abnormal, from the appetite and the
    /// volume it has.
    ///
    /// `records == 0` is a machine with no profile, and there the owner's
    /// settled [`RUN_BUDGET_KIB`] stands **exactly** — the plan's rule that the
    /// constants are priors for a fresh machine, replaced by the first recorded
    /// profile. With a profile, the number derives: a share of the volume,
    /// never below the declared 2 GiB (a small disk still gets a usable run) and
    /// never above four of them (a runaway stays bounded in absolute terms
    /// however large the disk is, so a 4 TiB volume cannot license a 40 GiB run).
    pub fn run_budget_kib(self, volume_kib: u64, records: usize) -> u64 {
        if records == 0 {
            return RUN_BUDGET_KIB;
        }
        let share = match self {
            Appetite::Tight => 0.002,
            Appetite::Balanced => 0.005,
            Appetite::Roomy => 0.01,
        };
        let derived = (volume_kib as f64 * share) as u64;
        derived.clamp(RUN_BUDGET_KIB, 4 * RUN_BUDGET_KIB)
    }

    /// How many days may pass between upkeep passes, from the fill rate. A
    /// machine losing 2 GiB a day with 120 GiB free does not need the same
    /// interval as one losing 2 GiB a day at 25 GiB free: the cadence is the
    /// headroom above the reserve over the daily loss, floored at one day and
    /// capped at the weekly habit the maintenance script already runs on.
    pub fn upkeep_days(self, env: &Env, available_kib: u64, reserve_kib: u64) -> u64 {
        if env.fill_gib_per_day <= 0.0 {
            return match self {
                Appetite::Tight => 1,
                Appetite::Balanced | Appetite::Roomy => 7,
            };
        }
        let headroom_gib = available_kib.saturating_sub(reserve_kib) as f64 / 1024.0 / 1024.0;
        let days = headroom_gib / env.fill_gib_per_day;
        let cap = match self {
            Appetite::Tight => 3,
            Appetite::Balanced => 7,
            Appetite::Roomy => 7,
        };
        (days.floor() as u64).clamp(1, cap)
    }
}

/// The appetite from the environment's standing free share **and its spread**.
/// A habitually full machine tightens fleet — it never relaxes it: the tolerance
/// is evidence of no margin, not a budget grant.
///
/// The spread is the plan's "and its volatility": a machine whose free share
/// swings by more than [`VOLATILE_MAD`] has no margin to absorb a runaway even
/// when its median reads comfortable, so it is graded one step tighter than its
/// standing share alone would earn. The direction is one-way on purpose — a
/// volatile history can tighten the appetite and never loosen it, so a
/// cleanup spree cannot buy a looser contract.
pub fn appetite(env: &Env) -> Appetite {
    let graded = match env.standing_free() {
        Some(share) if share < NEAR_FLOOR => Appetite::Tight,
        Some(share) if share < 0.25 => Appetite::Balanced,
        Some(_) => Appetite::Roomy,
        // No history is not room: a fresh machine starts balanced.
        None => Appetite::Balanced,
    };
    if env.volatility() > VOLATILE_MAD {
        graded.tighter()
    } else {
        graded
    }
}

/// The reserve in kibibytes, and which of the two inputs produced it: the
/// owner's explicit floor wins, then the declared appetite, then the inferred
/// one. An explicit floor is the owner naming the number itself rather than the
/// word — the plan's "plus the option to name the floor outright" — and it is
/// reported as its own source so a refusal reads as *their* number, not the
/// code's.
pub fn reserve_kib(total_kib: u64, env: &Env, declared: Option<Appetite>) -> (u64, &'static str) {
    reserve_kib_with_floor(total_kib, env, declared, None)
}

/// [`reserve_kib`] with the owner's explicit floor, in kibibytes.
pub fn reserve_kib_with_floor(
    total_kib: u64,
    env: &Env,
    declared: Option<Appetite>,
    floor_kib: Option<u64>,
) -> (u64, &'static str) {
    if let Some(floor) = floor_kib {
        return (floor, "declared floor");
    }
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
/// exist — the prior C8 replaces once three settles have landed. The numbers
/// are the owner's, settled 2026-11-10 (*"leave them like that"*): they live
/// here and nowhere else, so changing one is one edit in one place.
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
    /// How far the free share swung around its own median — the second half of
    /// the appetite's input, published so a tighter grade than the standing
    /// share alone would earn is never a mystery.
    pub volatility: f64,
    /// How many of the window's readings sat under [`NEAR_FLOOR`].
    pub dips: usize,
    /// `laptop` or `server` — the profile's machine half.
    pub machine: &'static str,
    /// Memory pressure as a share of the machine's RAM, when the profile
    /// recorded one. `None` is not measured.
    pub memory_used_share: Option<f64>,
    /// The appetite the machine's own history earns, whether or not the owner
    /// declared one. [`Reading::appetite`] is the grade **in force**; this is
    /// the one the environment would have produced, so the report can name the
    /// gap between the two rather than hiding the owner's hand.
    pub inferred_appetite: Appetite,
    /// The run budget the appetite derives, in kibibytes: what a run's ceiling
    /// is on this machine rather than on the one the constant was written for.
    pub run_budget_kib: u64,
    /// The turn cadence the appetite derives.
    pub cadence_turns: u64,
    /// The upkeep interval the fill rate derives, in days.
    pub upkeep_days: u64,
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
        Reading::with_floor(lines, volume_kib, available_kib, env, declared, None)
    }

    /// Build the reading with the owner's explicit reserve floor. The floor wins
    /// over both the declared appetite and the inference, and the reading says
    /// so in [`Reading::reserve_source`] — the plan's "say the number itself".
    pub fn with_floor(
        lines: Vec<Line>,
        volume_kib: u64,
        available_kib: u64,
        env: &Env,
        declared: Option<Appetite>,
        floor_kib: Option<u64>,
    ) -> Reading {
        let (reserve_kib, reserve_source) =
            reserve_kib_with_floor(volume_kib, env, declared, floor_kib);
        // The profile's own volume is the fallback for a caller that measured
        // none: the derived budgets are shares of a disk, and a zero volume
        // would silently floor every one of them.
        let volume_kib = if volume_kib > 0 {
            volume_kib
        } else {
            env.volume_kib
        };
        // The grade in force: the owner's word when they declared one, the
        // machine's own otherwise. Every derived number — the share, the turn
        // cadence, the run budget, the upkeep interval — reads this one, so a
        // declaration cannot tighten the reserve while leaving the rest roomy.
        let inferred_appetite = appetite(env);
        let appetite = declared.unwrap_or(inferred_appetite);
        Reading {
            lines,
            volume_kib,
            available_kib,
            reserve_kib,
            reserve_source,
            appetite,
            inferred_appetite,
            standing_free: env.standing_free(),
            fill_gib_per_day: env.fill_gib_per_day,
            records: env.free_fractions.len(),
            volatility: env.volatility(),
            dips: env.dips(),
            machine: env.machine(),
            memory_used_share: env.memory_used_share,
            run_budget_kib: appetite.run_budget_kib(volume_kib, env.free_fractions.len()),
            cadence_turns: appetite.cadence_turns(),
            upkeep_days: appetite.upkeep_days(env, available_kib, reserve_kib),
        }
    }

    /// A reading with no profile behind it: a caller that already holds the
    /// volume, the free space, the reserve and the appetite the run was admitted
    /// against, and needs the derived numbers without re-reading a history.
    ///
    /// Used where a row is landed about a run whose admission already happened:
    /// re-deriving the profile there would let a row disagree with the admission
    /// that produced it. The machine and the history fields read `unknown`,
    /// which is the truth for a reading assembled this way.
    pub fn bare(
        volume_kib: u64,
        available_kib: u64,
        reserve_kib: u64,
        reserve_source: &'static str,
        appetite: Appetite,
    ) -> Reading {
        Reading {
            lines: Vec::new(),
            volume_kib,
            available_kib,
            reserve_kib,
            reserve_source,
            appetite,
            standing_free: None,
            fill_gib_per_day: 0.0,
            records: 0,
            volatility: 0.0,
            dips: 0,
            machine: "unknown",
            memory_used_share: None,
            inferred_appetite: appetite,
            run_budget_kib: appetite.run_budget_kib(volume_kib, 0),
            cadence_turns: appetite.cadence_turns(),
            upkeep_days: appetite.upkeep_days(&Env::default(), available_kib, reserve_kib),
        }
    }

    /// The one sentence that names the inference and its evidence beside the
    /// number it produced, so a refusal reads as a decision (the plan's C11
    /// and C12): *"reserve 42 GiB of 460 GiB (inferred; laptop, 9% free for
    /// 30 record(s) ±2 pts, filling 1.2 GiB/day — the spread grades it tight)"*.
    ///
    /// Every clause is earned: the spread clause appears only when the spread
    /// actually tightened the grade, and the filling clause only when the volume
    /// is filling, so the sentence never implies a fact the history lacks.
    pub fn environment_clause(&self) -> String {
        let share = self
            .standing_free
            .map(|free| format!("{:.0}% free", free * 100.0))
            .unwrap_or_else(|| "no history".to_string());
        let mut evidence = format!("{} {share} for {} record(s)", self.machine, self.records);
        if self.records > 0 {
            evidence.push_str(&format!(" \u{b1}{:.0} pt", self.volatility * 100.0));
        }
        if self.dips > 0 {
            evidence.push_str(&format!(", {} near the floor", self.dips));
        }
        if self.fill_gib_per_day > 0.0 {
            evidence.push_str(&format!(", filling {:.1} GiB/day", self.fill_gib_per_day));
        }
        // Memory pressure is named only when the profile recorded it: an
        // unmeasured machine says nothing rather than claiming an idle one.
        if let Some(used) = self.memory_used_share {
            evidence.push_str(&format!(", memory {:.0}% used", used * 100.0));
        }
        // The spread's own consequence, named only when it fired: a reader who
        // sees a grade tighter than the standing share explains must be told
        // why. The inferred grade is what the spread acted on — the declared one
        // is the owner's and the spread never touches it.
        if self.inferred_appetite != self.by_share() {
            evidence.push_str(&format!(
                " \u{2014} the spread grades it {}",
                self.inferred_appetite.word()
            ));
        }
        // The owner's hand, when it moved the grade: the words the plan's
        // closing frame asks for, so a refusal never looks arbitrary.
        if self.appetite != self.inferred_appetite {
            evidence.push_str(&format!(
                " \u{2014} declared {}, the machine infers {}",
                self.appetite.word(),
                self.inferred_appetite.word()
            ));
        }
        format!(
            "reserve {} GiB of {} GiB ({}; {evidence})",
            self.reserve_kib / 1024 / 1024,
            self.volume_kib / 1024 / 1024,
            self.reserve_source,
        )
    }

    /// The appetite the standing free share alone earns, before the spread's
    /// step and before any declaration — the baseline both of those are
    /// measured against.
    fn by_share(&self) -> Appetite {
        match self.standing_free {
            Some(free) if free < NEAR_FLOOR => Appetite::Tight,
            Some(free) if free < 0.25 => Appetite::Balanced,
            Some(_) => Appetite::Roomy,
            None => Appetite::Balanced,
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
            estimate_kib(class).unwrap_or(self.run_budget_kib),
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
            "volatility": self.volatility,
            "dips": self.dips,
            "machine": self.machine,
            "memory_used_share": self.memory_used_share,
            "run_budget_kib": self.run_budget_kib,
            "cadence_turns": self.cadence_turns,
            "upkeep_days": self.upkeep_days,
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
            ..Env::default()
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
            ..Env::default()
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
        // A plain run grades against its own budget, not a class prior — and
        // with a profile that budget is the appetite's derivation, bounded.
        assert_eq!(
            reading.admission(CLASS_RUN).needed_kib,
            reading.run_budget_kib,
            "a run with no declared estimate is admitted against its own budget"
        );
        // Roomy at 1% of 460 GiB is 4.6 GiB: above the declared floor, under the roof.
        assert_eq!(reading.run_budget_kib, (volume as f64 * 0.01) as u64);
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
            laptop: true,
            ..Env::default()
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

    // ---- C10/C11/C12: the profile, the derived numbers, the evidence -------

    /// A profile with a given standing share and a given spread, so each rule
    /// below is pinned against one input and not a mixture.
    fn profile(standing: f64, swing: f64, records: usize) -> Env {
        // Equal counts either side of the standing share (plus the share itself
        // when the count is odd), so the median is exactly `standing` and the
        // median absolute deviation is exactly `swing` — each rule below is
        // pinned against one input rather than a mixture of two.
        let mut free_fractions = Vec::new();
        if records % 2 == 1 {
            free_fractions.push(standing);
        }
        while free_fractions.len() < records {
            free_fractions.push(standing + swing);
            if free_fractions.len() < records {
                free_fractions.push(standing - swing);
            }
        }
        Env {
            free_fractions,
            volume_kib: 460 * 1024 * 1024,
            laptop: true,
            ..Env::default()
        }
    }

    #[test]
    fn the_standing_share_reads_the_window_and_not_the_whole_history() {
        // Thirty-one records: the oldest one is outside the window and must not
        // vote, even though it is the roomiest of them all.
        let mut env = Env::default();
        env.free_fractions.push(0.90);
        for _ in 0..STANDING_WINDOW {
            env.free_fractions.push(0.20);
        }
        assert_eq!(env.window().len(), STANDING_WINDOW);
        assert_eq!(env.standing_free(), Some(0.20));
        // A history shorter than the window is read whole.
        let short = profile(0.20, 0.01, 3);
        assert_eq!(short.window().len(), 3);
        assert!((short.standing_free().unwrap() - 0.20).abs() < 1e-9);
    }

    #[test]
    fn a_volatile_history_tightens_the_appetite_and_never_loosens_it() {
        // A comfortable median with a wide swing: the spread grades it one step
        // tighter than the standing share alone would earn.
        let steady = profile(0.60, 0.01, 30);
        assert_eq!(appetite(&steady), Appetite::Roomy);
        let swingy = profile(0.60, 0.09, 30);
        assert!(swingy.volatility() > VOLATILE_MAD, "{swingy:?}");
        assert_eq!(appetite(&swingy), Appetite::Balanced, "roomy steps down");
        // The step saturates: a tight machine stays tight.
        let tight_and_swingy = profile(0.05, 0.09, 30);
        assert_eq!(appetite(&tight_and_swingy), Appetite::Tight);
        // And it never moves the other way: no spread can buy a looser grade.
        for swing in [0.0, 0.001, 0.05, 0.2] {
            for standing in [0.05, 0.12, 0.30, 0.70] {
                let graded = appetite(&profile(standing, swing, 30));
                let by_share = appetite(&profile(standing, 0.0, 30));
                assert!(
                    graded.reserve_share() >= by_share.reserve_share(),
                    "{standing}+-{swing}: {graded:?} looser than {by_share:?}"
                );
            }
        }
    }

    #[test]
    fn the_dips_are_counted_over_the_window_below_the_floor() {
        let env = Env {
            free_fractions: vec![0.05, 0.09, 0.30, 0.11, 0.02],
            ..Env::default()
        };
        assert_eq!(env.dips(), 3, "under NEAR_FLOOR: 5%, 9%, 2%");
        assert_eq!(Env::default().dips(), 0);
    }

    #[test]
    fn the_volume_and_the_cadence_and_the_upkeep_follow_the_appetite() {
        // Roomy, filling steadily: the long cadence and the weekly upkeep.
        let roomy = profile(0.60, 0.01, 30);
        let reading = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            200 * 1024 * 1024,
            &roomy,
            None,
        );
        assert_eq!(reading.appetite, Appetite::Roomy);
        assert_eq!(reading.cadence_turns, 5);
        assert_eq!(reading.machine, "laptop");
        // A tidy machine is never told to upkeep less often than weekly, and a
        // filling one is never told to wait past the appetite's own cap.
        assert_eq!(reading.upkeep_days, 7, "no fill rate: the standing habit");
        let mut filling = roomy.clone();
        filling.fill_gib_per_day = 1.0;
        let measured = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            21 * 1024 * 1024,
            &filling,
            None,
        );
        // 21 GiB free minus a 4% reserve of 18.4 GiB = 2.6 GiB of headroom, at
        // 1 GiB a day is two days before the floor.
        assert_eq!(measured.upkeep_days, 2, "{measured:?}");
        // A tight machine reads the volume twice as often and upkeeps daily.
        let mut tight = profile(0.05, 0.01, 30);
        tight.fill_gib_per_day = 0.0;
        let careful = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            60 * 1024 * 1024,
            &tight,
            None,
        );
        assert_eq!(careful.appetite, Appetite::Tight);
        assert_eq!(careful.cadence_turns, 2);
        assert_eq!(careful.upkeep_days, 1);
        // Never zero: a rate that would round to no days still upkeeps today.
        let mut huge = profile(0.60, 0.01, 30);
        huge.fill_gib_per_day = 9000.0;
        let urgent = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            200 * 1024 * 1024,
            &huge,
            None,
        );
        assert_eq!(urgent.upkeep_days, 1);
    }

    #[test]
    fn the_run_budget_is_the_prior_with_no_profile_and_a_bounded_share_with_one() {
        let volume = 460 * 1024 * 1024;
        let none = Reading::new(Vec::new(), volume, 200 * 1024 * 1024, &Env::default(), None);
        assert_eq!(
            none.run_budget_kib, RUN_BUDGET_KIB,
            "no profile: the owner's settled number stands exactly"
        );
        let roomy = Reading::new(
            Vec::new(),
            volume,
            200 * 1024 * 1024,
            &profile(0.60, 0.01, 30),
            None,
        );
        assert!(roomy.run_budget_kib > RUN_BUDGET_KIB, "{roomy:?}");
        assert!(roomy.run_budget_kib <= 4 * RUN_BUDGET_KIB, "{roomy:?}");
        // A tiny volume cannot derive below the declared floor.
        let small = Reading::new(
            Vec::new(),
            100 * 1024,
            60 * 1024,
            &profile(0.60, 0.01, 30),
            None,
        );
        assert_eq!(small.run_budget_kib, RUN_BUDGET_KIB, "the floor holds");
        // A huge one cannot license a runaway: the roof holds.
        let huge = Reading::new(
            Vec::new(),
            4096 * 1024 * 1024,
            3000 * 1024 * 1024,
            &profile(0.60, 0.01, 30),
            None,
        );
        assert_eq!(huge.run_budget_kib, 4 * RUN_BUDGET_KIB, "the roof holds");
    }

    #[test]
    fn a_declared_appetite_governs_every_derived_number_and_not_only_the_reserve() {
        // The inconsistency a live `FLEET_HOME` run found on 2026-11-11: a
        // declared `tight` derived the tight reserve but left the cadence, the
        // run budget and the upkeep reading `roomy`, because those read the
        // inferred grade. One grade is in force, and everything reads it.
        let roomy_machine = profile(0.60, 0.01, 30);
        let inferred = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            200 * 1024 * 1024,
            &roomy_machine,
            None,
        );
        let declared = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            200 * 1024 * 1024,
            &roomy_machine,
            Some(Appetite::Tight),
        );
        assert_eq!(inferred.appetite, Appetite::Roomy);
        assert_eq!(inferred.inferred_appetite, Appetite::Roomy);
        assert_eq!(declared.appetite, Appetite::Tight, "the word is in force");
        assert_eq!(
            declared.inferred_appetite,
            Appetite::Roomy,
            "and the machine's own grade is still named beside it"
        );
        // Every derived number follows the grade in force.
        assert!(declared.reserve_kib > inferred.reserve_kib, "the reserve");
        assert_eq!(declared.cadence_turns, 2, "the cadence");
        assert_eq!(inferred.cadence_turns, 5);
        assert!(
            declared.run_budget_kib < inferred.run_budget_kib,
            "the budget"
        );
        assert_eq!(declared.upkeep_days, 1, "the upkeep");
        // The clause names which produced what.
        let clause = declared.environment_clause();
        assert!(clause.contains("declared"), "{clause}");
        assert!(
            clause.contains("declared tight, the machine infers roomy"),
            "{clause}"
        );
        // The inferred reading carries no such clause: nothing was overridden.
        assert!(
            !inferred.environment_clause().contains("declared"),
            "{}",
            inferred.environment_clause()
        );
    }

    #[test]
    fn an_explicit_floor_names_itself_beside_the_word_it_overrides() {
        let env = profile(0.60, 0.01, 30);
        let volume = 460 * 1024 * 1024;
        let worded = Reading::new(
            Vec::new(),
            volume,
            200 * 1024 * 1024,
            &env,
            Some(Appetite::Tight),
        );
        assert_eq!(worded.reserve_source, "declared");
        let floored = Reading::with_floor(
            Vec::new(),
            volume,
            200 * 1024 * 1024,
            &env,
            Some(Appetite::Tight),
            Some(60 * 1024 * 1024),
        );
        assert_eq!(floored.reserve_kib, 60 * 1024 * 1024);
        assert_eq!(floored.reserve_source, "declared floor");
        // The explicit floor wins over the word, and the report says which.
        assert!(floored.reserve_kib > worded.reserve_kib);
        assert!(floored.environment_clause().contains("declared floor"));
    }

    #[test]
    fn the_environment_clause_names_its_evidence_beside_the_number() {
        // The plan's C12 sentence: the machine, the standing share, the spread,
        // the fill rate, and the inference's own words — one line, no hidden
        // step between the history and the number it produced.
        let mut env = profile(0.09, 0.01, 30);
        env.fill_gib_per_day = 1.2;
        let reading = Reading::new(Vec::new(), 460 * 1024 * 1024, 100 * 1024 * 1024, &env, None);
        let clause = reading.environment_clause();
        for expected in [
            "reserve",
            "of 460 GiB",
            "inferred",
            "laptop",
            "9% free",
            "30 record(s)",
            "filling 1.2 GiB/day",
        ] {
            assert!(
                clause.contains(expected),
                "{expected} missing from: {clause}"
            );
        }
        // A fresh machine says so rather than claiming a history it lacks.
        let fresh = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            100 * 1024 * 1024,
            &Env::default(),
            None,
        );
        assert!(
            fresh.environment_clause().contains("no history"),
            "{}",
            fresh.environment_clause()
        );
        assert!(!fresh.environment_clause().contains("filling"));
        // The spread's consequence is named only when it fired.
        let swingy = profile(0.60, 0.09, 30);
        let graded = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            300 * 1024 * 1024,
            &swingy,
            None,
        );
        assert_eq!(graded.appetite, Appetite::Balanced);
        assert!(
            graded
                .environment_clause()
                .contains("the spread grades it balanced"),
            "{}",
            graded.environment_clause()
        );
        // ...and not when the standing share alone produced the grade.
        let steady = profile(0.60, 0.01, 30);
        let plain = Reading::new(
            Vec::new(),
            460 * 1024 * 1024,
            300 * 1024 * 1024,
            &steady,
            None,
        );
        assert!(
            !plain.environment_clause().contains("spread"),
            "{}",
            plain.environment_clause()
        );
    }

    #[test]
    fn the_document_publishes_the_profile_the_clause_reads() {
        let mut env = profile(0.09, 0.01, 30);
        env.fill_gib_per_day = 1.2;
        let reading = Reading::new(Vec::new(), 460 * 1024 * 1024, 100 * 1024 * 1024, &env, None);
        let doc = reading.to_json(None);
        assert_eq!(doc["machine"], "laptop");
        // 0.09 +-0.01 puts half the window at 0.08, under NEAR_FLOOR.
        assert_eq!(doc["dips"].as_u64().unwrap(), (env.dips()) as u64);
        assert!(env.dips() > 0, "{env:?}");
        assert!(doc["volatility"].as_f64().unwrap() < VOLATILE_MAD);
        assert_eq!(doc["cadence_turns"].as_u64().unwrap(), 2);
        // Tight caps the interval at three days however slow the fill: 44.8 GiB
        // of headroom at 1.2 GiB/day would otherwise say thirty-seven.
        assert_eq!(doc["upkeep_days"].as_u64().unwrap(), 3);
        assert_eq!(
            doc["run_budget_kib"].as_u64().unwrap(),
            reading.run_budget_kib
        );
        // A reading assembled from measurements (no profile) admits its own gaps.
        let bare = Reading::bare(
            460 * 1024 * 1024,
            100 * 1024 * 1024,
            18 * 1024 * 1024,
            "inferred",
            Appetite::Roomy,
        );
        assert_eq!(bare.to_json(None)["machine"], "unknown");
        assert!(bare.to_json(None)["standing_free"].is_null());
    }
}
