//! The loop: the thread that owns the connection and never runs user code.
//!
//! River holds input processing while a manage sequence is open, so a sequence answered by user
//! code is a sequence answered at the speed of whatever that code does. One helper that waits on
//! a person -- a menu, a prompt -- is then not a slow window manager but a deadlocked session:
//! the menu is a layer surface asking for exclusive keyboard focus, which river grants at the end
//! of the manage sequence, so the handler waits for a key the user cannot send. See
//! river-design.md §2.
//!
//! So this thread answers sequences from the last plan the worker published, with a bounded wait
//! for a fresher one. A handler that returns promptly still lands its decisions in the sequence
//! its own key press provoked; one that blocks degrades to exactly the X11 failure mode, which is
//! a window manager that stops managing windows while the session carries on.
use crate::{
    core::{
        bindings::{KeySym, MouseState},
        conn::WinId,
    },
    pure::geometry::{Point, Rect},
    river::{
        RiverEvent,
        plan::{ManagePlan, RenderPlan},
        protocol::{
            self,
            river_layer_shell::{
                river_layer_shell_output_v1::RiverLayerShellOutputV1,
                river_layer_shell_seat_v1::RiverLayerShellSeatV1,
                river_layer_shell_v1::RiverLayerShellV1,
            },
            river_window_management_v1::{
                river_node_v1::RiverNodeV1, river_output_v1::RiverOutputV1,
                river_pointer_binding_v1::RiverPointerBindingV1, river_seat_v1::RiverSeatV1,
                river_window_manager_v1::RiverWindowManagerV1, river_window_v1::RiverWindowV1,
            },
            river_xkb_bindings::{
                river_xkb_binding_v1::RiverXkbBindingV1,
                river_xkb_bindings_seat_v1::RiverXkbBindingsSeatV1,
                river_xkb_bindings_v1::RiverXkbBindingsV1,
            },
        },
        shared::{Shared, WindowFacts},
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Sender},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, trace, warn};
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle, backend::ObjectId, globals::GlobalListContents,
    protocol::wl_registry,
};

/// How long to wait for the worker to publish before answering an ordinary sequence.
///
/// Long enough that a handler doing ordinary work lands in its own sequence, short enough that a
/// handler which is not going to return does not hold the compositor's input for a human span.
const PLAN_WAIT: Duration = Duration::from_millis(20);

/// How long to wait when the batch carried a key press.
///
/// A key press is the one thing that can arm a capture, and arming has to be atomic with the
/// press: river should not look at the next key until the bindings for the sequence in progress
/// are live, or the outer bindings run instead. This is a strong preference and not a guarantee,
/// which is the price of the bound.
const KEY_WAIT: Duration = Duration::from_millis(200);

/// How long the worker may be behind before it is worth saying so.
///
/// Not the wait above: this is not "your handler is slower than a sequence", which a menu makes
/// true for as long as somebody takes to choose, but "something is stuck". A person picking from
/// a menu takes a few seconds; a wedged handler takes forever, and window management is stopped
/// for the duration either way.
const SLOW_HANDLER: Duration = Duration::from_secs(10);

/// What the loop sends the worker.
#[derive(Debug)]
pub(super) enum FromLoop {
    /// An event, and how many events have been sent including this one.
    Event(u64, RiverEvent),
    /// River has handed window management to somebody else, or is shutting down. An orderly end.
    Finished,
    /// The connection died. Somebody should exit non-zero over this.
    Fatal(String),
}

/// Which half of the protocol a sequence river has started accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sequence {
    Manage,
    Render,
}

/// A window river has told us about.
#[derive(Debug)]
pub(super) struct WindowEntry {
    /// `None` once river has closed the window and the object has been destroyed.
    pub(super) obj: Option<RiverWindowV1>,
    pub(super) node: Option<RiverNodeV1>,
    pub(super) facts: WindowFacts,
    /// Whether we have asked river to make this window fullscreen, so that the request is only
    /// made when it changes rather than restated every sequence.
    pub(super) fullscreen_set: bool,
}

