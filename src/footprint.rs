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
}
