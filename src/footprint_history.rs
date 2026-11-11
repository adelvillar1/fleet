//! The environment's history, read once and read the same way everywhere.
//!
//! `scripts/footprint.sh --record` appends one line per day to
//! `<FLEET_HOME>/footprint-history.jsonl`: the volume, the free space, and when.
//! Three callers need the same series — `fleet footprint` (the report and the
//! JSON document), `fleet doctor`'s disk line, and the agent loop's admission —
//! and before this module each had its own reader. The plan's C10 requires the
//! profile and every number derived from it to come from *one* reader, because
//! two readers that disagree is the same defect as two reserve formulas.
//!
//! The profile is derived, never stored: [`profile`] turns the rows into the
//! [`Env`] the contract reads, and the machine half is measured from the
//! machine itself. Nothing here calls a model, and a host that cannot answer the
//! machine probe reads as a server rather than guessing.

use std::path::Path;

use crate::footprint::Env;

/// The file's name inside the home.
pub const HISTORY_FILE: &str = "footprint-history.jsonl";

/// One recorded reading, as the script writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The UTC stamp the script wrote, `YYYY-MM-DDTHH:MM:SSZ`.
    pub at: String,
    pub volume_kib: u64,
    pub free_kib: u64,
    /// Memory in use when the row was written, and how much the machine has.
    /// Both zero on a row written before the probe existed, or by a host that
    /// could not answer — which is why the share below is an `Option`.
    pub mem_used_kib: u64,
    pub mem_total_kib: u64,
}

/// Every readable row of the home's history, oldest first. A malformed line is
/// skipped rather than guessed at — the script appends under its own lock and a
/// torn tail is a crash, not a reading — and a missing file is an empty history,
/// which is what a fresh machine has.
pub fn records(home: &Path) -> Vec<Record> {
    let text = std::fs::read_to_string(home.join(HISTORY_FILE)).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let doc: serde_json::Value = serde_json::from_str(line).ok()?;
            let volume_kib = doc.get("volume_kib").and_then(|v| v.as_u64()).unwrap_or(0);
            let free_kib = doc.get("free_kib").and_then(|v| v.as_u64()).unwrap_or(0);
            // A row with no volume cannot produce a share, and a share is the
            // only thing this series is for.
            if volume_kib == 0 {
                return None;
            }
            Some(Record {
                at: doc
                    .get("at")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                volume_kib,
                free_kib,
                mem_used_kib: doc
                    .get("mem_used_kib")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                mem_total_kib: doc
                    .get("mem_total_kib")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            })
        })
        .collect()
}

/// The environment profile: the free-share series, the fill rate the series
/// implies, the volume it was measured on, and which machine this is.
///
/// This is the plan's C10 in one function — *"an environment profile (volume
/// size, standing free space, fill rate, memory pressure, laptop or server) is
/// derived from the footprint rows"*. Memory pressure is not read here: the
/// process tree's own reading already grades memory every `fleet footprint`
/// call, and duplicating it into the profile would be a second opinion about a
/// number that has one home.
pub fn profile(home: &Path) -> Env {
    profile_from(&records(home), is_laptop())
}

/// [`profile`] over rows a caller already read — so a test supplies its own
/// series and no home, and the derivation is testable without a filesystem.
pub fn profile_from(rows: &[Record], laptop: bool) -> Env {
    let mut env = Env {
        volume_kib: rows.last().map(|row| row.volume_kib).unwrap_or(0),
        laptop,
        ..Env::default()
    };
    for row in rows {
        env.free_fractions
            .push(row.free_kib as f64 / row.volume_kib as f64);
    }
    env.fill_gib_per_day = fill_gib_per_day(rows);
    // The pressure is the newest row's, not a series: what matters for the
    // profile is how loaded the machine is now, and a median over ninety days
    // would describe a machine nobody is using.
    env.memory_used_share = rows
        .last()
        .filter(|row| row.mem_total_kib > 0)
        .map(|row| (row.mem_used_kib as f64 / row.mem_total_kib as f64).clamp(0.0, 1.0));
    env
}

