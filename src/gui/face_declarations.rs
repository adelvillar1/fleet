//! The face's own declarations, read rather than copied.
//!
//! `src/gui/assets/app.js` declares both of the tables a documentation
//! count gets wrong: `ROUTES` (every address the face resolves) and
//! `RAIL_MAP` (the rows the rail actually renders, grouped into regions).
//!
//! ## Why this module exists
//!
//! The documentation audit of 2026-11-23 found the front door stating a
//! rail of "nine lenses" and a functional specification stating "fourteen
//! destinations in four groups", while the face rendered fifteen rows in
//! four regions with different names. Nothing failed: the counts lived in
//! prose, the tables lived in a shipped JavaScript asset, and no test
//! could hold the two together.
//!
//! `drift` can only register a fact it can **produce**. So the count has
//! to be derived from the declaration itself, never restated beside it —
//! this module is that derivation. It reads the shipped asset (the same
//! bytes the server embeds through `include_str!`), so a row added to the
//! rail moves the number the moment it lands, and every doc site stating
//! the old one is named by its drift row.
//!
//! ## What it deliberately does not do
//!
//! It does not evaluate JavaScript. It reads two declarations by their
//! own shape and refuses to guess: a table it cannot find is a zero, and
//! the tests below fail on a zero rather than reporting a clean count of
//! nothing.

/// The shipped face asset, the one the server embeds.
const SCRIPT: &str = include_str!("assets/app.js");

/// The `RAIL_MAP` declaration alone, from `var` to its closing brace.
fn rail_map() -> &'static str {
    let start = match SCRIPT.find("var RAIL_MAP = {") {
        Some(index) => index,
        None => return "",
    };
    let rest = &SCRIPT[start..];
    let end = rest
        .find("\n  };")
        .map(|index| index + 5)
        .unwrap_or(rest.len());
    &rest[..end]
}

/// The quoted names inside the `rows: [...]` list that follows `needle`,
/// or empty when the pair is not there.
fn rows_after(source: &str, needle: &str) -> Vec<String> {
    let after = match source.find(needle) {
        Some(index) => index,
        None => return Vec::new(),
    };
    let open = match source[after..].find("rows: [") {
        Some(offset) => after + offset + "rows: [".len(),
        None => return Vec::new(),
    };
    let close = match source[open..].find(']') {
        Some(offset) => open + offset,
        None => return Vec::new(),
    };
    source[open..close]
        .split('\'')
        .skip(1)
        .step_by(2)
        .map(|name| name.to_string())
        .collect()
}

/// Every address the face resolves, from the `ROUTES` table.
pub fn routes() -> usize {
    let start = match SCRIPT.find("var ROUTES = [") {
        Some(index) => index,
        None => return 0,
    };
    let rest = &SCRIPT[start..];
    let body = match (rest.find('['), rest.find("\n  ];")) {
        (Some(open), Some(close)) if close > open => &rest[open + 1..close],
        _ => return 0,
    };
    body.lines()
        .filter(|line| line.trim_start().starts_with("['"))
        .count()
}

/// The region names the rail paints, in declaration order.
///
/// `surfaces.md` states the rail's groups by name, and the audit found it
/// naming a taxonomy the face no longer uses. Exposing the names lets the
/// doc-truth checker compare the words, not only the count.
pub fn rail_regions() -> Vec<String> {
    let body = rail_map();
    let mut names = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = body[cursor..].find("name: '") {
        let open = cursor + found + "name: '".len();
        let close = match body[open..].find('\'') {
            Some(offset) => open + offset,
            None => break,
        };
        names.push(body[open..close].to_string());
        cursor = close;
    }
    names
}

/// The rows under one rail region, by its declaration id, or empty when
/// the region is not declared.
pub fn rail_region_rows(region: &str) -> Vec<String> {
    rows_after(rail_map(), &format!("id: '{region}'"))
}

/// The rows the rail actually renders: every region's rows, the system
/// drawer included.
///
/// The rail is **not** `GROUPS`: since the surface pass the rail paints
/// `RAIL_MAP` while the palette still searches `GROUPS`. Counting the
/// palette's table is the exact confusion the audit found in
/// `surfaces.md`, so this reads `RAIL_MAP` and nothing else.
pub fn rail_rows() -> usize {
    rail_regions()
        .iter()
        .map(|region| rail_region_rows(region).len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rail_and_the_routes_are_read_from_the_shipped_asset() {
        // Not a pinned number: the assertion is that the readers find the
        // declarations at all. A zero here means the shape moved, and the
        // drift rows would then report a clean count of nothing.
        assert!(routes() > 0, "the ROUTES table was not found");
        assert!(rail_rows() > 0, "the RAIL_MAP rows were not found");
    }

    #[test]
    fn every_rail_region_is_named_and_carries_rows() {
        let regions = rail_regions();
        assert!(
            !regions.is_empty(),
            "RAIL_MAP declares no named regions — the reader is on the wrong table"
        );
        for region in &regions {
            assert!(
                !rail_region_rows(region).is_empty(),
                "rail region `{region}` declares no rows"
            );
        }
    }

    #[test]
    fn the_rail_rows_are_the_sum_of_every_declared_region() {
        let regions = rail_regions();
        let summed: usize = regions.iter().map(|r| rail_region_rows(r).len()).sum();
        assert_eq!(
            rail_rows(),
            summed,
            "rail_rows() disagrees with the {regions:?} it reads"
        );
    }

    #[test]
    fn no_rail_row_is_declared_twice() {
        // A duplicate would inflate the count the docs state without
        // changing what the owner sees.
        let mut all: Vec<String> = rail_regions()
            .iter()
            .flat_map(|region| rail_region_rows(region))
            .collect();
        let total = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), total, "a rail row is declared twice: {all:?}");
    }

    #[test]
    fn a_region_that_is_not_declared_reads_empty() {
        // The negative case, so `rows_after` is exercised on a miss and
        // cannot silently return the previous region's list.
        assert!(rail_region_rows("no-such-region").is_empty());
    }
}
