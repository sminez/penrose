//! A [Conn] implementation for the [river][1] Wayland compositor.
//!
//! River's window management protocol defers position, size, focus, keybindings and decorations
//! to a separate process, which is what makes an ordinary tiling window manager possible on
//! Wayland at all. See `river-design.md` in the repository root for the design this implements
//! and the reasoning behind it.
//!
//! The window manager draws nothing: bars, menus and prompts are separate clients.
//!
//! # Two threads
//!
//! River holds input processing while a manage sequence is open, so a handler that waits on
//! anything -- a menu, a prompt, a build -- would hold the whole session rather than just the
//! window manager. So the connection lives on a thread of its own which never runs user code and
//! answers sequences from the last plan this side published, and everything here runs on
//! penrose's ordinary run loop exactly as it does under X11. A handler may block; it degrades to
//! the X11 failure mode, where window management stops until it returns and the session carries
//! on.
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
    river_layer_shell::river_layer_shell_v1::RiverLayerShellV1,
    river_window_management_v1::river_window_manager_v1::RiverWindowManagerV1,
    river_xkb_bindings::river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use shared::Shared;
use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Receiver},
    thread,
};
use tracing::{error, info, trace};
use wayland::{FromLoop, Loop};
use wayland_client::{Connection, globals::registry_queue_init};

mod bindings;
mod event;
mod plan;
pub(crate) mod protocol;
mod shared;
mod wayland;

pub use event::RiverEvent;

/// The window manager protocol versions this backend is written against.
///
/// The lower bound of each range is what we actually need: `identifier` for restart and
/// `unreliable_pid` come from window management v4 and v2, and `ensure_next_key_eaten` from xkb
/// bindings v2. Refusing to start is better than discovering the gap at the first key sequence.
const WM_VERSIONS: std::ops::RangeInclusive<u32> = 4..=5;
const XKB_VERSIONS: std::ops::RangeInclusive<u32> = 2..=3;
const LAYER_SHELL_VERSIONS: std::ops::RangeInclusive<u32> = 1..=1;

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
/// Everything that mutates state is written into a plan rather than sent, because river only
/// accepts state changes inside a sequence which it starts: see river-design.md §2.
#[derive(Debug)]
pub struct RiverConn {
    /// Only for `manage_dirty` and `stop`, which are not sequence state and must not wait for the
    /// loop to notice them. Everything else the compositor is told goes through the plan.
    wm: RiverWindowManagerV1,
    conn: Connection,
    shared: Arc<Shared>,
    events: Receiver<FromLoop>,

    manage: ManagePlan,
    render: RenderPlan,
    ops: Vec<Op>,
    /// Whether the plan holds changes river has not been told about yet. River starts a sequence
    /// on its own only when something it knows about changes, so a plan change we made for our
    /// own reasons -- a layout message, a workspace switch -- needs `manage_dirty` to get one.
    plan_dirty: bool,
    /// How many events this side has finished handling, which is what the loop waits for before
    /// answering a sequence.
    handled: u64,
    /// The last event handed to penrose, which has not been handled until the next flush.
    received: u64,

    screen_order: ScreenOrder,
    /// Which tag to put an existing window back on, keyed by river's window identifier.
    restore_tags: HashMap<String, String>,
    finished: bool,
}

/// Why the river connection ended, readable after the window manager has returned.
///
/// See [RiverConn::fatal_watch].
#[derive(Debug, Clone)]
pub struct FatalWatch(Arc<Shared>);

impl FatalWatch {
    /// The reason the connection died, or `None` if it ended in an orderly way -- a hot swap, or
    /// the compositor shutting down.
    pub fn reason(&self) -> Option<String> {
        self.0.fatal()
    }
}

