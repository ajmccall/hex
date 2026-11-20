//! The mode badge the HUD hands to live text: the mode a shortcut latched,
//! published only when it changes so a meter never queues UI work.

use super::Spot;
use super::live_text_place;

/// Last published badge and placement.
#[derive(Default)]
pub(crate) struct ModeBadge {
    mode: Option<String>,
    spot: Option<Spot>,
}

impl ModeBadge {
    /// Shows `mode` beside the HUD at `spot`, or clears it, when either
    /// changed. A shown badge follows the HUD's placement; a hidden HUD has
    /// none.
    pub(crate) fn sync(&mut self, mode: Option<&str>) {
        let spot = live_text_place();
        if self.mode.as_deref() == mode && (mode.is_none() || self.spot == Some(spot)) {
            return;
        }
        crate::rune_ui::live_text::set_mode(mode.map(str::to_owned), spot);
        self.mode = mode.map(str::to_owned);
        self.spot = Some(spot);
    }
}
