//! The listener's two consumers of dictation-indicator events.

use crate::dictation::indicator::{DictationIndicatorEvent, DictationIndicatorSender};

/// The HUD and the Caps Lock light see the same dictation events. The HUD
/// channel has one consumer; the light's model is fed beside each send instead
/// of taking that channel over.
#[derive(Clone, Copy)]
pub(super) struct DictationSink<'a> {
    pub(super) hud: Option<&'a DictationIndicatorSender>,
    pub(super) caps_lock: &'a crate::caps_lock_dictation::CapsLockDictation,
}

impl DictationSink<'_> {
    pub(super) fn send(&self, event: DictationIndicatorEvent) {
        if let Some(hud) = self.hud {
            hud.send(event.clone());
        }
        self.caps_lock.observe(event);
    }

    /// One chunk's loudness reaches both consumers: the light even when no
    /// HUD listens, and the HUD exactly as it would have received it.
    pub(super) fn meter(&self, samples: &[f32]) {
        if let Some(event) = DictationIndicatorEvent::meter(samples) {
            self.send(event);
        }
    }

    /// A non-dictation capture (Annotate) moves only the Caps Lock light, not
    /// the dictation HUD.
    #[cfg(hex_capture)]
    pub(super) fn caps_only(&self, event: DictationIndicatorEvent) {
        self.caps_lock.observe(event);
    }
}

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
