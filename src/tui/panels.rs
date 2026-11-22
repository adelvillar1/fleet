//! The knowledge panels' reads. They run on their own thread — the
//! library scan and the archive replay are too heavy for the event
//! loop — and the same `boards` readers the browser face uses, so the
//! two surfaces cannot disagree about what the machine holds.

use std::path::Path;

use crate::boards;
use crate::tui::app::{
    BrainSectionRow, CalibrationData, DelegationGraph, DelegationPanel, PanelData, PanelKind,
    SecondBrainPanel, WorkBoard, WorkCard, WorkColumn,
};

/// Read one panel's data. A read that cannot land is the error the
/// panel's banner carries; the panel itself stays open.
pub fn read(home: &Path, kind: PanelKind, session: &str) -> Result<PanelData, String> {
    let user_home = std::env::var("HOME").ok();
    let day = crate::calib::today();
    match kind {
        PanelKind::Memory => {
            let root = crate::memory::resolve_root(&std::env::current_dir().unwrap_or_default());
            let calib_root = crate::calib::resolve_root(home, user_home.as_deref());
            Ok(PanelData::Memory(boards::read_memory(
                &root,
                &calib_root,
                &day,
            )))
        }
        PanelKind::Skills => {
            let override_dir = std::env::var("FLEET_SKILLS_DIR").ok();
            let canonical =
                crate::skills::resolve_skills_dir(override_dir.as_deref(), user_home.as_deref());
            Ok(PanelData::Skills(boards::read_skills(
                canonical.as_deref(),
                home,
            )))
        }
        PanelKind::Calibration => {
            let root = crate::calib::resolve_root(home, user_home.as_deref());
            let rows = crate::calib::read_day(&root, &day, crate::calib::READ_CAP);
            let cells = crate::calib::calibration(&rows);
            Ok(PanelData::Calibration(CalibrationData {
                day,
                rows_read: rows.len(),
                cells,
            }))
        }
        PanelKind::Bots => Ok(PanelData::Bots(crate::tui::app::BotsSnapshot::read(home))),
        PanelKind::Delegation => {
            let snapshot = boards::read_delegation(home, session);
            // The overlay join: the day's delegation rows over the
            // declared graph's nodes, plus the plan's own declared
            // workload — the same two `boards` functions
            // `bridge::delegation_read` joins with, so the terminal and
            // the face read one document. The trees still draw when the
            // graph cannot.
            let graph = snapshot.graph.as_ref().map(|graph| {
                let (overlays, divergences) = boards::delegation_overlay(home, &graph.nodes);
                let node_ids: Vec<String> =
                    graph.nodes.iter().map(|node| node.id.clone()).collect();
                let cwd = std::env::current_dir().unwrap_or_default();
                DelegationGraph {
                    snapshot: graph.clone(),
                    overlays,
                    divergences,
                    declared_workload: boards::declared_workload(&cwd, &node_ids),
                }
            });
            Ok(PanelData::Delegation(Box::new(DelegationPanel {
                snapshot,
                graph,
            })))
        }
        PanelKind::Board => {
            let cwd = std::env::current_dir().unwrap_or_default();
            Ok(PanelData::Board(work_board(boards::read_work_board(
                home, &cwd,
            ))))
        }
        PanelKind::SecondBrain => Ok(PanelData::SecondBrain(Box::new(second_brain(home)))),
    }
}

/// The three columns, derived from one work-board read. A card is one
/// plan's tasks standing in that column — its name, that count, and the
/// plan's own state word — and the owner's ad-hoc tasks ride beside the
/// plans as their own card. The columns are built once; the strip reads
/// their own counts.
fn work_board(read: boards::WorkBoardRead) -> WorkBoard {
    let states: [(&'static str, &'static str); 3] = [
        ("queued", "queued"),
        ("in-process", "in process"),
        ("done", "done"),
    ];
    let columns = states
        .iter()
        .map(|(key, label)| {
            let mut cards: Vec<WorkCard> = Vec::new();
            for plan in &read.plans {
                let count = plan.tasks.iter().filter(|task| task.column == *key).count();
                if count > 0 {
                    cards.push(WorkCard {
                        plan: plan.id.clone(),
                        count,
                        state: plan.status.clone(),
                    });
                }
            }
            let loose = read.tasks.iter().filter(|task| task.column == *key).count();
            if loose > 0 {
                cards.push(WorkCard {
                    plan: "ad-hoc work".to_string(),
                    count: loose,
                    state: "the owner's own".to_string(),
                });
            }
            WorkColumn { key, label, cards }
        })
        .collect();
    WorkBoard {
        read_at: boards::now_stamp(),
        columns,
        refused: read.refused,
    }
}

