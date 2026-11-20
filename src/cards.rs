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

#[cfg(test)]
mod tests {
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
}
