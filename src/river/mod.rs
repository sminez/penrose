//! A [Conn] implementation for the [river][1] Wayland compositor.
//!
//! River's window management protocol defers position, size, focus, keybindings and decorations
//! to a separate process, which is what makes an ordinary tiling window manager possible on
//! Wayland at all. See `river-design.md` in the repository root for the design this implements
//! and the reasoning behind it.
//!
//! The window manager draws nothing: bars, menus and prompts are separate clients.
//!
//! [1]: https://codeberg.org/river/river
use crate::{
    Color, Error, Result,
    core::{
        Config, State,
        bindings::{KeyBindings, KeySym, MouseBindings, MouseState, dispatch_key, dispatch_mouse},
        conn::{Conn, ConnExt, WinId, manage_without_refresh},
    },
    pure::geometry::{Point, Rect},
};
use plan::{ManagePlan, Op, RenderPlan};
use protocol::{
    river_layer_shell::{
        river_layer_shell_output_v1::RiverLayerShellOutputV1,
        river_layer_shell_seat_v1::RiverLayerShellSeatV1, river_layer_shell_v1::RiverLayerShellV1,
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
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, trace, warn};
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle,
    backend::ObjectId,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};

mod bindings;
mod event;
mod plan;
pub(crate) mod protocol;

pub use event::RiverEvent;

/// The window manager protocol versions this backend is written against.
///
/// The lower bound of each range is what we actually need: `identifier` for restart and
/// `unreliable_pid` come from window management v4 and v2, and `ensure_next_key_eaten` from xkb
/// bindings v2. Refusing to start is better than discovering the gap at the first key sequence.
const WM_VERSIONS: std::ops::RangeInclusive<u32> = 4..=5;
const XKB_VERSIONS: std::ops::RangeInclusive<u32> = 2..=3;
const LAYER_SHELL_VERSIONS: std::ops::RangeInclusive<u32> = 1..=1;

/// How long a handler may run before it is worth complaining about.
///
/// River stalls input while a manage sequence is open, so this is a user visible pause rather
/// than a slow window manager.
const SLOW_HANDLER: Duration = Duration::from_millis(50);

/// The order screen indices are assigned in.
///
/// Penrose indexes screens in whatever order `screen_details` returns them, and river makes no
/// promise about the order it announces outputs in, so the conn sorts them by position. Which end
/// to start from is a preference: `xmonad-contrib`'s `PhysicalScreens` counts right to left.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScreenOrder {
    /// Screen 0 is the leftmost, then top to bottom.
    #[default]
    LeftToRight,
    /// Screen 0 is the rightmost, then bottom to top.
    RightToLeft,
}

/// A [Conn] implementation backed by the river Wayland compositor.
///
/// Everything that mutates state is buffered into a plan rather than sent, because river only
/// accepts state changes inside a sequence which it starts: see [plan] and river-design.md §2.
#[derive(Debug)]
pub struct RiverConn {
    conn: Connection,
    queue: EventQueue<Inner>,
    inner: Inner,
}

/// Which half of the protocol a sequence we have been given accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sequence {
    Manage,
    Render,
}

/// A window river has told us about.
#[derive(Debug)]
struct WindowEntry {
    /// `None` once river has closed the window and the object has been destroyed.
    obj: Option<RiverWindowV1>,
    node: Option<RiverNodeV1>,
    identifier: Option<String>,
    app_id: Option<String>,
    title: Option<String>,
    parent: Option<WinId>,
    pid: Option<u32>,
    dimensions: Option<(u32, u32)>,
    /// Whether we have asked river to make this window fullscreen, so that the request is only
    /// made when it changes rather than restated every sequence.
    fullscreen_set: bool,
}

/// An output, and the usable area left on it after bars have claimed their exclusive zones.
#[derive(Debug)]
struct OutputEntry {
    obj: RiverOutputV1,
    layer_shell: Option<RiverLayerShellOutputV1>,
    position: Option<(i32, i32)>,
    dimensions: Option<(u32, u32)>,
    non_exclusive_area: Option<Rect>,
    removed: bool,
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
struct SeatEntry {
    obj: RiverSeatV1,
    xkb: Option<RiverXkbBindingsSeatV1>,
    layer_shell: Option<RiverLayerShellSeatV1>,
    key_bindings: HashMap<KeySym, RiverXkbBindingV1>,
    mouse_bindings: HashMap<MouseState, RiverPointerBindingV1>,
    pointer: Point,
    removed: bool,
}

/// The connection state that wayland events are dispatched against.
///
/// This is split out from [RiverConn] so that pumping the queue can borrow the queue and the
/// state it dispatches into at the same time.
#[derive(Debug)]
struct Inner {
    wm: RiverWindowManagerV1,
    xkb: RiverXkbBindingsV1,
    layer_shell: RiverLayerShellV1,
    qh: QueueHandle<Inner>,