/// An output, and the usable area left on it after bars have claimed their exclusive zones.
#[derive(Debug)]
pub(super) struct OutputEntry {
    pub(super) obj: RiverOutputV1,
    pub(super) layer_shell: Option<RiverLayerShellOutputV1>,
    pub(super) position: Option<(i32, i32)>,
    pub(super) dimensions: Option<(u32, u32)>,
    pub(super) non_exclusive_area: Option<Rect>,
    pub(super) removed: bool,
}

impl OutputEntry {
    /// The area of this output penrose should lay windows out in, or `None` until river has told
    /// us where the output is and how big it is.
    fn usable_area(&self) -> Option<Rect> {
        if self.removed {
            return None;
        }

        if let Some(r) = self.non_exclusive_area {
            return Some(r);
        }

        let ((x, y), (w, h)) = (self.position?, self.dimensions?);

        Some(Rect { x, y, w, h })
    }
}

/// A seat, and the binding objects registered on it.
#[derive(Debug)]
pub(super) struct SeatEntry {
    pub(super) obj: RiverSeatV1,
    pub(super) xkb: Option<RiverXkbBindingsSeatV1>,
    pub(super) layer_shell: Option<RiverLayerShellSeatV1>,
    pub(super) key_bindings: HashMap<KeySym, RiverXkbBindingV1>,
    pub(super) mouse_bindings: HashMap<MouseState, RiverPointerBindingV1>,
    pub(super) pointer: Point,
    pub(super) removed: bool,
}

/// Everything the loop owns.
#[derive(Debug)]
pub(super) struct Loop {
    pub(super) wm: RiverWindowManagerV1,
    pub(super) xkb: RiverXkbBindingsV1,
    pub(super) layer_shell: RiverLayerShellV1,
    pub(super) qh: QueueHandle<Loop>,

    pub(super) windows: HashMap<WinId, WindowEntry>,
    /// Wayland object ids are recycled after `wl_display.delete_id`, so a [WinId] is a counter of
    /// our own rather than an object id: reusing river's would let a stale id name a live window.
    pub(super) by_object: HashMap<ObjectId, WinId>,
    pub(super) next_win_id: u32,
    pub(super) outputs: Vec<OutputEntry>,
    pub(super) seats: Vec<SeatEntry>,

    /// The last plan the worker published, re-affirmed as often as river asks for it.
    pub(super) manage: ManagePlan,
    pub(super) render: RenderPlan,
    pub(super) border_width: u32,

    /// Input routing is loop state, not the worker's: which bindings are live has to be coherent
    /// with the key press that river matched against them, and `ate_unbound_key` arrives here.
    pub(super) grabbed_keys: Vec<KeySym>,
    pub(super) grabbed_mouse: Vec<MouseState>,
    pub(super) capture_continuations: Vec<KeySym>,
    pub(super) capture_armed: bool,

    pub(super) shared: Arc<Shared>,
    tx: Sender<FromLoop>,
    /// How many events have been sent to the worker.
    sent: u64,
    /// Whether the batch just delivered carried a key press, which is the one thing that can arm
    /// a capture and so wants the longer wait.
    batch_had_key: bool,
    /// The last event sent, and since when the worker has been behind: together they turn a
    /// mysterious pause into a line in the log naming what is running.
    last_event: Option<String>,
    behind_since: Option<Instant>,
    warned_slow: bool,

    pub(super) pending_sequence: Option<Sequence>,
    /// Windows river has closed, to be forgotten once the worker has handled the closure.
    pending_purge: Vec<(u64, WinId)>,
    /// Windows river has told us about but the worker has not been told about.
    new_windows: Vec<WinId>,
    screens_changed: bool,
    /// Whether a layer surface holds keyboard focus, in which case river ignores ours.
    pub(super) focus_is_exclusive: bool,
    pub(super) finished: bool,
}