impl RiverConn {
    /// Connect to river, bind the globals a window manager needs, and start the loop thread.
    ///
    /// This blocks until river has sent its initial state -- every existing window, output and
    /// seat -- which it always does before the first manage sequence.
    pub fn new() -> Result<Self> {
        let conn = Connection::connect_to_env().map_err(|e| {
            Error::Custom(format!("unable to connect to a wayland compositor: {e}"))
        })?;

        let (globals, mut queue) = registry_queue_init::<Loop>(&conn)
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

        let shared = Arc::new(Shared::default());
        let (tx, events) = std::sync::mpsc::channel();
        let mut river = Loop::new(wm.clone(), xkb, layer_shell, qh, Arc::clone(&shared), tx);

        // River sends a window event for every existing window, and an output and seat event for
        // each of those, before the first manage sequence. Waiting for that first sequence here,
        // before the loop thread takes over, is what makes existing_clients and screen_details
        // answerable by the time penrose asks.
        while !river.has_pending_sequence() && !river.is_finished() {
            queue
                .blocking_dispatch(&mut river)
                .map_err(|e| Error::Custom(format!("wayland error during startup: {e}")))?;
        }

        if river.is_finished() {
            return Err(Error::Custom(
                "river is not offering window management to us: is another window manager running?"
                    .to_owned(),
            ));
        }

        info!(
            windows = river.window_count(),
            "connected to river, starting the loop thread"
        );

        let loop_conn = conn.clone();
        thread::Builder::new()
            .name("river".to_owned())
            .spawn(move || river.run(loop_conn, queue))
            .map_err(|e| Error::Custom(format!("unable to start the river thread: {e}")))?;

        Ok(Self {
            wm,
            conn,
            shared,
            events,
            manage: ManagePlan::default(),
            render: RenderPlan::default(),
            ops: Vec::new(),
            plan_dirty: false,
            handled: 0,
            received: 0,
            screen_order: ScreenOrder::default(),
            restore_tags: HashMap::new(),
            finished: false,
        })
    }

    /// Index screens from the right rather than from the left.
    pub fn with_screen_order(mut self, order: ScreenOrder) -> Self {
        self.screen_order = order;
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
        self.restore_tags = tags;
        self
    }

    /// Ask river to hand window management to somebody else.
    ///
    /// This is how a restart works: river keeps every client alive across the swap, so the window
    /// manager can exit and be replaced without the session noticing. River answers with
    /// `finished`, which arrives as [RiverEvent::Finished] and stops the run loop; `run` then
    /// returns and the caller can exec its new binary.
    pub fn stop(&mut self) {
        info!("asking river to stop sending us events");
        // Straight down the connection rather than through the plan: this needs no sequence, and
        // waiting for one would mean waiting for something nothing is going to ask for.
        self.wm.stop();
        self.write();
    }

    /// Make a window fullscreen, or take it out of fullscreen.
    ///
    /// River has no `_NET_WM_STATE` for a config to set, so this is the river counterpart of the
    /// `toggle_fullscreen` action: bind it, or call it in response to
    /// [RiverEvent::FullscreenRequested] if you want windows to be able to fullscreen themselves.
    pub fn set_fullscreen(&mut self, id: WinId, fullscreen: bool) {
        if fullscreen {
            self.manage.fullscreen.insert(id);
        } else {
            self.manage.fullscreen.remove(&id);
        }
        self.plan_dirty = true;
    }

    /// Whether river has taken window management away from us.
    ///
    /// This is a hot swap to another window manager, the compositor shutting down, or a protocol
    /// error. [RiverConn::fatal_error] says which.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Why the connection ended, if it ended badly.
    pub fn fatal_error(&self) -> Option<String> {
        self.shared.fatal()
    }

    /// A handle to the same, which outlives the conn.
    ///
    /// A protocol error is the likely outcome of a bug in the plan, and it disconnects the window
    /// manager while leaving the compositor running and looking fine. Exiting non-zero on it is
    /// what lets a supervisor notice and restart; river keeps the clients alive either way, so
    /// the restart costs the session nothing.
    ///
    /// This exists rather than only [RiverConn::fatal_error] because `WindowManager::run` both
    /// consumes the conn and returns `Ok` regardless: its loop hands an error to `handle_error`,
    /// which logs it and carries on, so there is no way for the death of the connection to reach
    /// the caller through the return value. Take one of these before `run`:
    ///
    /// ```no_run
    /// # use penrose::{river::RiverConn, core::{Config, WindowManager}};
    /// # fn main() -> penrose::Result<()> {
    /// let conn = RiverConn::new()?;
    /// let fatal = conn.fatal_watch();
    /// # let wm: WindowManager<RiverConn> = todo!();
    /// wm.run()?;
    ///
    /// if let Some(reason) = fatal.reason() {
    ///     eprintln!("river connection lost: {reason}");
    ///     std::process::exit(1);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn fatal_watch(&self) -> FatalWatch {
        FatalWatch(Arc::clone(&self.shared))
    }

    /// River's identifier for a window, which is stable across a window manager restart.
    pub fn window_identifier(&self, id: WinId) -> Option<String> {
        self.shared.view().windows.get(&id)?.identifier.clone()
    }

    /// Hand the plan to the loop, so that the next sequence transmits it.
    ///
    /// Publishing is what makes a handler's work visible to the compositor, so it happens
    /// wherever penrose would have sent its requests under X11: after each event, and after the
    /// two startup steps that run before any event has arrived.
    fn publish(&mut self) {
        self.shared.publish(
            self.manage.clone(),
            self.render.clone(),
            std::mem::take(&mut self.ops),
            self.handled,
        );

        if self.plan_dirty {
            trace!("asking river for a manage sequence");
            self.wm.manage_dirty();
            self.plan_dirty = false;
        }

        self.write();
    }

    /// Write what we have queued to the socket.
    fn write(&mut self) {
        // Wayland requests have no replies, so the only failure a write can have is a fatal one,
        // which arrives as wl_display.error and makes the loop's next read fail: the event
        // channel is the error channel.
        if let Err(e) = self.conn.flush() {
            error!(%e, "unable to flush the wayland connection");
        }
    }
}