    windows: HashMap<WinId, WindowEntry>,
    /// Wayland object ids are recycled after `wl_display.delete_id`, so a [WinId] is a counter of
    /// our own rather than an object id: reusing river's would let a stale id name a live window.
    by_object: HashMap<ObjectId, WinId>,
    next_win_id: u32,
    outputs: Vec<OutputEntry>,
    seats: Vec<SeatEntry>,

    manage: ManagePlan,
    render: RenderPlan,
    border_width: u32,

    /// The keys and mouse states penrose has asked us to grab.
    grabbed_keys: HashSet<KeySym>,
    grabbed_mouse: HashSet<MouseState>,
    /// The keys which would continue the sequence in progress, enabled for as long as the
    /// capture is armed.
    capture_continuations: HashSet<KeySym>,

    /// A sequence river has started which we have not yet answered.
    pending_sequence: Option<Sequence>,
    /// Events penrose has not yet been given. A pending sequence is only answered when this is
    /// empty, or the plan we transmit is the one from before penrose handled the event which
    /// caused the sequence: see river-design.md §2.
    pending_events: VecDeque<RiverEvent>,
    /// Windows which closed, to be forgotten once penrose has handled the closure.
    pending_purge: Vec<WinId>,
    /// Windows river has told us about but penrose has not yet been told about.
    new_windows: Vec<WinId>,
    /// Whether an output changed in a way that penrose needs to hear about.
    screens_changed: bool,
    /// Whether `ensure_next_key_eaten` is armed. It is one-shot at river's end, so re-arming an
    /// armed capture would eat a second key.
    capture_armed: bool,
    /// Whether a layer surface holds keyboard focus, in which case river ignores our focus.
    focus_is_exclusive: bool,
    /// Whether the plan holds changes river has not been told about yet.
    ///
    /// River starts a sequence on its own only when something it knows about changes, so a plan
    /// change we made for our own reasons -- a layout message, a workspace switch -- needs
    /// `manage_dirty` to get a sequence to send it in.
    plan_dirty: bool,
    /// Whether `manage_dirty` has been sent for the current plan, so it is asked for once.
    dirty_requested: bool,
    /// The last event handed to penrose and when, for the slow handler warning.
    delivered: Option<(RiverEvent, Instant)>,
    /// Set when river tells us it is done with us, either a hot swap or a compositor shutdown.
    finished: bool,
    finished_delivered: bool,

    screen_order: ScreenOrder,
    /// Which tag to put an existing window back on, keyed by river's window identifier.
    ///
    /// River has no property store, so unlike X11 there is nowhere on a window to record which
    /// workspace it was on. This is how a restart puts them back: see [RiverConn::restore_tags].
    restore_tags: HashMap<String, String>,
}

impl RiverConn {
    /// Connect to river and bind the globals a window manager needs.
    ///
    /// This blocks until river has sent its initial state -- every existing window, output and
    /// seat -- which it always does before the first manage sequence.
    pub fn new() -> Result<Self> {
        let conn = Connection::connect_to_env().map_err(|e| {
            Error::Custom(format!("unable to connect to a wayland compositor: {e}"))
        })?;

        let (globals, mut queue) = registry_queue_init::<Inner>(&conn)
            .map_err(|e| Error::Custom(format!("unable to set up the wayland registry: {e}")))?;
        let qh = queue.handle();

        let wm: RiverWindowManagerV1 = globals
            .bind(&qh, WM_VERSIONS, ())
            .map_err(|e| bind_error("river_window_manager_v1", e))?;
        // Binding the layer shell is not optional even though we draw nothing ourselves: without
        // it river closes every layer surface on sight, so no bars, no notifications, no menus.
        let layer_shell: RiverLayerShellV1 = globals
            .bind(&qh, LAYER_SHELL_VERSIONS, ())
            .map_err(|e| bind_error("river_layer_shell_v1", e))?;
        let xkb: RiverXkbBindingsV1 = globals
            .bind(&qh, XKB_VERSIONS, ())
            .map_err(|e| bind_error("river_xkb_bindings_v1", e))?;

        let mut inner = Inner {
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
            grabbed_keys: HashSet::new(),
            grabbed_mouse: HashSet::new(),
            capture_continuations: HashSet::new(),
            pending_sequence: None,
            pending_events: VecDeque::new(),
            pending_purge: Vec::new(),
            new_windows: Vec::new(),
            screens_changed: false,
            capture_armed: false,
            focus_is_exclusive: false,
            plan_dirty: false,
            dirty_requested: false,
            delivered: None,
            finished: false,
            finished_delivered: false,
            screen_order: ScreenOrder::default(),
            restore_tags: HashMap::new(),
        };

        // River sends a window event for every existing window, and an output and seat event for
        // each of those, before the first manage_start. So waiting for that first sequence is
        // what makes existing_clients and screen_details answerable.
        while inner.pending_sequence.is_none() && !inner.finished {
            queue
                .blocking_dispatch(&mut inner)
                .map_err(|e| Error::Custom(format!("wayland error during startup: {e}")))?;
        }

        if inner.finished {
            return Err(Error::Custom(
                "river is not offering window management to us: is another window manager running?"
                    .to_owned(),
            ));
        }

        info!(
            windows = inner.windows.len(),
            outputs = inner.outputs.len(),
            seats = inner.seats.len(),
            "connected to river"
        );

        Ok(Self { conn, queue, inner })
    }

