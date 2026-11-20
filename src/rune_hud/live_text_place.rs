//! Where the live text plate starts: the HUD's own plate records it, so a
//! line of provisional words sits just past the HUD on its screen.

use std::sync::Mutex;

use rune::ScreenEdge;

#[cfg(not(target_os = "macos"))]
use super::dot_scope::PLATE_HEIGHT;
#[cfg(not(target_os = "macos"))]
use super::{BOTTOM_OFFSET, TOP_OFFSET, WINDOW};

/// The live text's gap from the HUD's plate.
pub(super) const GAP: f32 = 6.0;

/// Where the live text starts, as of the HUD's last show.
static LIVE_TEXT: Mutex<Option<super::Spot>> = Mutex::new(None);

/// Where the live text plate starts: just past the HUD's plate, on its
/// screen; with the HUD hidden, where its plate would be.
pub(crate) fn live_text_place() -> super::Spot {
    let spot = *LIVE_TEXT.lock().unwrap_or_else(|error| error.into_inner());
    spot.unwrap_or(super::Spot::Edge(ScreenEdge::Top, 40.0))
}

/// Records where this show put the HUD's plate and the live text past it.
#[cfg(not(target_os = "macos"))]
pub(super) fn set(edge: ScreenEdge, margin: f32) {
    let past_plate = margin + (WINDOW.1 + PLATE_HEIGHT) / 2.0 + GAP;
    remember(super::Spot::Edge(edge, past_plate));
}

/// Records a placement the HUD measured itself (macOS's island).
pub(super) fn remember(spot: super::Spot) {
    *LIVE_TEXT.lock().unwrap_or_else(|error| error.into_inner()) = Some(spot);
}

/// The edge and margin a position puts the live text at, before the plate.
#[cfg(not(target_os = "macos"))]
pub(super) fn edge_of(position: super::IndicatorPosition) -> (ScreenEdge, f32) {
    match position {
        super::IndicatorPosition::Bottom => (ScreenEdge::Bottom, BOTTOM_OFFSET),
        _ => (ScreenEdge::Top, TOP_OFFSET),
    }
}