impl Loop {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        wm: RiverWindowManagerV1,
        xkb: RiverXkbBindingsV1,
        layer_shell: RiverLayerShellV1,
        qh: QueueHandle<Loop>,
        shared: Arc<Shared>,
        tx: Sender<FromLoop>,
    ) -> Self {
        Self {
            wm,
            xkb,
            layer_shell,
            qh,
            windows: HashMap::new(),
            by_object: HashMap::new(),
            next_win_id: 1, // 0 is the root sentinel
            outputs: Vec::new(),
            seats: Vec::new(),
            manage: ManagePlan::default(),
            render: RenderPlan::default(),
            border_width: 0,
            grabbed_keys: Vec::new(),
            grabbed_mouse: Vec::new(),
            capture_continuations: Vec::new(),
            capture_armed: false,
            shared,
            tx,
            sent: 0,
            batch_had_key: false,
            last_event: None,
            behind_since: None,
            warned_slow: false,
            pending_sequence: None,
            pending_purge: Vec::new(),
            new_windows: Vec::new(),
            screens_changed: false,
            focus_is_exclusive: false,
            finished: false,
        }
    }

    /// Whether river has started a sequence we have not answered, which is how startup knows the
    /// initial state has all arrived.
    pub(super) fn has_pending_sequence(&self) -> bool {
        self.pending_sequence.is_some()
    }

    pub(super) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(super) fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Hand an event to the worker. Named for the callers in `bindings`, which are dispatch
    /// callbacks rather than methods here.
    pub(super) fn send_event(&mut self, e: RiverEvent) {
        self.send(e);
    }

    /// Own the connection until river is done with us.
    pub(super) fn run(mut self, conn: Connection, mut queue: EventQueue<Loop>) {
        loop {
            // Answering comes first, because startup hands this thread a sequence that has
            // already started: river sends its initial state and a manage_start before anything
            // else, and it will send nothing further until that sequence is finished. Dispatching
            // first would be waiting for an event that cannot arrive.
            if let Some(seq) = self.pending_sequence.take() {
                self.answer(seq);

                if let Err(e) = conn.flush() {
                    error!(%e, "unable to flush the wayland connection");
                }
            }

            self.purge_handled();

            if self.finished {
                let _ = self.tx.send(FromLoop::Finished);
                return;
            }

            if let Err(e) = queue.blocking_dispatch(&mut self) {
                let reason = describe_fatal(e);
                error!(%reason, "river connection lost");
                self.shared.set_fatal(reason.clone());
                let _ = self.tx.send(FromLoop::Fatal(reason));
                return;
            }
        }
    }

    /// Answer a sequence, giving the worker a bounded chance to publish first.
    fn answer(&mut self, seq: Sequence) {
        let wait = if self.batch_had_key {
            KEY_WAIT
        } else {
            PLAN_WAIT
        };
        self.batch_had_key = false;

        if !self.shared.wait_for(self.sent, wait) {
            // Answering stale is the point of the bound -- it is what a blocking handler degrades
            // to -- but it does mean whatever that handler decides lands a sequence late, which
            // for a key press is the binding atomicity §5 wants. Debug rather than a warning:
            // this is true for every sequence while a menu is open, which is not a fault.
            debug!(
                ?seq,
                ?wait,
                event = self.last_event.as_deref().unwrap_or("-"),
                "answering with the plan we already have: the worker is still busy"
            );
            self.note_behind();
        } else {
            self.behind_since = None;
            self.warned_slow = false;
        }

        let ops = {
            let mut p = self.shared.published();
            self.manage = p.manage.clone();
            self.render = p.render.clone();
            std::mem::take(&mut p.ops)
        };

        match seq {
            Sequence::Manage => self.transmit_manage(ops),
            Sequence::Render => {
                // Ops are manage state to a one, so a render sequence cannot carry them. Putting
                // them back is what keeps a close from being dropped when a render sequence
                // happens to be the one that picked the plan up.
                if !ops.is_empty() {
                    self.shared.published().ops.splice(0..0, ops);
                    self.wm.manage_dirty();
                }
                self.transmit_render();
            }
        }
    }

    /// Say so, once, when the worker has been busy long enough that the session has noticed.
    fn note_behind(&mut self) {
        let since = *self.behind_since.get_or_insert_with(Instant::now);
        let elapsed = since.elapsed();

        if elapsed > SLOW_HANDLER && !self.warned_slow {
            self.warned_slow = true;
            warn!(
                ?elapsed,
                event = self.last_event.as_deref().unwrap_or("-"),
                "a handler has been running long enough to stop window management: \
                 windows are not being placed until it returns"
            );
        }
    }

    /// Forget windows the worker has finished hearing about.
    ///
    /// Not when river closes them: the worker is still going to ask this window's geometry while
    /// it unmanages it, and a window that vanishes underneath that is an error where a window
    /// that is merely dead is not.
    fn purge_handled(&mut self) {
        let handled = self.shared.published().handled;
        let (done, waiting): (Vec<_>, Vec<_>) = self
            .pending_purge
            .drain(..)
            .partition(|&(seq, _)| seq <= handled);
        self.pending_purge = waiting;

        if done.is_empty() {
            return;
        }

        let mut view = self.shared.view();
        for (_, id) in done {
            debug!(%id, "forgetting closed window");
            if let Some(win) = self.windows.remove(&id)
                && let Some(obj) = win.obj.as_ref()
            {
                self.by_object.remove(&obj.id());
            }
            view.windows.remove(&id);
        }
    }

    /// Hand an event to the worker.
    fn send(&mut self, e: RiverEvent) {
        trace!(event = %e, "sending event to the worker");

        if matches!(e, RiverEvent::KeyPress(_) | RiverEvent::UnboundKey) {
            self.batch_had_key = true;
        }
        if let RiverEvent::WindowClosed(id) = e {
            self.pending_purge.push((self.sent + 1, id));
        }

        self.sent += 1;
        self.last_event = Some(e.to_string());
        if self.tx.send(FromLoop::Event(self.sent, e)).is_err() {
            debug!("the worker is gone");
            self.finished = true;
        }
    }

    /// Turn everything river said in this batch into events for the worker.
    ///
    /// New windows are announced here rather than when the window event arrives so that
    /// everything river had to say about a window -- its app id, title and parent all arrive as
    /// separate events -- is known by the time a manage hook runs.
    fn announce_batch(&mut self) {
        if self.screens_changed {
            self.screens_changed = false;
            self.republish_screens();
            self.send(RiverEvent::ScreenChange);
        }

        for id in std::mem::take(&mut self.new_windows) {
            if self.windows.contains_key(&id) {
                self.send(RiverEvent::WindowOpened(id));
            }
        }
    }

    /// Record that river has started a sequence for us to answer.
    fn start_sequence(&mut self, seq: Sequence) {
        if let Some(pending) = self.pending_sequence {
            // River waits for the finish request before starting another sequence, so this means
            // we have misunderstood the protocol rather than that river has changed its mind.
            error!(
                ?pending,
                ?seq,
                "river started a sequence before we answered the last one"
            );
        }

        trace!(?seq, "sequence started");
        self.pending_sequence = Some(seq);
        self.announce_batch();
    }

    /// The window object for an id, if river has not closed it.
    pub(super) fn live_window(&self, id: WinId) -> Option<&RiverWindowV1> {
        self.windows.get(&id)?.obj.as_ref()
    }

    /// The render list node for an id, if river has not closed the window.
    pub(super) fn live_node(&self, id: WinId) -> Option<&RiverNodeV1> {
        self.windows.get(&id)?.node.as_ref()
    }

    fn win_id(&self, obj: &RiverWindowV1) -> Option<WinId> {
        self.by_object.get(&obj.id()).copied()
    }

    pub(super) fn for_each_seat(&self, f: impl Fn(&SeatEntry)) {
        self.seats.iter().filter(|s| !s.removed).for_each(f);
    }

    /// The output a window has been laid out on, which is the one river should fullscreen it to.
    pub(super) fn output_for(&self, id: WinId) -> Option<&RiverOutputV1> {
        let p = self.render.positions.get(&id)?;

        self.outputs
            .iter()
            .find(|o| o.usable_area().is_some_and(|r| r.contains_point(*p)))
            .map(|o| &o.obj)
    }

    /// The layer shell object for the first usable output, which is where layer surfaces that do
    /// not ask for an output themselves will be placed.
    pub(super) fn default_layer_output(&self) -> Option<&RiverLayerShellOutputV1> {
        self.outputs
            .iter()
            .find(|o| o.usable_area().is_some())?
            .layer_shell
            .as_ref()
    }

    /// Publish the usable areas, sorted, for the worker to hand penrose as screens.
    fn republish_screens(&self) {
        let mut screens: Vec<Rect> = self
            .outputs
            .iter()
            .filter_map(|o| o.usable_area())
            .collect();

        // River makes no promise about the order it announces outputs in.
        screens.sort_by_key(|r| (r.x, r.y));
        self.shared.view().screens = screens;
    }

    /// Update what the worker can see of a window.
    fn with_facts(&mut self, id: WinId, f: impl FnOnce(&mut WindowFacts)) {
        if let Some(win) = self.windows.get_mut(&id) {
            f(&mut win.facts);
            let facts = win.facts.clone();
            self.shared.view().windows.insert(id, facts);
        }
    }
}