/// The second brain, read the way the face's `#/second-brain` reads it:
/// the same home resolution (`SECOND_BRAIN_HOME`, then `~/.second-brain`)
/// and the same six-file `boards::read_brain`. The morning read is the
/// face's five rules over that document — each line derived, and a
/// section that did not land prints its own refusal by name.
fn second_brain(home: &Path) -> SecondBrainPanel {
    let passphrase = crate::home::vault_passphrase();
    let dotenv = crate::home::read_dotenv_with_vault(home, passphrase.as_deref());
    let sources = boards::resolve_sources(home, &dotenv);
    let read_at = boards::now_stamp();
    let Some(brain_home) = sources.brain_home.clone() else {
        let said = "the second-brain board has no home on this machine".to_string();
        return SecondBrainPanel {
            home: String::new(),
            read_at,
            morning: vec![said.clone(); 5],
            sections: Vec::new(),
            refused: vec![said],
        };
    };
    let home_shown = brain_home.display().to_string();
    match boards::read_brain(&brain_home, &sources.bounds) {
        Ok(snapshot) => second_brain_panel(home_shown, read_at, snapshot),
        Err(reason) => SecondBrainPanel {
            home: home_shown,
            read_at,
            morning: vec![reason.clone(); 5],
            sections: Vec::new(),
            refused: vec![reason],
        },
    }
}