    /// Index screens from the right rather than from the left.
    pub fn with_screen_order(mut self, order: ScreenOrder) -> Self {
        self.inner.screen_order = order;
        self
    }

    /// Put existing windows back on the workspaces they were on before a restart.
    ///
    /// The map is from river's window identifier to a workspace tag, and is consulted by
    /// `manage_existing_clients`, so it has to be set before `run`. Identifiers are stable across
    /// a window manager restart and are never reused, which is what makes them the key: they
    /// belong to the window rather than to our connection.
    ///
    /// Producing the map is the caller's job, because where it is kept is: see
    /// [RiverConn::window_identifier] and [RiverConn::stop].
    pub fn restore_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.inner.restore_tags = tags;
        self
    }

    /// Ask river to hand window management to somebody else.
    ///
    /// This is how a restart works: river keeps every client alive across the swap, so the
    /// window manager can exit and be replaced without the session noticing. River answers with
    /// `finished`, which arrives as [RiverEvent::Finished] and stops the run loop; `run` then
    /// returns and the caller can exec its new binary.
    ///
    /// Exiting any other way -- a crash, a protocol error -- is not this: it is an unclean
    /// disconnect, and while river should leave the windows alone, only the orderly path is
    /// specified.
    pub fn stop(&mut self) {
        info!("asking river to stop sending us events");
        self.inner.wm.stop();
        self.flush();
    }

    /// Make a window fullscreen, or take it out of fullscreen.
    ///
    /// River has no `_NET_WM_STATE` for a config to set, so this is the river counterpart of the
    /// `toggle_fullscreen` action: bind it, or call it in response to
    /// [RiverEvent::FullscreenRequested] if you want windows to be able to fullscreen themselves.
    pub fn set_fullscreen(&mut self, id: WinId, fullscreen: bool) {
        if fullscreen {
            self.inner.manage.fullscreen.insert(id);
        } else {
            self.inner.manage.fullscreen.remove(&id);
        }
        self.inner.touch();
    }

    /// Whether river has taken window management away from us.
    ///
    /// This is a hot swap to another window manager or the compositor shutting down. River keeps
    /// every client alive across a hot swap, which is what makes restarting recoverable.
    pub fn is_finished(&self) -> bool {
        self.inner.finished
    }

    /// River's identifier for a window, which is stable across a window manager restart.
    pub fn window_identifier(&self, id: WinId) -> Option<&str> {
        self.inner.windows.get(&id)?.identifier.as_deref()
    }
}

fn bind_error(interface: &str, e: wayland_client::globals::BindError) -> Error {
    Error::Custom(format!(
        "river does not offer a usable {interface}: {e}. \
         penrose needs a river new enough to have the window management protocol."
    ))
}

impl Inner {
    /// The window object for an id, if river has not closed it.
    fn live_window(&self, id: WinId) -> Option<&RiverWindowV1> {
        self.windows.get(&id)?.obj.as_ref()
    }

    /// The render list node for an id, if river has not closed the window.
    fn live_node(&self, id: WinId) -> Option<&RiverNodeV1> {
        self.windows.get(&id)?.node.as_ref()
    }