/// A dead connection is not something to carry on from: every subsequent read fails, so a loop
/// that kept going would spin logging the same error. River keeps clients alive when we
/// disconnect, which is what makes stopping here recoverable rather than destructive.
fn describe_fatal(e: wayland_client::DispatchError) -> String {
    if let wayland_client::DispatchError::Backend(
        wayland_client::backend::WaylandError::Protocol(p),
    ) = &e
    {
        error!(
            interface = %p.object_interface,
            object = p.object_id,
            code = p.code,
            message = %p.message,
            "fatal river protocol error"
        );

        return format!(
            "protocol error on {} (object {}, code {}): {}",
            p.object_interface, p.object_id, p.code, p.message
        );
    }

    format!("wayland error: {e}")
}

// --- wayland dispatch ---

impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for Loop {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // River's globals are all present when we bind and none of them come and go.
    }
}

impl wayland_client::Dispatch<RiverWindowManagerV1, ()> for Loop {
    wayland_client::event_created_child!(Loop, RiverWindowManagerV1, [
        protocol::river_window_management_v1::river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
        protocol::river_window_management_v1::river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
        protocol::river_window_management_v1::river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ()),
    ]);

    fn event(
        l: &mut Self,
        _: &RiverWindowManagerV1,
        event: protocol::river_window_management_v1::river_window_manager_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_window_manager_v1::Event;

        match event {
            Event::ManageStart => l.start_sequence(Sequence::Manage),
            Event::RenderStart => l.start_sequence(Sequence::Render),

            Event::Unavailable => {
                error!(
                    "river is not offering window management: another window manager is running"
                );
                l.finished = true;
            }

            Event::Finished => {
                info!("river has finished with the window manager");
                l.finished = true;
            }

            Event::Window { id } => {
                let win_id = WinId(l.next_win_id);
                l.next_win_id += 1;

                let node = id.get_node(qh, ());
                debug!(%win_id, "new window");
                l.by_object.insert(id.id(), win_id);
                l.windows.insert(
                    win_id,
                    WindowEntry {
                        obj: Some(id),
                        node: Some(node),
                        facts: WindowFacts::default(),
                        fullscreen_set: false,
                    },
                );
                l.shared
                    .view()
                    .windows
                    .insert(win_id, WindowFacts::default());
                l.new_windows.push(win_id);
            }

            Event::Output { id } => {
                debug!("new output");
                let layer_shell = Some(l.layer_shell.get_output(&id, qh, ()));
                l.outputs.push(OutputEntry {
                    obj: id,
                    layer_shell,
                    position: None,
                    dimensions: None,
                    non_exclusive_area: None,
                    removed: false,
                });
                l.screens_changed = true;
            }

            Event::Seat { id } => {
                debug!("new seat");
                let xkb = Some(l.xkb.get_seat(&id, qh, ()));
                let layer_shell = Some(l.layer_shell.get_seat(&id, qh, ()));
                l.seats.push(SeatEntry {
                    obj: id,
                    xkb,
                    layer_shell,
                    key_bindings: HashMap::new(),
                    mouse_bindings: HashMap::new(),
                    pointer: Point { x: 0, y: 0 },
                    removed: false,
                });
                l.rebind_seats();
            }

            // Nothing is restricted while the session is locked: the lock screen is a layer
            // surface with exclusive focus, so river is already refusing to give windows focus.
            Event::SessionLocked | Event::SessionUnlocked => (),
        }
    }
}