fn bind_error(interface: &str, e: wayland_client::globals::BindError) -> Error {
    Error::Custom(format!(
        "river does not offer a usable {interface}: {e}. \
         penrose needs a river new enough to have the window management protocol."
    ))
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
        match self.events.recv() {
            Ok(FromLoop::Event(seq, e)) => {
                self.received = seq;
                Ok(e)
            }

            Ok(FromLoop::Finished) => {
                self.finished = true;
                Ok(RiverEvent::Finished)
            }

            // The loop has stopped, so there will never be another event. Reporting this as an
            // error would spin the run loop, which logs the error and calls straight back in, so
            // it ends the session instead and the reason is left where a caller can find it after
            // `run` -- which consumes the conn -- has returned. See [RiverConn::fatal_watch].
            Ok(FromLoop::Fatal(reason)) => {
                error!(%reason, "the river connection has ended");
                self.finished = true;
                Ok(RiverEvent::Finished)
            }

            Err(_) => {
                self.finished = true;
                Ok(RiverEvent::Finished)
            }
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

    /// Called by the run loop after every event, which is where the plan reaches the loop thread.
    fn flush(&mut self) {
        self.handled = self.received;
        self.publish();
    }

    fn capture_next_key(&mut self, continuations: &[KeySym]) -> Result<()> {
        self.ops.push(Op::Capture(continuations.to_vec()));
        self.plan_dirty = true;

        Ok(())
    }

    fn cancel_capture_next_key(&mut self) -> Result<()> {
        self.ops.push(Op::CancelCapture);
        self.plan_dirty = true;

        Ok(())
    }

    fn grab(&mut self, keys: &[KeySym], mouse_states: &[MouseState]) -> Result<()> {
        self.ops.push(Op::Grab {
            keys: keys.to_vec(),
            mouse: mouse_states.to_vec(),
        });
        self.plan_dirty = true;
        // Grabbing happens before the run loop starts, so there is no flush coming to carry it.
        self.publish();

        Ok(())
    }

    fn existing_clients(&mut self) -> Result<Vec<WinId>> {
        let mut ids: Vec<WinId> = self.shared.view().windows.keys().copied().collect();
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
                    .window_identifier(id)
                    .and_then(|i| self.restore_tags.get(&i).cloned())
                    .filter(|t| known.contains(t));

                info!(%id, %title, ?tag, "managing existing client");
                manage_without_refresh(id, tag.as_deref(), state, self)?;
            }
        }

        self.refresh(state)?;
        // Also before the run loop starts, so this is the publish that puts the first layout on
        // screen rather than leaving it until whatever event happens to arrive first.
        self.publish();

        Ok(())
    }

    fn screen_details(&mut self) -> Result<Vec<Rect>> {
        let mut rects = self.shared.view().screens.clone();

        if rects.is_empty() {
            return Err(Error::NoScreens);
        }

        if self.screen_order == ScreenOrder::RightToLeft {
            rects.reverse();
        }

        Ok(rects)
    }

    fn cursor_position(&mut self) -> Result<Point> {
        Ok(self.shared.view().pointer)
    }

    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()> {
        // River warps in absolute coordinates where penrose warps within a window, so the
        // planned position of that window is what makes the two the same request.
        let origin = if id == WinId(0) {
            Point { x: 0, y: 0 }
        } else {
            match self.render.positions.get(&id) {
                Some(&p) => p,
                None => return Ok(()), // nowhere to warp to yet
            }
        };

        self.ops.push(Op::WarpPointer(Point {
            x: origin.x + x as i32,
            y: origin.y + y as i32,
        }));
        self.plan_dirty = true;

        Ok(())
    }

    /// A window's size is manage state and its position is render state, so this one call feeds
    /// both halves of the plan and lands on screen over two sequences.
    ///
    /// The client fills its whole allocation: river draws borders over the client's own edges
    /// rather than around them, so there is no room to make. Neighbouring windows then touch and
    /// the line between two of them is one border from each, which is the X11 picture.
    fn position_client(&mut self, id: WinId, r: Rect, border: u32) -> Result<()> {
        let dimensions = (r.w, r.h);
        let position = Point { x: r.x, y: r.y };

        if self.manage.dimensions.insert(id, dimensions) != Some(dimensions)
            || self.render.positions.insert(id, position) != Some(position)
            || self.render.border_widths.insert(id, border) != Some(border)
        {
            self.plan_dirty = true;
        }

        Ok(())
    }

    fn show_client(&mut self, id: WinId, _: &mut State<Self>) -> Result<()> {
        if self.render.visible.insert(id) {
            self.plan_dirty = true;
        }

        Ok(())
    }

    fn hide_client(&mut self, id: WinId, _: &mut State<Self>) -> Result<()> {
        if self.render.visible.remove(&id) {
            self.plan_dirty = true;
        }

        Ok(())
    }

    /// There is no withdrawn state to set: river tells us a window has closed and destroys it.
    fn withdraw_client(&mut self, _: WinId) -> Result<()> {
        Ok(())
    }

    fn kill_client(&mut self, id: WinId) -> Result<()> {
        // A close is not restated: re-sending it would ask a second window to close.
        self.ops.push(Op::Close(id));
        self.plan_dirty = true;

        Ok(())
    }

    fn focus_client(&mut self, id: WinId) -> Result<()> {
        let focus = if id == WinId(0) { None } else { Some(id) };

        if self.manage.focus != focus {
            self.manage.focus = focus;
            self.plan_dirty = true;
        }

        Ok(())
    }

    fn client_geometry(&mut self, id: WinId) -> Result<Rect> {
        let (w, h) = self
            .shared
            .view()
            .windows
            .get(&id)
            .ok_or(Error::UnknownClient(id))?
            .dimensions
            .unwrap_or((0, 0));
        let p = self
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
            .shared
            .view()
            .windows
            .get(&id)
            .and_then(|w| w.title.clone())
            .unwrap_or_default())
    }

    /// River is explicit that this is unreliable, and names the event accordingly.
    fn client_pid(&mut self, id: WinId) -> Option<u32> {
        self.shared.view().windows.get(&id)?.pid
    }

    fn client_should_float(&mut self, id: WinId, floating_classes: &[String]) -> bool {
        match self
            .shared
            .view()
            .windows
            .get(&id)
            .and_then(|w| w.app_id.clone())
        {
            Some(app_id) => floating_classes.iter().any(|c| *c == app_id),
            None => false,
        }
    }

    /// River only tells the window manager about windows it should manage, so the only windows
    /// rejected here are ones it has already closed.
    fn client_should_be_managed(&mut self, id: WinId) -> bool {
        self.shared.view().windows.contains_key(&id)
    }

    fn client_is_fullscreen(&mut self, id: WinId) -> bool {
        self.manage.fullscreen.contains(&id)
    }

    fn client_transient_parent(&mut self, id: WinId) -> Option<WinId> {
        self.shared.view().windows.get(&id)?.parent
    }

    fn set_client_border_color(&mut self, id: WinId, color: impl Into<Color>) -> Result<()> {
        let color = color.into();
        if self.render.borders.insert(id, color) != Some(color) {
            self.plan_dirty = true;
        }

        Ok(())
    }

    fn set_initial_properties(&mut self, id: WinId, _: &Config<Self>) -> Result<()> {
        self.manage.initial_props.insert(id);
        self.plan_dirty = true;

        Ok(())
    }

    fn restack<'a, I>(&mut self, ids: I) -> Result<()>
    where
        WinId: 'a,
        I: Iterator<Item = &'a WinId>,
    {
        let order: Vec<WinId> = ids.copied().collect();
        if order != self.render.order {
            self.render.order = order;
            self.plan_dirty = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn proxies_can_be_used_from_another_thread() {
        // The worker has to be able to send manage_dirty and stop without waiting for the loop to
        // notice, which is only possible if a proxy may be used from another thread. Without this
        // a plan change made for our own reasons would sit unsent until river happened to start a
        // sequence for a reason of its own.
        assert_send_sync::<RiverWindowManagerV1>();
        assert_send_sync::<Connection>();
    }
}
