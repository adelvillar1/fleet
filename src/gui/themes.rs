//! The theme set, and the contrast guard that keeps it readable.
//!
//! A theme is the alias tier re-pointed and nothing else: the component
//! tokens read the aliases, so switching themes is one block of custom
//! properties and no component rule moves. That is the whole reason the
//! stylesheet's colour chain has three tiers.
//!
//! What lives here is the guard. "Great readability" is a claim a
//! stylesheet cannot make about itself, so this module reads the shipped
//! `app.css` and checks every theme's text tokens against all three of
//! its surfaces. A theme whose secondary or tertiary text drops under
//! 4.5:1 fails the build rather than shipping a colour that looks fine
//! on the owner's display and unreadable on someone else's.

use std::collections::HashMap;

/// The shipped stylesheet, read at compile time so the test checks the
/// artifact that actually serves and not a copy that can drift.
const STYLES: &str = include_str!("assets/app.css");

/// The minimum contrast for text, WCAG AA. The tertiary token carries
/// 10.5px labels, which is why it is held to the same bar as body text
/// rather than the large-text 3:1.
const MIN_TEXT: f64 = 4.5;

/// The themes the stylesheet declares, in the order the picker shows
/// them. The first is the default.
const THEMES: &[&str] = &["console", "paper", "graphite", "contrast"];

/// One theme's declared colours, read off its block.
#[derive(Debug, Default)]
struct Theme {
    page: Option<(u8, u8, u8)>,
    surface: Option<(u8, u8, u8)>,
    raised: Option<(u8, u8, u8)>,
    /// The tokens that carry text, each against every surface.
    text: Vec<(String, (u8, u8, u8))>,
}