impl wayland_client::Dispatch<RiverWindowV1, ()> for Loop {
    fn event(
        l: &mut Self,
        obj: &RiverWindowV1,
        event: protocol::river_window_management_v1::river_window_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_window_v1::Event;

        let Some(id) = l.win_id(obj) else {
            return; // an event for a window we have already forgotten
        };

        match event {
            Event::Closed => {
                debug!(%id, "window closed");
                if let Some(win) = l.windows.get_mut(&id) {
                    if let Some(node) = win.node.take() {
                        node.destroy();
                    }
                    if let Some(obj) = win.obj.take() {
                        obj.destroy();
                    }
                }
                l.send(RiverEvent::WindowClosed(id));
            }

            Event::Dimensions { width, height } => {
                let d = Some((width.max(0) as u32, height.max(0) as u32));
                l.with_facts(id, |f| f.dimensions = d);
            }

            Event::AppId { app_id } => {
                let announced = !l.new_windows.contains(&id);
                l.with_facts(id, |f| f.app_id = app_id);
                if announced {
                    l.send(RiverEvent::AppId(id));
                }
            }

            Event::Title { title } => {
                let announced = !l.new_windows.contains(&id);
                l.with_facts(id, |f| f.title = title);
                if announced {
                    l.send(RiverEvent::Title(id));
                }
            }

            Event::Parent { parent } => {
                let parent = parent.and_then(|p| l.win_id(&p));
                l.with_facts(id, |f| f.parent = parent);
            }

            Event::Identifier { identifier } => {
                l.with_facts(id, |f| f.identifier = Some(identifier));
            }

            Event::UnreliablePid { unreliable_pid } => {
                let pid = u32::try_from(unreliable_pid).ok();
                l.with_facts(id, |f| f.pid = pid);
            }

            Event::FullscreenRequested { .. } => l.send(RiverEvent::FullscreenRequested(id, true)),
            Event::ExitFullscreenRequested => l.send(RiverEvent::FullscreenRequested(id, false)),

            // Penrose decides sizes itself rather than negotiating them, drives fullscreen from
            // the window manager rather than from the window, and has no concept of maximized,
            // minimized or a window menu. River is free to ignore all of that on our behalf.
            Event::DimensionsHint { .. }
            | Event::DecorationHint { .. }
            | Event::PointerMoveRequested { .. }
            | Event::PointerResizeRequested { .. }
            | Event::ShowWindowMenuRequested { .. }
            | Event::MaximizeRequested
            | Event::UnmaximizeRequested
            | Event::MinimizeRequested
            | Event::PresentationHint { .. }
            | Event::CaptureSessions { .. } => (),
        }
    }
}

