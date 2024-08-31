//! Utilities for running scheduled updates to widgets
use crate::bar::widgets::Text;
use penrose::util::spawn_with_args;
use std::{
    cmp::max,
    fmt,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tracing::trace;

/// The minimum allowed interval for an [UpdateSchedule].
pub const MIN_DURATION: Duration = Duration::from_secs(1);

/// For widgets that want to have their content updated periodically by the status bar by calling
/// an external function.
///
/// See [IntervalText] for a simple implementation of this behaviour
pub struct UpdateSchedule {
    pub(crate) next: Instant,
    pub(crate) interval: Duration,
    pub(crate) get_text: Box<dyn Fn() -> Option<String> + Send + 'static>,
    pub(crate) txt: Arc<Mutex<Text>>,
}

impl fmt::Debug for UpdateSchedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateSchedule")
            .field("next", &self.next)
            .field("interval", &self.interval)
            .field("txt", &self.txt)
            .finish()
    }
}

impl UpdateSchedule {
    /// Construct a new [UpdateSchedule] specifying the interval that the [Widget] content should
    /// be updated on and an update function for producing the widget content.
    ///
    /// The updated content will then be stored in the provided `Arc<Mutex<Text>>` for access
    /// within your widget logic.
    pub fn new(
        interval: Duration,
        get_text: Box<dyn Fn() -> Option<String> + Send + 'static>,
        txt: Arc<Mutex<Text>>,
    ) -> Self {
        if interval < MIN_DURATION {
            panic!("UpdateSchedule interval is too small: {interval:?} < {MIN_DURATION:?}");
        }

        Self {
            next: Instant::now(),
            interval,
            get_text,
            txt,
        }
    }

    /// Call our `get_text` function to update the contents of our paired [CronText] and then bump
    /// our `next` time to the next interval point.
    ///
    /// This is gives us behaviour of a consistent interval between invocation end/start but not
    /// necessarily a consistent interval between start/start depending on how long `get_text`
    /// takes to run.
    fn update_text(&mut self) {
        trace!("running UpdateSchedule get_text");
        let s = (self.get_text)();
        trace!(?s, "ouput from running get_text");

        if let Some(s) = s {
            let mut t = match self.txt.lock() {
                Ok(inner) => inner,
                Err(poisoned) => poisoned.into_inner(),
            };
            t.set_text(s);
        }

        let next = self.next + self.interval;
        let now = Instant::now();
        self.next = max(next, now);
        trace!(next = ?self.next, "next update at");
    }
}

/// Run the polling thread for a set of [UpdateSchedule]s and update their contents on
/// their requested intervals.
pub(crate) fn run_update_schedules(mut schedules: Vec<UpdateSchedule>) {
    thread::spawn(move || loop {
        trace!("running UpdateSchedule updates for all pending widgets");
        while schedules[0].next < Instant::now() {
            schedules[0].update_text();
            schedules.sort_by(|a, b| a.next.cmp(&b.next));
        }

        // FIXME: this is a hack at the moment to ensure that an event drops into the main
        // window manager event loop and triggers the `on_event` hook of the status bar.
        let _ = spawn_with_args("xsetroot", &["-name", ""]);

        let interval = schedules[0].next - Instant::now();
        trace!(?interval, "sleeping until next update point");
        thread::sleep(interval);
    });
}