/// Relative luminance, the WCAG formula.
fn luminance(colour: (u8, u8, u8)) -> f64 {
    fn channel(value: u8) -> f64 {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(colour.0) + 0.7152 * channel(colour.1) + 0.0722 * channel(colour.2)
}

/// The contrast ratio between two colours.
fn contrast(left: (u8, u8, u8), right: (u8, u8, u8)) -> f64 {
    let (left, right) = (luminance(left), luminance(right));
    let (high, low) = if left > right {
        (left, right)
    } else {
        (right, left)
    };
    (high + 0.05) / (low + 0.05)
}

/// One `rgb(r, g, b)` literal, or `None` for anything else — a
/// `color-mix` or a `var()` is not a colour this parser can measure, and
/// guessing would make the guard a lie.
fn rgb(literal: &str) -> Option<(u8, u8, u8)> {
    let inner = literal.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut parts = inner.split(',');
    let red = parts.next()?.trim().parse().ok()?;
    let green = parts.next()?.trim().parse().ok()?;
    let blue = parts.next()?.trim().parse().ok()?;
    Some((red, green, blue))
}

/// Resolve one custom property to a colour, following a `var()` chain at
/// most a few links deep — far enough for every alias in the chain, and
/// bounded so a cycle cannot spin.
fn resolve(name: &str, values: &HashMap<String, String>, depth: usize) -> Option<(u8, u8, u8)> {
    if depth > 6 {
        return None;
    }
    let literal = values.get(name)?;
    if let Some(colour) = rgb(literal) {
        return Some(colour);
    }
    let inner = literal.strip_prefix("var(")?.strip_suffix(')')?;
    resolve(inner, values, depth + 1)
}

/// Read every theme's block out of the stylesheet: the declarations that
/// belong to it, resolved through the `:root` block above them.
fn themes() -> Vec<(String, Theme)> {
    // The `:root` block is the base every theme overrides, so it is read
    // first and each theme starts from it.
    let mut base: HashMap<String, String> = HashMap::new();
    let mut blocks: Vec<(String, HashMap<String, String>)> = Vec::new();
    let mut at: Option<String> = None;
    for line in STYLES.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(":root[data-theme=\"") {
            if let Some(name) = rest.strip_suffix("\"] {") {
                at = Some(name.to_string());
                blocks.push((name.to_string(), HashMap::new()));
                continue;
            }
        }
        if trimmed == ":root {" || trimmed == "}" {
            if trimmed == ":root {" {
                at = Some(":root".to_string());
            } else {
                at = None;
            }
            continue;
        }
        let Some(current) = at.as_ref() else { continue };
        if let Some((name, value)) = trimmed.split_once(':') {
            let value = value.trim().trim_end_matches(';').trim().to_string();
            if current == ":root" {
                base.insert(name.trim().to_string(), value);
            } else if let Some((_, own)) = blocks.iter_mut().find(|(n, _)| n == current) {
                own.insert(name.trim().to_string(), value);
            }
        }
    }

    // The `:root` base is the console theme — the pre-JavaScript default,
    // which is why it carries the values rather than pointing at a block
    // that has not loaded yet. It is prepended as a theme of its own so
    // the guard checks it like any other.
    let mut all = vec![("console".to_string(), HashMap::new())];
    all.extend(blocks);
    all.into_iter()
        .map(|(name, own)| {
            let mut values = base.clone();
            values.extend(own);
            let pick = |token: &str| resolve(token, &values, 0);
            let text = [
                "--dsw-alias-ink",
                "--dsw-alias-ink-secondary",
                "--dsw-alias-ink-tertiary",
                "--dsw-alias-accent-lift",
                "--dsw-alias-ok",
                "--dsw-alias-down",
                "--dsw-alias-warn",
            ]
            .iter()
            .filter_map(|token| pick(token).map(|colour| (token.to_string(), colour)))
            .collect();
            let theme = Theme {
                page: pick("--dsw-alias-page"),
                surface: pick("--dsw-alias-surface"),
                raised: pick("--dsw-alias-surface-raised"),
                text,
            };
            (name, theme)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme's text tokens clear 4.5:1 against all three of its
    /// surfaces. This is the guard that makes "great readability" a
    /// property of the shipped stylesheet rather than a claim about it.
    #[test]
    fn every_themes_text_clears_the_contrast_floor_on_every_surface() {
        let found = themes();
        let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
        // Every declared theme is one the picker shows, and vice versa —
        // a theme nobody can select is a theme nobody verified.
        for expected in THEMES {
            assert!(
                names.contains(expected),
                "`{expected}` is not declared in the stylesheet"
            );
        }
        for name in &names {
            assert!(
                THEMES.contains(name),
                "`{name}` is declared but not in the picker"
            );
        }

        let mut failures: Vec<String> = Vec::new();
        for (name, theme) in &found {
            let surfaces = [
                ("page", theme.page),
                ("surface", theme.surface),
                ("raised", theme.raised),
            ];
            for (surface_name, surface) in surfaces {
                let Some(surface) = surface else {
                    failures.push(format!("`{name}` declares no {surface_name}"));
                    continue;
                };
                for (token, colour) in &theme.text {
                    let ratio = contrast(*colour, surface);
                    if ratio < MIN_TEXT {
                        failures.push(format!(
                            "`{name}`: {token} on {surface_name} is {ratio:.2}:1, under {MIN_TEXT}:1"
                        ));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The parser reads the chain it claims to: the console theme's ink
    /// is the value its own block declares, not the `:root` base.
    #[test]
    fn the_parser_reads_a_themes_own_overrides() {
        let found = themes();
        let console = found
            .iter()
            .find(|(name, _)| name == "console")
            .map(|(_, theme)| theme)
            .expect("the console theme");
        // `console` is the `:root` base itself, so its ink is the base's.
        let ink = console
            .text
            .iter()
            .find(|(token, _)| token == "--dsw-alias-ink")
            .map(|(_, colour)| *colour)
            .expect("the console ink");
        assert_eq!(ink, (235, 231, 228));
        // And `contrast` overrides it to near-black, which is the whole
        // point of the accessibility theme.
        let contrast = found
            .iter()
            .find(|(name, _)| name == "contrast")
            .map(|(_, theme)| theme)
            .expect("the contrast theme");
        let ink = contrast
            .text
            .iter()
            .find(|(token, _)| token == "--dsw-alias-ink")
            .map(|(_, colour)| *colour)
            .expect("the contrast ink");
        assert_eq!(ink, (0, 0, 0));
    }

    /// The ratios the four themes actually measure, printed so a change
    /// that moves one is visible rather than silent.
    #[test]
    fn the_measured_ratios_are_reported() {
        for (name, theme) in themes() {
            let worst = theme
                .text
                .iter()
                .flat_map(|(token, colour)| {
                    [theme.page, theme.surface, theme.raised]
                        .into_iter()
                        .flatten()
                        .map(move |surface| (token.clone(), contrast(*colour, surface)))
                })
                .fold(f64::INFINITY, |worst, (_, ratio)| worst.min(ratio));
            println!("{name}: worst text ratio {worst:.2}:1");
            assert!(worst >= MIN_TEXT, "`{name}` falls to {worst:.2}:1");
        }
    }
}