impl wayland_client::Dispatch<RiverOutputV1, ()> for Loop {
    fn event(
        l: &mut Self,
        obj: &RiverOutputV1,
        event: protocol::river_window_management_v1::river_output_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_output_v1::Event;

        let Some(output) = l.outputs.iter_mut().find(|o| o.obj.id() == obj.id()) else {
            return;
        };

        match event {
            Event::Position { x, y } => output.position = Some((x, y)),
            Event::Dimensions { width, height } => {
                output.dimensions = Some((width.max(0) as u32, height.max(0) as u32))
            }
            Event::Removed => {
                output.removed = true;
                if let Some(ls) = output.layer_shell.take() {
                    ls.destroy();
                }
                output.obj.destroy();
            }
            Event::WlOutput { .. } | Event::CaptureSessions { .. } => return,
        }

        l.screens_changed = true;
    }
}

impl wayland_client::Dispatch<RiverLayerShellOutputV1, ()> for Loop {
    fn event(
        l: &mut Self,
        obj: &RiverLayerShellOutputV1,
        event: protocol::river_layer_shell::river_layer_shell_output_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_layer_shell::river_layer_shell_output_v1::Event;

        let Event::NonExclusiveArea {
            x,
            y,
            width,
            height,
        } = event;

        // This is the area left after bars have claimed their exclusive zones, in the global
        // coordinate space, which is exactly what penrose wants a screen to be. It is also why
        // the X11 backend's ReserveTop has no counterpart here.
        let found = l
            .outputs
            .iter_mut()
            .find(|o| o.layer_shell.as_ref().is_some_and(|ls| ls.id() == obj.id()));

        if let Some(output) = found {
            output.non_exclusive_area = Some(Rect {
                x,
                y,
                w: width.max(0) as u32,
                h: height.max(0) as u32,
            });
            l.screens_changed = true;
        }
    }
}

