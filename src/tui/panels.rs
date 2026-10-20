//! The knowledge panels' reads. They run on their own thread — the
//! library scan and the archive replay are too heavy for the event
//! loop — and the same `boards` readers the browser face uses, so the
//! two surfaces cannot disagree about what the machine holds.

use std::path::Path;

use crate::boards;
use crate::tui::app::{CalibrationData, PanelData, PanelKind};

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
        PanelKind::Delegation => Ok(PanelData::Delegation(boards::read_delegation(
            home, session,
        ))),
    }
}
