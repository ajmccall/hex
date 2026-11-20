//! Dictation-indicator reporting beyond the sink's own two consumers.

use super::DictationSink;
use crate::dictation::indicator::DictationIndicatorEvent;

/// Reports silent API capture transitions so they mask pending mode badges
/// without starting a recording HUD.
pub(super) fn sync_programmatic_indicator(
    recording: bool,
    previous: &mut bool,
    indicator: DictationSink<'_>,
) {
    if recording == *previous {
        return;
    }
    *previous = recording;
    indicator.send(DictationIndicatorEvent::ProgrammaticRecording { recording });
}