impl wayland_client::Dispatch<RiverSeatV1, ()> for Loop {
    fn event(
        l: &mut Self,
        obj: &RiverSeatV1,
        event: protocol::river_window_management_v1::river_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_seat_v1::Event;

        match event {
            Event::PointerPosition { x, y } => {
                if let Some(seat) = l.seats.iter_mut().find(|s| s.obj.id() == obj.id()) {
                    seat.pointer = Point { x, y };
                    l.shared.view().pointer = Point { x, y };
                }
            }

            Event::PointerEnter { window } => {
                if let Some(id) = l.win_id(&window) {
                    l.send(RiverEvent::PointerFocus(id));
                }
            }

            Event::WindowInteraction { window } => {
                if let Some(id) = l.win_id(&window) {
                    l.send(RiverEvent::Interaction(id));
                }
            }

            Event::Removed => {
                if let Some(seat) = l.seats.iter_mut().find(|s| s.obj.id() == obj.id()) {
                    // The layer shell and xkb objects for a seat are made inert when the seat is
                    // removed and should be destroyed with it.
                    seat.removed = true;
                    if let Some(ls) = seat.layer_shell.take() {
                        ls.destroy();
                    }
                    if let Some(xkb) = seat.xkb.take() {
                        xkb.destroy();
                    }
                    seat.obj.destroy();
                }
            }

            // Interactive move and resize are river's own operation rather than something driven
            // from motion events, so nothing here maps onto penrose's floating layer yet.
            Event::WlSeat { .. }
            | Event::PointerLeave
            | Event::ShellSurfaceInteraction { .. }
            | Event::OpDelta { .. }
            | Event::OpRelease => (),
        }
    }
}

impl wayland_client::Dispatch<RiverLayerShellSeatV1, ()> for Loop {
    fn event(
        l: &mut Self,
        _: &RiverLayerShellSeatV1,
        event: protocol::river_layer_shell::river_layer_shell_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_layer_shell::river_layer_shell_seat_v1::Event;

        // While a layer surface holds exclusive focus -- a lock screen, or a menu -- river
        // ignores everything we say about focus. Knowing that is what lets us avoid fighting it.
        l.focus_is_exclusive = matches!(event, Event::FocusExclusive);
    }
}

wayland_client::delegate_noop!(Loop: ignore RiverNodeV1);
wayland_client::delegate_noop!(Loop: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(Loop: ignore RiverLayerShellV1);