    fn win_id(&self, obj: &RiverWindowV1) -> Option<WinId> {
        self.by_object.get(&obj.id()).copied()
    }

    fn for_each_seat(&self, f: impl Fn(&SeatEntry)) {
        self.seats.iter().filter(|s| !s.removed).for_each(f);
    }

    /// The output a window has been laid out on, which is the one river should fullscreen it to.
    fn output_for(&self, id: WinId) -> Option<&RiverOutputV1> {
        let p = self.render.positions.get(&id)?;

        self.outputs
            .iter()
            .find(|o| o.usable_area().is_some_and(|r| r.contains_point(*p)))
            .map(|o| &o.obj)
    }

    /// The layer shell object for the first usable output, which is where layer surfaces that do
    /// not ask for an output themselves will be placed.
    fn default_layer_output(&self) -> Option<&RiverLayerShellOutputV1> {
        self.outputs
            .iter()
            .find(|o| o.usable_area().is_some())?
            .layer_shell
            .as_ref()
    }

    /// Mark the plan as holding changes river has not seen.
    fn touch(&mut self) {
        self.plan_dirty = true;
    }

    /// Ask river for a manage sequence if the plan is waiting for one.
    ///
    /// Every render sequence is preceded by a manage sequence, so this is how render state that
    /// penrose changed without resizing anything -- a restack, a border colour -- reaches the
    /// screen as well.
    fn request_manage_sequence(&mut self) {
        if !self.plan_dirty || self.dirty_requested {
            return;
        }
        if self.pending_sequence == Some(Sequence::Manage) {
            return; // one is already waiting to be answered
        }

        trace!("asking river for a manage sequence");
        self.wm.manage_dirty();
        self.dirty_requested = true;
    }

    /// Queue an event for penrose.
    fn queue_event(&mut self, e: RiverEvent) {
        trace!(event = %e, "queueing event for penrose");
        self.pending_events.push_back(e);
    }

    /// Turn everything river said in this batch into events for penrose.
    ///
    /// New windows are announced here rather than when the window event arrives so that
    /// everything river had to say about a window -- its app id, title and parent all arrive as
    /// separate events -- is known by the time a manage hook runs.
    fn announce_batch(&mut self) {
        if self.screens_changed {
            self.screens_changed = false;
            self.queue_event(RiverEvent::ScreenChange);
        }

        for id in std::mem::take(&mut self.new_windows) {
            if self.windows.contains_key(&id) {
                self.queue_event(RiverEvent::WindowOpened(id));
            }
        }
    }

    /// Forget windows penrose has finished hearing about.
    fn purge_closed(&mut self) {
        for id in std::mem::take(&mut self.pending_purge) {
            if let Some(win) = self.windows.remove(&id) {
                debug!(%id, "forgetting closed window");
                if let Some(obj) = win.obj.as_ref() {
                    self.by_object.remove(&obj.id());
                }
            }

            self.manage.dimensions.remove(&id);
            self.manage.fullscreen.remove(&id);
            self.manage.initial_props.remove(&id);
            self.render.positions.remove(&id);
            self.render.borders.remove(&id);
            self.render.visible.remove(&id);
            self.render.order.retain(|&o| o != id);

            if self.manage.focus == Some(id) {
                self.manage.focus = None;
            }
        }
    }

    /// Answer a sequence river has started, if penrose owes us nothing.
    fn answer_sequence(&mut self) {
        if !self.pending_events.is_empty() {
            return;
        }

        match self.pending_sequence.take() {
            Some(Sequence::Manage) => self.transmit_manage(),
            Some(Sequence::Render) => self.transmit_render(),
            None => (),
        }
    }