/// The volume's fall rate in GiB per day: the total drop over the span the
/// records cover, divided by that span **in days**. Zero when the history
/// cannot answer — fewer than two records, one day, or a series that is not
/// falling — never a negative rate the caller would have to remember to clamp.
///
/// The span is in days, not records: two records an hour apart are a steep rate
/// and two a week apart are not, and only the calendar can tell them apart.
pub fn fill_gib_per_day(rows: &[Record]) -> f64 {
    let parsed: Vec<(f64, f64)> = rows
        .iter()
        .filter_map(|row| {
            let day = day_number(&row.at)?;
            Some((day, row.free_kib as f64 / 1024.0 / 1024.0))
        })
        .collect();
    let (Some(first), Some(last)) = (parsed.first(), parsed.last()) else {
        return 0.0;
    };
    let span = last.0 - first.0;
    if span <= 0.0 {
        return 0.0;
    }
    let per_day = (first.1 - last.1) / span;
    per_day.max(0.0)
}

/// One record's UTC timestamp as a day ordinal, so a fill rate is a span in
/// days rather than a count of records. `None` for a stamp that does not open
/// with `YYYY-MM-DD`.
fn day_number(at: &str) -> Option<f64> {
    let date = at.get(..10)?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Days from the civil epoch (Howard Hinnant's algorithm) — exact for every
    // date the history can hold, and no dependency for one subtraction.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe - 719468) as f64)
}

/// Whether this machine runs on a portable's cell. A portable is the
/// constrained machine the plan's closing frame names: a full disk on one costs
/// its owner the machine, where on a server it costs a slow afternoon.
///
/// `pmset -g batt` answers "AC Power" on a desktop and names a cell source or a
/// cell percentage on a portable, so the probe looks for either and lowercases
/// the output first — the tool's own capitalisation is not a fact this module
/// depends on. A host that cannot answer reads as a server (the roomier grade),
/// because a probe's failure must not silently tighten every number on the
/// machine. `FLEET_MACHINE` overrides it outright for a machine whose probe
/// lies, and it is the only declared input this module accepts.
pub fn is_laptop() -> bool {
    if let Ok(named) = std::env::var("FLEET_MACHINE") {
        match named.trim().to_ascii_lowercase().as_str() {
            "laptop" => return true,
            "server" => return false,
            _ => {}
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output();
        if let Ok(out) = out {
            let text = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
            // "now drawing from 'battery power'", or a cell percentage line: a
            // desktop reports neither.
            return text.contains("battery power") || text.contains("internalbattery");
        }
    }
    #[cfg(target_os = "linux")]
    {
        let power = Path::new("/sys/class/power_supply");
        if let Ok(entries) = std::fs::read_dir(power) {
            return entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with("bat")
            });
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(at: &str, free_gib: u64) -> Record {
        Record {
            at: at.to_string(),
            volume_kib: 460 * 1024 * 1024,
            free_kib: free_gib * 1024 * 1024,
            mem_used_kib: 12 * 1024 * 1024,
            mem_total_kib: 16 * 1024 * 1024,
        }
    }

    #[test]
    fn the_profile_is_derived_from_the_rows_it_was_given() {
        let rows = vec![
            row("2026-11-08T09:20:00Z", 130),
            row("2026-11-09T09:20:00Z", 128),
            row("2026-11-10T09:20:00Z", 120),
        ];
        let env = profile_from(&rows, true);
        assert_eq!(env.free_fractions.len(), 3);
        assert_eq!(env.volume_kib, 460 * 1024 * 1024);
        assert!(env.laptop);
        assert_eq!(env.machine(), "laptop");
        // 10 GiB over two days: 5 GiB/day, and the standing share is the
        // median 128/460 rather than the last reading's 120/460.
        assert!((env.fill_gib_per_day - 5.0).abs() < 0.001, "{env:?}");
        assert!((env.standing_free().unwrap() - 128.0 / 460.0).abs() < 0.0001);
    }

    #[test]
    fn the_memory_pressure_is_the_newest_rows_and_absent_when_unmeasured() {
        let rows = vec![
            row("2026-11-09T09:20:00Z", 130),
            row("2026-11-10T09:20:00Z", 120),
        ];
        let env = profile_from(&rows, true);
        assert_eq!(env.memory_used_share, Some(0.75), "12 of 16 GiB");
        // A row written before the probe existed carries no pressure, and the
        // profile says "not measured" rather than claiming an idle machine.
        let old = Record {
            mem_used_kib: 0,
            mem_total_kib: 0,
            ..row("2026-11-10T09:20:00Z", 120)
        };
        assert_eq!(profile_from(&[old], true).memory_used_share, None);
        // A host whose probe answered nonsense is clamped, never negative or
        // above one: the share is a fraction of the RAM by construction.
        let absurd = Record {
            mem_used_kib: 99 * 1024 * 1024,
            mem_total_kib: 16 * 1024 * 1024,
            ..row("2026-11-10T09:20:00Z", 120)
        };
        assert_eq!(profile_from(&[absurd], true).memory_used_share, Some(1.0));
        assert_eq!(profile_from(&[], true).memory_used_share, None);
    }

    #[test]
    fn a_history_that_cannot_answer_rates_zero_rather_than_negative() {
        // One record: no span.
        assert_eq!(fill_gib_per_day(&[row("2026-11-10T09:20:00Z", 120)]), 0.0);
        // Two records on the same day: no span in days, however many records.
        let same_day = vec![
            row("2026-11-10T01:00:00Z", 120),
            row("2026-11-10T23:00:00Z", 90),
        ];
        assert_eq!(fill_gib_per_day(&same_day), 0.0);
        // A series that gained space reports zero, never a negative rate.
        let cleaned = vec![
            row("2026-11-09T09:20:00Z", 90),
            row("2026-11-10T09:20:00Z", 130),
        ];
        assert_eq!(fill_gib_per_day(&cleaned), 0.0);
        assert_eq!(fill_gib_per_day(&[]), 0.0);
    }

    #[test]
    fn a_malformed_or_volumeless_line_is_skipped_not_guessed() {
        let dir = std::env::temp_dir().join(format!("fleet-fph-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(HISTORY_FILE);
        std::fs::write(
            &path,
            "{\"at\":\"2026-11-09T09:20:00Z\",\"volume_kib\":1048576,\"free_kib\":524288}\n\
             not json at all\n\
             {\"at\":\"2026-11-10T09:20:00Z\",\"volume_kib\":0,\"free_kib\":10}\n\
             {\"at\":\"2026-11-10T09:20:00Z\",\"volume_kib\":1048576,\"free_kib\":262144}\n",
        )
        .unwrap();
        let rows = records(&dir);
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(rows[0].free_kib, 524288);
        assert_eq!(rows[1].free_kib, 262144);
        // A missing home is an empty history, not an error.
        assert!(records(&dir.join("nope")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_override_names_the_machine_and_an_unknown_word_falls_through() {
        // The probe is the environment's; the override is the one declared
        // input, and a word it does not know must fall through to the probe
        // rather than guess.
        std::env::set_var("FLEET_MACHINE", "laptop");
        assert!(is_laptop());
        std::env::set_var("FLEET_MACHINE", "server");
        assert!(!is_laptop());
        std::env::remove_var("FLEET_MACHINE");
        let probed = is_laptop();
        std::env::set_var("FLEET_MACHINE", "workstation");
        assert_eq!(
            is_laptop(),
            probed,
            "an unknown word must answer exactly what the probe answers"
        );
        std::env::remove_var("FLEET_MACHINE");
    }
}

/// The declared halves of the contract, as the owner wrote them: the appetite
/// word and the reserve floor in GiB.
///
/// One struct so the three surfaces that build a reading (`fleet footprint`,
/// `fleet doctor`'s disk line, and the agent loop's admission) pass the same two
/// inputs, and so a test can state a declaration without a home on disk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Declared {
    /// The appetite word, or `None` to infer it from the history.
    pub appetite: Option<crate::footprint::Appetite>,
    /// The reserve floor in GiB, or `None` for the appetite's share.
    pub reserve_gib: Option<u64>,
}

impl Declared {
    /// The floor in kibibytes, which is the unit the contract reads.
    pub fn reserve_kib(&self) -> Option<u64> {
        self.reserve_gib
            .map(|gib| gib.saturating_mul(1024).saturating_mul(1024))
    }
}

/// Read the owner's declaration out of the home's `config.json`. An unreadable
/// or unparseable file is *no declaration*, never an error: every surface that
/// asks this must keep working on a home whose config a half-written edit left
/// broken, and the contract's own fallback (inference) is the correct answer
/// then — the same rule `declared_appetite` has always applied.
///
/// `steady` is accepted as `roomy`: the plan's closing frame uses the word for
/// the owner who keeps a wide reserve, and the contract's own vocabulary names
/// that grade `roomy`. Both spellings mean the same share.
pub fn declared(home: &Path) -> Declared {
    let Ok(text) = std::fs::read_to_string(home.join("config.json")) else {
        return Declared::default();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Declared::default();
    };
    let appetite = match doc.get("appetite").and_then(|value| value.as_str()) {
        Some("tight") => Some(crate::footprint::Appetite::Tight),
        Some("balanced") => Some(crate::footprint::Appetite::Balanced),
        Some("roomy") | Some("steady") => Some(crate::footprint::Appetite::Roomy),
        _ => None,
    };
    let reserve_gib = doc
        .get("footprint")
        .and_then(|section| section.get("reserve_gib"))
        .and_then(|value| value.as_u64())
        .filter(|gib| *gib > 0);
    Declared {
        appetite,
        reserve_gib,
    }
}

#[cfg(test)]
mod declared_tests {
    use super::*;
    use crate::footprint::Appetite;

    fn home_with(config: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "fleet-declared-{}-{}",
            std::process::id(),
            config.len()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), config).unwrap();
        dir
    }

    #[test]
    fn the_declaration_is_read_with_steady_meaning_roomy() {
        let dir = home_with(r#"{"appetite":"tight","footprint":{"reserve_gib":40}}"#);
        let read = declared(&dir);
        assert_eq!(read.appetite, Some(Appetite::Tight));
        assert_eq!(read.reserve_gib, Some(40));
        assert_eq!(read.reserve_kib(), Some(40 * 1024 * 1024));
        let _ = std::fs::remove_dir_all(&dir);

        let steady = home_with(r#"{"appetite":"steady"}"#);
        assert_eq!(declared(&steady).appetite, Some(Appetite::Roomy));
        assert_eq!(declared(&steady).reserve_gib, None);
        let _ = std::fs::remove_dir_all(&steady);
    }

    #[test]
    fn no_declaration_is_the_default_and_a_broken_one_is_not_an_error() {
        // No file at all.
        let missing = std::env::temp_dir().join("fleet-declared-absent-home");
        assert_eq!(declared(&missing), Declared::default());
        // A file that is not JSON, and one that is JSON but not an object.
        let broken = home_with("{not json");
        assert_eq!(declared(&broken), Declared::default());
        let _ = std::fs::remove_dir_all(&broken);
        let wrong = home_with("[]");
        assert_eq!(declared(&wrong), Declared::default());
        let _ = std::fs::remove_dir_all(&wrong);
        // A word the contract does not own is no declaration, not a guess.
        let unknown = home_with(r#"{"appetite":"generous"}"#);
        assert_eq!(declared(&unknown), Declared::default());
        let _ = std::fs::remove_dir_all(&unknown);
        // A zero floor would admit nothing ever: it reads as unset.
        let zero = home_with(r#"{"footprint":{"reserve_gib":0}}"#);
        assert_eq!(declared(&zero).reserve_gib, None);
        let _ = std::fs::remove_dir_all(&zero);
    }
}
