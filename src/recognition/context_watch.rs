//! The listener's foreground context: the monitor's latest snapshot, the
//! page refresh a capture asks for once it is real, and the finish that waits
//! for it briefly instead of using the previous dictation's page.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use color_eyre::Result;

use crate::desktop::events::{EventLog, VoiceEvent, now_ms};
use crate::dictation::context::{CaptureRefresh, ContextMonitor, ContextSnapshot};
use crate::suppression::InputActivity;

/// How long a finished shortcut waits for the page its capture asked to
/// refresh before handing the worker a context with no page.
const REFRESH_WAIT: Duration = Duration::from_secs(1);
/// How often that wait rechecks the completed ticket: a worker stores it
/// just after queueing the snapshot, so a delivered page can arrive a moment
/// before `refreshed_since` sees it.
const REFRESH_POLL: Duration = Duration::from_millis(10);

pub(super) struct ContextWatch {
    monitor: ContextMonitor,
    snapshot: ContextSnapshot,
    activity: InputActivity,
    /// The one refresh this capture owes, if any.
    capture: CaptureRefresh,
    /// An explicit mode owned by the current capture's initiating shortcut.
    processing_mode: Option<String>,
}

impl ContextWatch {
    pub(super) fn start(activity: InputActivity) -> Self {
        Self {
            monitor: ContextMonitor::start(),
            snapshot: ContextSnapshot::default(),
            activity,
            capture: CaptureRefresh::default(),
            processing_mode: None,
        }
    }

    /// Drains every published context into the loop's own snapshot.
    pub(super) fn drain(&mut self, events: &mut EventLog) -> Result<()> {
        while let Ok(next) = self.monitor.updates.try_recv() {
            self.apply(next, events)?;
        }
        Ok(())
    }

    fn apply(&mut self, next: ContextSnapshot, events: &mut EventLog) -> Result<()> {
        if self.snapshot != next {
            self.activity.invalidate();
        }
        let switched_app = self.snapshot.application != next.application;
        self.snapshot = next;
        crate::dictation::vocabulary::focus(&self.snapshot_with_processing_mode());
        if switched_app {
            events.emit(&VoiceEvent::Context {
                timestamp_ms: now_ms(),
                application: self.snapshot.application.clone(),
            })?;
        }
        Ok(())
    }

    /// Sets the processing mode latched by the shortcut that starts this
    /// capture, so vocabulary refreshes and the finished job use the same
    /// selected mode even if the foreground context changes.
    pub(super) fn set_processing_mode(&mut self, mode: Option<String>) {
        self.processing_mode = mode;
        crate::dictation::vocabulary::focus(&self.snapshot_with_processing_mode());
    }

    pub(super) fn clear_processing_mode(&mut self) {
        self.processing_mode = None;
        crate::dictation::vocabulary::focus(&self.snapshot);
    }

    fn snapshot_with_processing_mode(&self) -> ContextSnapshot {
        ContextSnapshot {
            processing_mode: self.processing_mode.clone(),
            ..self.snapshot.clone()
        }
    }

    /// A shortcut's press started a capture. It may still be a provisional
    /// press a chord discards, so nothing reads the page yet.
    pub(super) fn begin(&mut self) {
        self.capture.begin();
    }

    /// The capture became real: a control start (no hold to judge), a hold
    /// past its threshold, or a released microphone finally opening. Ask for
    /// its page now, once per capture.
    pub(super) fn intend(&mut self) {
        let monitor = &self.monitor;
        self.capture.intend(|| monitor.request_refresh());
    }

    /// The capture ended without transcribing: no finish waits for its page.
    pub(super) fn cancel(&mut self) {
        self.capture.cancel();
        self.clear_processing_mode();
    }

    /// The context to hand a finished capture: the refreshed page when it
    /// arrived in time, else the same context without a page rather than the
    /// page of a previous dictation. A capture that finished before it was
    /// real asks for its page here, and a finish with no capture at all (the
    /// loop's own context) returns it unchanged.
    pub(super) fn finish(&mut self, events: &mut EventLog) -> Result<ContextSnapshot> {
        let monitor = &self.monitor;
        let Some(ticket) = self.capture.finish(|| monitor.request_refresh()) else {
            return Ok(self.snapshot_with_processing_mode());
        };
        let deadline = Instant::now() + REFRESH_WAIT;
        loop {
            let refreshed = self.monitor.refreshed_since(ticket);
            // A completed refresh has its snapshot in the slot or already
            // delivered, so this drain takes it or a newer one.
            self.drain(events)?;
            if refreshed {
                return Ok(self.snapshot_with_processing_mode());
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(self.without_page());
            };
            match self
                .monitor
                .updates
                .recv_timeout(remaining.min(REFRESH_POLL))
            {
                Ok(next) => self.apply(next, events)?,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Ok(self.without_page());
                }
            }
        }
    }

    /// Hands a capture the same context with no page, and focuses the
    /// vocabulary there too: the previous dictation's page must not name this
    /// one's mode.
    fn without_page(&self) -> ContextSnapshot {
        let snapshot = ContextSnapshot {
            browser_url: None,
            ..self.snapshot_with_processing_mode()
        };
        crate::dictation::vocabulary::focus(&snapshot);
        snapshot
    }
}