    /// Warn if penrose took long enough over an event that river's input was held up.
    ///
    /// A handler may not block: river holds input processing while a manage sequence is open, so
    /// a handler which waits on something turns a frozen window manager into a frozen session.
    /// The one blocking helper penrose has -- `DMenu::build_menu` -- is kept out of a river
    /// config by its `XConn` bound, but a config can always write its own, and a rule nobody
    /// checks is a rule that gets broken.
    ///
    /// This reports after the fact rather than while it is happening: catching it live needs a
    /// timer in the event loop, which is the wakeup path a non-blocking DMenu needs anyway.
    fn check_handler_time(&mut self) {
        let Some((event, at)) = self.delivered.take() else {
            return;
        };

        let elapsed = at.elapsed();
        if elapsed > SLOW_HANDLER && self.pending_sequence.is_some() {
            warn!(
                ?elapsed,
                %event,
                "handler held a manage sequence open: river's input was stalled for the duration"
            );
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
}

impl Conn for RiverConn {
    type Event = RiverEvent;
    type State = ();
    type KeyBindingKey = KeySym;

    fn initial_state(&mut self) -> Self::State {}

    /// River has no root window. The only use of this is `set_focus`'s fallback, which becomes
    /// `clear_focus`, so a sentinel that names no window is exactly right.
    fn root(&mut self) -> WinId {
        WinId(0)
    }

    fn next_event(&mut self) -> Result<Self::Event> {
        loop {
            // Anything penrose is owed goes out before a sequence is answered, so that the plan
            // river sees is the one penrose left behind rather than the one it started with.
            if let Some(e) = self.inner.pending_events.pop_front() {
                if let RiverEvent::WindowClosed(id) = e {
                    self.inner.pending_purge.push(id);
                }

                self.inner.delivered = Some((e.clone(), Instant::now()));

                return Ok(e);
            }

            self.inner.check_handler_time();
            self.inner.purge_closed();
            self.inner.answer_sequence();

            if self.inner.finished && !self.inner.finished_delivered {
                self.inner.finished_delivered = true;
                return Ok(RiverEvent::Finished);
            }

            self.queue
                .blocking_dispatch(&mut self.inner)
                .map_err(fatal_wayland_error)?;
        }
    }

    fn handle_event(
        &mut self,
        evt: Self::Event,
        key_bindings: &mut KeyBindings<Self>,
        mouse_bindings: &mut MouseBindings<Self>,
        state: &mut State<Self>,
    ) -> Result<()> {
        use RiverEvent::*;

        match evt {
            KeyPress(key) => dispatch_key(key, key_bindings, state, self)?,
            UnboundKey => bindings::abandon_key_sequence(state, self)?,
            MouseEvent(e) => dispatch_mouse(e, mouse_bindings, state, self)?,

            WindowOpened(id) => {
                if !state.client_set.contains(&id) && self.client_should_be_managed(id) {
                    self.manage(id, state)?;
                }
            }
            WindowClosed(id) => self.unmanage(id, state)?,

            ScreenChange => {
                let rects = self.screen_details()?;
                info!(?rects, "screens changed");
                state.client_set.update_screens(rects)?;
                self.refresh(state)?;
            }

            PointerFocus(id) | Interaction(id) => {
                if state.client_set.contains(&id) && state.config.focus_follow_mouse {
                    self.modify_and_refresh(state, |cs| cs.focus_client(&id))?;
                }
            }

            Finished => {
                info!("river has finished with us");
                state.running = false;
            }

            // Nothing is done with these by default: a config that wants to react to them can do
            // so from a hook. Fullscreen in particular is deliberately not automatic, matching
            // the X11 backend, where honouring _NET_WM_STATE is an opt-in extension.
            Title(_) | AppId(_) | FullscreenRequested(_, _) => (),
        }

        Ok(())
    }

    fn flush(&mut self) {
        // Wayland requests have no replies, so the only failure a write can have is a fatal one,
        // which arrives as wl_display.error and makes the next read fail: next_event is the
        // error channel.
        self.inner.request_manage_sequence();

        if let Err(e) = self.conn.flush() {
            error!(%e, "unable to flush the wayland connection");
        }
    }

    fn capture_next_key(&mut self, continuations: &[KeySym]) -> Result<()> {
        self.inner.capture(continuations);

        Ok(())
    }

    fn cancel_capture_next_key(&mut self) -> Result<()> {
        self.inner.cancel_capture();

        Ok(())
    }

    fn grab(&mut self, keys: &[KeySym], mouse_states: &[MouseState]) -> Result<()> {
        self.inner.grab(keys, mouse_states);

        Ok(())
    }

    fn existing_clients(&mut self) -> Result<Vec<WinId>> {
        let mut ids: Vec<WinId> = self.inner.windows.keys().copied().collect();
        ids.sort();

        Ok(ids)
    }

    /// River has no property store, so unlike X11 there is nothing to read back off a window:
    /// where each one belongs comes from [RiverConn::restore_tags], which a restart fills in from
    /// a state file it wrote before asking river to swap us out.
    fn manage_existing_clients(&mut self, state: &mut State<Self>) -> Result<()> {
        let known: Vec<String> = state.client_set.ordered_tags();

        for id in self.existing_clients()? {
            if !state.client_set.contains(&id) && self.client_should_be_managed(id) {
                let title = self.client_title(id)?;
                // A tag that no longer exists in the config would be a workspace nothing can
                // reach, so an unknown one falls back to the current workspace.
                let tag = self
                    .inner
                    .windows
                    .get(&id)
                    .and_then(|w| w.identifier.as_ref())
                    .and_then(|i| self.inner.restore_tags.get(i))
                    .filter(|t| known.contains(t))
                    .cloned();

                info!(%id, %title, ?tag, "managing existing client");
                manage_without_refresh(id, tag.as_deref(), state, self)?;
            }
        }

        self.refresh(state)
    }

    fn screen_details(&mut self) -> Result<Vec<Rect>> {
        let mut rects: Vec<Rect> = self
            .inner
            .outputs
            .iter()
            .filter_map(|o| o.usable_area())
            .collect();

        if rects.is_empty() {
            return Err(Error::NoScreens);
        }

        // River makes no promise about the order it announces outputs in, so they are sorted by
        // position.
        rects.sort_by_key(|r| (r.x, r.y));
        if self.inner.screen_order == ScreenOrder::RightToLeft {
            rects.reverse();
        }

        Ok(rects)
    }

    fn cursor_position(&mut self) -> Result<Point> {
        Ok(self
            .inner
            .seats
            .iter()
            .find(|s| !s.removed)
            .map(|s| s.pointer)
            .unwrap_or(Point { x: 0, y: 0 }))
    }

    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()> {
        // River warps in absolute coordinates where penrose warps within a window, so the
        // planned position of that window is what makes the two the same request.
        let origin = if id == WinId(0) {
            Point { x: 0, y: 0 }
        } else {
            match self.inner.render.positions.get(&id) {
                Some(&p) => p,
                None => return Ok(()), // nowhere to warp to yet
            }
        };

        self.inner.manage.ops.push(Op::WarpPointer(Point {
            x: origin.x + x as i32,
            y: origin.y + y as i32,
        }));
        self.inner.touch();

        Ok(())
    }

    /// A window's size is manage state and its position is render state, so this one call feeds
    /// both halves of the plan and lands on screen over two sequences.
    fn position_client(&mut self, id: WinId, r: Rect) -> Result<()> {
        let dimensions = (r.w, r.h);
        let position = Point { x: r.x, y: r.y };

        if self.inner.manage.dimensions.insert(id, dimensions) != Some(dimensions)
            || self.inner.render.positions.insert(id, position) != Some(position)
        {
            self.inner.touch();
        }

        Ok(())
    }

    fn show_client(&mut self, id: WinId, _: &mut State<Self>) -> Result<()> {
        if self.inner.render.visible.insert(id) {
            self.inner.touch();
        }

        Ok(())
    }

    fn hide_client(&mut self, id: WinId, _: &mut State<Self>) -> Result<()> {
        if self.inner.render.visible.remove(&id) {
            self.inner.touch();
        }

        Ok(())
    }

    /// There is no withdrawn state to set: river tells us a window has closed and destroys it.
    fn withdraw_client(&mut self, _: WinId) -> Result<()> {
        Ok(())
    }

    fn kill_client(&mut self, id: WinId) -> Result<()> {
        // A close is not restated: re-sending it would ask a second window to close.
        self.inner.manage.ops.push(Op::Close(id));
        self.inner.touch();

        Ok(())
    }

    fn focus_client(&mut self, id: WinId) -> Result<()> {
        let focus = if id == WinId(0) { None } else { Some(id) };

        if self.inner.manage.focus != focus {
            self.inner.manage.focus = focus;
            self.inner.touch();
        }

        Ok(())
    }

    fn client_geometry(&mut self, id: WinId) -> Result<Rect> {
        let win = self
            .inner
            .windows
            .get(&id)
            .ok_or(Error::UnknownClient(id))?;
        let (w, h) = win.dimensions.unwrap_or((0, 0));
        let p = self
            .inner
            .render
            .positions
            .get(&id)
            .copied()
            .unwrap_or(Point { x: 0, y: 0 });

        Ok(Rect {
            x: p.x,
            y: p.y,
            w,
            h,
        })
    }

    fn client_title(&mut self, id: WinId) -> Result<String> {
        Ok(self
            .inner
            .windows
            .get(&id)
            .and_then(|w| w.title.clone())
            .unwrap_or_default())
    }

    /// River is explicit that this is unreliable, and names the event accordingly.
    fn client_pid(&mut self, id: WinId) -> Option<u32> {
        self.inner.windows.get(&id)?.pid
    }

    fn client_should_float(&mut self, id: WinId, floating_classes: &[String]) -> bool {
        match self.inner.windows.get(&id).and_then(|w| w.app_id.as_ref()) {
            Some(app_id) => floating_classes.iter().any(|c| c == app_id),
            None => false,
        }
    }

    /// River only tells the window manager about windows it should manage, so the only windows
    /// rejected here are ones it has already closed.
    fn client_should_be_managed(&mut self, id: WinId) -> bool {
        self.inner.live_window(id).is_some()
    }

    fn client_is_fullscreen(&mut self, id: WinId) -> bool {
        self.inner.manage.fullscreen.contains(&id)
    }

    fn client_transient_parent(&mut self, id: WinId) -> Option<WinId> {
        self.inner.windows.get(&id)?.parent
    }

    fn set_client_border_color(&mut self, id: WinId, color: impl Into<Color>) -> Result<()> {
        let color = color.into();
        if self.inner.render.borders.insert(id, color) != Some(color) {
            self.inner.touch();
        }

        Ok(())
    }

    fn set_initial_properties(&mut self, id: WinId, config: &Config<Self>) -> Result<()> {
        self.inner.border_width = config.border_width;
        self.inner.manage.initial_props.insert(id);
        self.inner.touch();

        Ok(())
    }

    fn restack<'a, I>(&mut self, ids: I) -> Result<()>
    where
        WinId: 'a,
        I: Iterator<Item = &'a WinId>,
    {
        let order: Vec<WinId> = ids.copied().collect();
        if order != self.inner.render.order {
            self.inner.render.order = order;
            self.inner.touch();
        }

        Ok(())
    }
}

/// A dead wayland connection is not something to carry on from: every subsequent read fails, so
/// the run loop would spin logging the same error. River keeps clients alive when we disconnect,
/// which is what makes exiting here recoverable rather than destructive -- a supervisor restarts
/// us and the windows are still there.
fn fatal_wayland_error(e: wayland_client::DispatchError) -> Error {
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
    } else {
        error!(%e, "fatal wayland error");
    }

    Error::Custom(format!("wayland connection lost: {e}"))
}

// --- wayland dispatch ---

impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for Inner {
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

impl wayland_client::Dispatch<RiverWindowManagerV1, ()> for Inner {
    wayland_client::event_created_child!(Inner, RiverWindowManagerV1, [
        protocol::river_window_management_v1::river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
        protocol::river_window_management_v1::river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
        protocol::river_window_management_v1::river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ()),
    ]);

    fn event(
        inner: &mut Self,
        _: &RiverWindowManagerV1,
        event: protocol::river_window_management_v1::river_window_manager_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_window_manager_v1::Event;

        match event {
            Event::ManageStart => inner.start_sequence(Sequence::Manage),
            Event::RenderStart => inner.start_sequence(Sequence::Render),

            Event::Unavailable => {
                error!(
                    "river is not offering window management: another window manager is running"
                );
                inner.finished = true;
            }

            Event::Finished => {
                info!("river has finished with the window manager");
                inner.finished = true;
            }

            Event::Window { id } => {
                let win_id = WinId(inner.next_win_id);
                inner.next_win_id += 1;

                let node = id.get_node(qh, ());
                debug!(%win_id, "new window");
                inner.by_object.insert(id.id(), win_id);
                inner.windows.insert(
                    win_id,
                    WindowEntry {
                        obj: Some(id),
                        node: Some(node),
                        identifier: None,
                        app_id: None,
                        title: None,
                        parent: None,
                        pid: None,
                        dimensions: None,
                        fullscreen_set: false,
                    },
                );
                inner.new_windows.push(win_id);
            }

            Event::Output { id } => {
                debug!("new output");
                let layer_shell = Some(inner.layer_shell.get_output(&id, qh, ()));
                inner.outputs.push(OutputEntry {
                    obj: id,
                    layer_shell,
                    position: None,
                    dimensions: None,
                    non_exclusive_area: None,
                    removed: false,
                });
                inner.screens_changed = true;
            }

            Event::Seat { id } => {
                debug!("new seat");
                let xkb = Some(inner.xkb.get_seat(&id, qh, ()));
                let layer_shell = Some(inner.layer_shell.get_seat(&id, qh, ()));
                inner.seats.push(SeatEntry {
                    obj: id,
                    xkb,
                    layer_shell,
                    key_bindings: HashMap::new(),
                    mouse_bindings: HashMap::new(),
                    pointer: Point { x: 0, y: 0 },
                    removed: false,
                });
                inner.rebind_seats();
            }

            // Nothing is restricted while the session is locked: the lock screen is a layer
            // surface with exclusive focus, so river is already refusing to give windows focus.
            Event::SessionLocked | Event::SessionUnlocked => (),
        }
    }
}

impl wayland_client::Dispatch<RiverWindowV1, ()> for Inner {
    fn event(
        inner: &mut Self,
        obj: &RiverWindowV1,
        event: protocol::river_window_management_v1::river_window_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_window_v1::Event;

        let Some(id) = inner.win_id(obj) else {
            return; // an event for a window we have already forgotten
        };

        match event {
            Event::Closed => {
                debug!(%id, "window closed");
                if let Some(win) = inner.windows.get_mut(&id) {
                    if let Some(node) = win.node.take() {
                        node.destroy();
                    }
                    if let Some(obj) = win.obj.take() {
                        obj.destroy();
                    }
                }
                inner.queue_event(RiverEvent::WindowClosed(id));
            }

            Event::Dimensions { width, height } => {
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.dimensions = Some((width.max(0) as u32, height.max(0) as u32));
                }
            }

            Event::AppId { app_id } => {
                let announced = !inner.new_windows.contains(&id);
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.app_id = app_id;
                }
                if announced {
                    inner.queue_event(RiverEvent::AppId(id));
                }
            }

