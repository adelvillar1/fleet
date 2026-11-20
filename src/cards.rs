//! The house card width: one number, one home.
//!
//! Every card surface — `calib`, `eval`, `improve`, `judge`, `delegate`,
//! `cron`, `doctor`, `explain`, `lanes`, `route` and their siblings — draws
//! at the same width. Each of them used to declare its own
//! `const CARD_WIDTH: usize = 72`: eighteen copies of one number, so a
//! surface could drift by editing its own copy and nothing would notice
//! (KI-140's class — the card that clips its content).
//!
//! The number lives here now. [`the_house_width_has_one_home`] scans the
//! census and fails if any module declares its own again — the same table
//! the guard-list-inversion plan builds, used here for its first real job.

/// The width every card draws at, in characters.
///
/// Inner content is drawn at `CARD_WIDTH - 4`: the box's border and its
/// one-column padding on each side.
pub const CARD_WIDTH: usize = 72;

/// The display width of text, in terminal cells.
///
/// ANSI SGR sequences are colour, not content: they are stripped before
/// measuring, because a card line carries `{DIM}` runs and its visible width
/// is what clips. East Asian wide characters count two cells, combining marks
/// count zero — `unicode-width` decides, not `chars().count()`, which is how
/// KI-140's cards clipped a CJK line while every length check passed.
pub fn width_of(text: &str) -> usize {
    let mut total = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip to the terminator of an escape sequence: CSI ... final
            // byte in 0x40..=0x7e.
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        total += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
    }
    total
}

/// Wrap `text` to `width` display cells, never truncating it.
///
/// Splits on spaces where it can and hard-splits a word that cannot fit on a
/// line of its own (an id, a URL). Returns at least one line, so a caller can
/// always draw a row. A card that truncates silently drops the tail of the
/// owner's text; a card that wraps shows all of it.
pub fn fit(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_width = 0usize;
    for word in text.split(' ') {
        let word_width: usize = word
            .chars()
            .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
            .sum();
        let space = usize::from(!line.is_empty());
        if line_width + space + word_width <= width {
            if space == 1 {
                line.push(' ');
            }
            line.push_str(word);
            line_width += space + word_width;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line_width = 0;
        }
        if word_width <= width {
            line.push_str(word);
            line_width = word_width;
            continue;
        }
        // A word wider than the line: hard-split it by cells.
        for c in word.chars() {
            let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if line_width + w > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line_width = 0;
            }
            line.push(c);
            line_width += w;
        }
    }
    lines.push(line);
    lines
}

/// Shorten `text` to at most `width` cells, ending with an ellipsis when it
/// had to lose anything. For a field that cannot wrap — a card's header — so
/// the loss is visible rather than silent: a truncated field says "…".
pub fn clip(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if width_of(text) <= width {
        return text.to_string();
    }
    let budget = width.saturating_sub(1);
    let mut out = String::new();
    let mut used = 0usize;
    for c in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + w > budget {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('\u{2026}');
    out
}

/// The rule a card draws between its corners: `width` cells of `─`.
///
/// Every module used to spell this itself — `"\u{2500}".repeat(n)`, 44 sites
/// across 17 modules — so the character, and the arithmetic that decides how
/// much of it to draw, lived in seventeen places. `the_rule_has_one_home`
/// refuses a new one.
pub fn rule(width: usize) -> String {
    "\u{2500}".repeat(width.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testcensus::MODULES;

    /// One home for the house width. A module that declares its own
    /// `CARD_WIDTH` is a surface that can clip its own card, and the
    /// eighteen that did are why this module exists.
    #[test]
    fn the_house_width_has_one_home() {
        let offenders: Vec<&str> = MODULES
            .iter()
            .filter(|(name, source)| *name != "cards.rs" && source.contains("const CARD_WIDTH"))
            .map(|(name, _)| *name)
            .collect();
        assert!(
            offenders.is_empty(),
            "modules declaring their own CARD_WIDTH — import `crate::cards::CARD_WIDTH` \
             instead: {offenders:?}"
        );
    }

    /// The measuring basic: colour is not width, CJK is two cells,
    /// combining marks are none.
    #[test]
    fn width_counts_cells_not_chars() {
        assert_eq!(width_of("abc"), 3);
        assert_eq!(width_of("\u{1b}[2mab\u{1b}[0m"), 2);
        assert_eq!(width_of("\u{65e5}\u{672c}"), 4);
        assert_eq!(width_of("e\u{301}"), 1);
    }

    /// The property the cards need: whatever the fixture, every line fits.
    /// The fixtures are the ones that broke cards — a 90-character id, CJK,
    /// emoji, and combining marks — across every width a pane takes.
    #[test]
    fn every_wrapped_line_fits_its_width() {
        let id = "a".repeat(90);
        let fixtures = [
            id.as_str(),
            "\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}",
            "\u{1f680}\u{1f680}\u{1f680}\u{1f680}",
            "e\u{301}e\u{301}e\u{301}e\u{301}",
            "provider acme-dev with a very long display name that goes on and on",
        ];
        for width in 20..=120usize {
            for fixture in fixtures {
                for line in fit(fixture, width) {
                    let cells = width_of(&line);
                    assert!(
                        cells <= width,
                        "width {width}: {cells} cells in {line:?} (fixture {fixture:?})"
                    );
                }
            }
        }
    }

    /// A clipped field says it was clipped.
    #[test]
    fn clipping_leaves_an_ellipsis() {
        assert_eq!(clip("short", 10), "short");
        let long = "a".repeat(40);
        let cut = clip(&long, 10);
        assert_eq!(width_of(&cut), 10);
        assert!(cut.ends_with('\u{2026}'), "{cut:?}");
        assert_eq!(clip("\u{65e5}\u{672c}\u{8a9e}", 4), "\u{65e5}\u{2026}");
    }

    /// Wrapping never drops text: the cells survive the wrap.
    #[test]
    fn wrapping_keeps_the_text() {
        let id = "b".repeat(90);
        let lines = fit(&id, 40);
        let joined: String = lines.join("");
        assert_eq!(joined, id, "no character is lost");
        assert!(lines.len() > 1, "a 90-character word spans lines");
    }

    /// The rule has one home. A module that spells the box character itself
    /// can drift from the house width, and seventeen of them did (45 sites
    /// before `rule` existed).
    #[test]
    fn the_rule_has_one_home() {
        let offenders: Vec<&str> = MODULES
            .iter()
            .filter(|(name, source)| *name != "cards.rs" && source.contains("\"\\u{2500}\".repeat"))
            .map(|(name, _)| *name)
            .collect();
        assert!(
            offenders.is_empty(),
            "modules repeating the box character — call `cards::rule(width)`: {offenders:?}"
        );
    }
}