fn second_brain_panel(
    home: String,
    read_at: String,
    snapshot: boards::BrainSnapshot,
) -> SecondBrainPanel {
    let reports = snapshot.reports.landed();
    let attention = snapshot
        .tower
        .landed()
        .map(|tower| boards::attention_queue(tower, reports));
    let mut refused: Vec<String> = Vec::new();
    for reason in [
        snapshot.gates.reason(),
        snapshot.harness.reason(),
        snapshot.loops.reason(),
        snapshot.reports.reason(),
        snapshot.maturity.reason(),
        snapshot.tower.reason(),
    ]
    .into_iter()
    .flatten()
    {
        refused.push(reason.to_string());
    }

    let mut morning: Vec<String> = Vec::new();
    // 1 needs-you: the queue's own count and its own counts.
    match &attention {
        Some(attention) => {
            morning.push(if attention.total > 0 {
                format!(
                    "{} things need you \u{2014} {} drift \u{b7} {} stale \u{b7} {} handoffs \u{b7} {} conflicts",
                    attention.total,
                    attention.drift.len(),
                    attention.stale.len(),
                    attention.handoff.len(),
                    attention.conflicts.len()
                )
            } else {
                "nothing needs you today".to_string()
            });
        }
        None => morning.push(section_refusal(&snapshot.tower)),
    }
    // 2 first: the queue's own priority order, and its first row's words.
    match &attention {
        Some(attention) => {
            let top = attention
                .drift
                .first()
                .or_else(|| attention.stale.first())
                .or_else(|| attention.handoff.first())
                .or_else(|| attention.conflicts.first());
            let said = top
                .map(|row| {
                    [row.project.clone(), row.what.clone(), row.evidence.clone()]
                        .into_iter()
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(" \u{b7} ")
                })
                .unwrap_or_default();
            morning.push(if said.is_empty() {
                "nothing is queued first".to_string()
            } else {
                said
            });
        }
        None => morning.push(section_refusal(&snapshot.tower)),
    }
    // 3 in-flight: the reports file's own doing/verification/conflict
    // counts.
    match &snapshot.reports {
        boards::Section::Landed(reports) => {
            morning.push(
                if reports.doing_total > 0
                    || reports.pending_verification > 0
                    || reports.open_conflicts > 0
                {
                    format!(
                        "{} in flight \u{b7} {} awaiting verification \u{b7} {} conflicts open",
                        reports.doing_total, reports.pending_verification, reports.open_conflicts
                    )
                } else {
                    "nothing in flight".to_string()
                },
            );
        }
        boards::Section::Refused(reason) => morning.push(reason.clone()),
    }
    // 4 last-ran: the harness log's newest event, in its own words.
    match &snapshot.harness {
        boards::Section::Landed(harness) => {
            morning.push(match harness.recent.first() {
                Some(last) => format!(
                    "{} {}: {} \u{2014} {}",
                    last.ts, last.kind, last.label, last.outcome
                ),
                None => "no harness events yet".to_string(),
            });
        }
        boards::Section::Refused(reason) => morning.push(reason.clone()),
    }
    // 5 loops: the newest finding's own sentence, else the review the
    // loops owe, else the quiet.
    match &snapshot.loops {
        boards::Section::Landed(loops) => {
            let owed: u64 = loops.loops.iter().map(|row| row.pending_review).sum();
            morning.push(match loops.findings.first() {
                Some(finding) => finding.clone(),
                None if owed > 0 => format!("{owed} loop iterations await review"),
                None => "the loops are quiet".to_string(),
            });
        }
        boards::Section::Refused(reason) => morning.push(reason.clone()),
    }

    let mut sections: Vec<BrainSectionRow> = Vec::new();
    sections.push(section_row("gates", &snapshot.gates, |gates| {
        let rows = gates
            .board
            .iter()
            .take(5)
            .map(|gate| {
                format!(
                    "{} \u{b7} {}v \u{b7} {}",
                    gate.gate, gate.verdicts, gate.graded_progress
                )
            })
            .collect();
        (
            format!(
                "{} verdicts \u{b7} {} dispositions \u{b7} {} events",
                boards::grouped(gates.totals.verdicts),
                boards::grouped(gates.totals.dispositions),
                boards::grouped(gates.totals.events)
            ),
            rows,
        )
    }));
    sections.push(section_row("harness", &snapshot.harness, |harness| {
        let runs: u64 = harness.loops.iter().map(|row| row.runs).sum();
        let converged: u64 = harness.loops.iter().map(|row| row.converged).sum();
        let rows = harness
            .recent
            .iter()
            .take(3)
            .map(|row| {
                format!(
                    "{} {}: {} \u{2014} {}",
                    row.ts, row.kind, row.label, row.outcome
                )
            })
            .collect();
        (format!("{runs} runs \u{b7} {converged} converged"), rows)
    }));
    sections.push(section_row("loops", &snapshot.loops, |loops| {
        let pending: u64 = loops.loops.iter().map(|row| row.pending_review).sum();
        let rows = loops
            .loops
            .iter()
            .take(5)
            .map(|row| {
                format!(
                    "{} \u{b7} {} \u{b7} {} \u{b7} {} iterations",
                    row.name, row.family, row.verdict, row.iterations
                )
            })
            .collect();
        (
            format!(
                "{} loops \u{b7} {pending} pending review",
                loops.loops.len()
            ),
            rows,
        )
    }));
    sections.push(section_row("reports", &snapshot.reports, |reports| {
        let rows = reports
            .feed
            .iter()
            .take(3)
            .map(|row| format!("{} {} \u{b7} {}", row.ts, row.kind, row.summary))
            .collect();
        (
            format!(
                "{} in flight \u{b7} {} awaiting verification \u{b7} {} conflicts",
                reports.doing_total, reports.pending_verification, reports.open_conflicts
            ),
            rows,
        )
    }));
    sections.push(section_row("maturity", &snapshot.maturity, |maturity| {
        let rows = maturity
            .series
            .iter()
            .take(5)
            .map(|(name, points)| format!("{name} \u{b7} {} weeks", points.len()))
            .collect();
        (
            format!(
                "{} weeks \u{b7} {} series \u{b7} {} milestones",
                maturity.weeks.len(),
                maturity.series.len(),
                maturity.milestones.len()
            ),
            rows,
        )
    }));
    sections.push(section_row("tower", &snapshot.tower, |tower| {
        let rows = tower
            .projects
            .iter()
            .take(3)
            .map(|project| {
                format!(
                    "{} \u{b7} {} commits in 30d \u{b7} {} dirty",
                    project.name, project.commits_30d, project.dirty
                )
            })
            .collect();
        (
            format!(
                "{} projects \u{b7} {} days of commits",
                tower.projects.len(),
                tower.timeline.len()
            ),
            rows,
        )
    }));

    SecondBrainPanel {
        home,
        read_at,
        morning,
        sections,
        refused,
    }
}

/// One section row: landed with its count line and rows, or refused with
/// the read's own sentence. A refused section never carries a zero.
fn section_row<T>(
    name: &'static str,
    section: &boards::Section<T>,
    landed: impl FnOnce(&T) -> (String, Vec<String>),
) -> BrainSectionRow {
    match section {
        boards::Section::Landed(data) => {
            let (counts, rows) = landed(data);
            BrainSectionRow {
                name,
                landed: Some(counts),
                refused: None,
                rows,
            }
        }
        boards::Section::Refused(reason) => BrainSectionRow {
            name,
            landed: None,
            refused: Some(reason.clone()),
            rows: Vec::new(),
        },
    }
}

/// A section's refusal, by name: the read's own sentence when it has
/// one, never a zero dressed as a read.
fn section_refusal<T>(section: &boards::Section<T>) -> String {
    section
        .reason()
        .map(str::to_string)
        .unwrap_or_else(|| "the second-brain read has not landed".to_string())
}