            Event::Title { title } => {
                let announced = !inner.new_windows.contains(&id);
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.title = title;
                }
                if announced {
                    inner.queue_event(RiverEvent::Title(id));
                }
            }

            Event::Parent { parent } => {
                let parent = parent.and_then(|p| inner.win_id(&p));
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.parent = parent;
                }
            }

            Event::Identifier { identifier } => {
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.identifier = Some(identifier);
                }
            }

            Event::UnreliablePid { unreliable_pid } => {
                if let Some(win) = inner.windows.get_mut(&id) {
                    win.pid = u32::try_from(unreliable_pid).ok();
                }
            }

            Event::FullscreenRequested { .. } => {
                inner.queue_event(RiverEvent::FullscreenRequested(id, true))
            }
            Event::ExitFullscreenRequested => {
                inner.queue_event(RiverEvent::FullscreenRequested(id, false))
            }

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

impl wayland_client::Dispatch<RiverOutputV1, ()> for Inner {
    fn event(
        inner: &mut Self,
        obj: &RiverOutputV1,
        event: protocol::river_window_management_v1::river_output_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_output_v1::Event;

        let Some(output) = inner.outputs.iter_mut().find(|o| o.obj.id() == obj.id()) else {
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

        inner.screens_changed = true;
    }
}

impl wayland_client::Dispatch<RiverLayerShellOutputV1, ()> for Inner {
    fn event(
        inner: &mut Self,
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
        let found = inner
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
            inner.screens_changed = true;
        }
    }
}

