//! What the two threads say to each other.
//!
//! The loop owns the connection and never runs user code; the worker owns penrose's state and
//! runs all of it. See river-design.md §2.
//!
//! Worker → loop is the [Published] plan: what the compositor should be told. Loop → worker is
//! the [View]: what the compositor has said. The worker's view is therefore always slightly
//! stale, which is why the loop filters against its own live objects at transmit time rather than
//! trusting the plan.
use crate::{
    core::conn::WinId,
    pure::geometry::{Point, Rect},
    river::plan::{ManagePlan, Op, RenderPlan},
};
use std::{
    collections::HashMap,
    sync::{Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};

/// The plan the worker has published, and how far through the events it has got.
#[derive(Debug, Default)]
pub(super) struct Published {
    pub(super) manage: ManagePlan,
    pub(super) render: RenderPlan,
    /// One-shot effects, drained by the loop rather than restated.
    pub(super) ops: Vec<Op>,
    /// The last event the worker has finished handling. The loop waits for this to catch up
    /// before answering a sequence, so that what it transmits is what the handler decided.
    pub(super) handled: u64,
}

/// What the compositor has told us, as much of it as the worker needs.
#[derive(Debug, Default)]
pub(super) struct View {
    pub(super) windows: HashMap<WinId, WindowFacts>,
    /// Usable areas, sorted left to right then top to bottom.
    pub(super) screens: Vec<Rect>,
    pub(super) pointer: Point,
}

/// Everything the worker can ask about a window.
#[derive(Debug, Default, Clone)]
pub(super) struct WindowFacts {
    pub(super) identifier: Option<String>,
    pub(super) app_id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) parent: Option<WinId>,
    pub(super) pid: Option<u32>,
    pub(super) dimensions: Option<(u32, u32)>,
}

/// The two halves, and the handshake between them.
#[derive(Debug, Default)]
pub(super) struct Shared {
    published: Mutex<Published>,
    /// Signalled when the worker publishes, so the loop's wait ends as soon as it can.
    published_changed: Condvar,
    view: Mutex<View>,
}

impl Shared {
    pub(super) fn publish(
        &self,
        manage: ManagePlan,
        render: RenderPlan,
        ops: Vec<Op>,
        handled: u64,
    ) {
        let mut p = self.published.lock().expect("published plan");

        p.manage = manage;
        p.render = render;
        // Ops accumulate rather than replace: two publishes between sequences must not lose the
        // first one's close.
        p.ops.extend(ops);
        p.handled = handled;
        drop(p);

        self.published_changed.notify_all();
    }

    pub(super) fn published(&self) -> MutexGuard<'_, Published> {
        self.published.lock().expect("published plan")
    }

    /// Wait for the worker to have handled everything up to `handled`, or give up.
    ///
    /// This is what buys back the guarantee that a handler's decisions land in the sequence its
    /// own key press provoked. The wait is bounded because an unbounded one is the deadlock this
    /// design exists to remove: a handler that blocks on a person would hold the compositor's
    /// input forever, and the layer surface it is waiting for would never get keyboard focus.
    pub(super) fn wait_for(&self, handled: u64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut p = self.published.lock().expect("published plan");

        while p.handled < handled {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };

            let (next, timed_out) = self
                .published_changed
                .wait_timeout(p, left)
                .expect("published plan");
            p = next;

            if timed_out.timed_out() {
                return p.handled >= handled;
            }
        }

        true
    }

    pub(super) fn view(&self) -> MutexGuard<'_, View> {
        self.view.lock().expect("compositor view")
    }
}