impl wayland_client::Dispatch<RiverSeatV1, ()> for Inner {
    fn event(
        inner: &mut Self,
        obj: &RiverSeatV1,
        event: protocol::river_window_management_v1::river_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_seat_v1::Event;

        match event {
            Event::PointerPosition { x, y } => {
                if let Some(seat) = inner.seats.iter_mut().find(|s| s.obj.id() == obj.id()) {
                    seat.pointer = Point { x, y };
                }
            }

            Event::PointerEnter { window } => {
                if let Some(id) = inner.win_id(&window) {
                    inner.queue_event(RiverEvent::PointerFocus(id));
                }
            }

            Event::WindowInteraction { window } => {
                if let Some(id) = inner.win_id(&window) {
                    inner.queue_event(RiverEvent::Interaction(id));
                }
            }

            Event::Removed => {
                if let Some(seat) = inner.seats.iter_mut().find(|s| s.obj.id() == obj.id()) {
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

impl wayland_client::Dispatch<RiverLayerShellSeatV1, ()> for Inner {
    fn event(
        inner: &mut Self,
        _: &RiverLayerShellSeatV1,
        event: protocol::river_layer_shell::river_layer_shell_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_layer_shell::river_layer_shell_seat_v1::Event;

        // While a layer surface holds exclusive focus -- a lock screen, or a menu -- river
        // ignores everything we say about focus. Knowing that is what lets us avoid fighting it.
        inner.focus_is_exclusive = matches!(event, Event::FocusExclusive);
    }
}

wayland_client::delegate_noop!(Inner: ignore RiverNodeV1);
wayland_client::delegate_noop!(Inner: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(Inner: ignore RiverLayerShellV1);
