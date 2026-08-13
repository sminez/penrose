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
    collections::{HashMap, HashSet},
    sync::{Arc, mpsc::Receiver},
    thread,
};
use tracing::{debug, error, info, trace};
use wayland::{FromLoop, Loop};
use wayland_client::{Connection, globals::registry_queue_init};

mod bindings;
mod event;
mod plan;
pub(crate) mod protocol;
pub mod query;
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

    /// Which tag to put an existing window back on, keyed by river's window identifier.
    restore_tags: HashMap<String, String>,
    /// The identifier of the window that had focus before a restart.
    restore_focus: Option<String>,
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
        // before the loop thread takes over, is what makes existing_clients and the screens
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
            restore_tags: HashMap::new(),
            restore_focus: None,
            finished: false,
        })
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

    /// Name the bindings that go on working while the session is locked.
    ///
    /// Nothing does, by default. River matches a key against the window manager's bindings before
    /// it reaches the surface with keyboard focus, and carries on doing so with a lock screen up,
    /// so a config's whole keymap would otherwise be available to whoever sits down in front of a
    /// locked machine -- and one of those keys is a terminal.
    ///
    /// This is the exception list, in the same patterns the bindings themselves use. It is the
    /// answer to "what can somebody at my locked laptop do", so it is worth keeping short and
    /// worth reading as a whole: volume and brightness say nothing about the session and change
    /// nothing in it, where anything that spawns, switches workspace or reveals a window does not
    /// belong here at any price.
    ///
    /// A key sequence leader is a poor choice: the keys that would continue it stay disabled, so
    /// pressing it eats the key and abandons the sequence.
    ///
    /// The X11 backend has no counterpart because it needs none. An X11 locker holds an active
    /// keyboard grab, which overrides the passive grabs bindings are made of, so none of them fire
    /// while it is up whatever anyone asks for.
    pub fn allow_while_locked(self, patterns: &[&str]) -> Result<Self> {
        let keys = patterns
            .iter()
            .map(|p| KeySym::parse(p))
            .collect::<Result<HashSet<_>>>()?;

        info!(count = keys.len(), "bindings allowed while locked");
        self.shared.set_allow_while_locked(keys);

        Ok(self)
    }

    /// Give focus back to the window that had it before a restart.
    ///
    /// The argument is river's window identifier, as [RiverConn::restore_tags] is keyed on, and is
    /// applied by `manage_existing_clients` once every window is back on its tag -- so it decides
    /// which workspace the session comes back up on. A window that no longer exists, or no
    /// identifier at all, falls back to the first tag.
    ///
    /// The X11 backend does this from `_NET_ACTIVE_WINDOW`, which the X server keeps for it. River
    /// has nowhere to keep anything, so this comes from wherever the caller wrote it.
    pub fn restore_focus(mut self, identifier: Option<String>) -> Self {
        self.restore_focus = identifier;
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

    /// End the Wayland session: log out.
    ///
    /// Everything in the session is disconnected, this window manager included, so this is the
    /// last thing a config does. River is explicit that it is for a user asking to log out and
    /// not for ordinary window manager termination -- use [RiverConn::stop] to hand over to a
    /// replacement, which is what a restart wants.
    pub fn exit_session(&mut self) {
        info!("ending the wayland session");
        self.wm.exit_session();
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

    /// The window's `app_id`, which is Wayland's answer to `WM_CLASS`.
    ///
    /// See [query::AppId], which is how a manage hook asks.
    pub fn window_app_id(&self, id: WinId) -> Option<String> {
        self.shared.view().windows.get(&id)?.app_id.clone()
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

/// Is this client on a workspace that is actually on a screen?
///
/// The pointer can only be over a window river is showing, so an enter naming one penrose has
/// hidden means the two disagree about what is on screen -- and following it would switch
/// workspace to whatever is under a cursor that has not moved.
///
/// They disagree at every restart. River un-hides every window when the window manager it was
/// talking to disconnects (`hidden = false` in `Window.zig`'s `handleDestroy`), so for as long as
/// it takes the replacement to publish its first render plan, the whole session is stacked on
/// screen and the pointer is over whichever of them happens to be on top. River then reports that
/// window as hovered in its initial state, which is how a restart used to end up on the workspace
/// of a window the user had not looked at in hours.
fn is_on_a_visible_tag(cs: &crate::pure::StackSet<WinId>, id: WinId) -> bool {
    match cs.tag_for_client(&id) {
        Some(tag) => cs.screens().any(|s| s.workspace.tag == tag),
        None => false,
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
                let rects = self.screens(state.config.screen_order)?;
                info!(?rects, "screens changed");
                state.client_set.update_screens(rects)?;
                self.refresh(state)?;
            }

            PointerFocus(id) | Interaction(id) => {
                if is_on_a_visible_tag(&state.client_set, id) && state.config.focus_follow_mouse {
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
        let mut had_focus = None;

        for id in self.existing_clients()? {
            if !state.client_set.contains(&id) && self.client_should_be_managed(id) {
                let title = self.client_title(id)?;
                let identifier = self.window_identifier(id);
                // A tag that no longer exists in the config would be a workspace nothing can
                // reach, so an unknown one falls back to the current workspace.
                let tag = identifier
                    .as_ref()
                    .and_then(|i| self.restore_tags.get(i).cloned())
                    .filter(|t| known.contains(t));

                if identifier.is_some() && identifier == self.restore_focus {
                    had_focus = Some(id);
                }

                info!(%id, %title, ?tag, "managing existing client");
                manage_without_refresh(id, tag.as_deref(), state, self)?;
            }
        }

        // Which workspace the session comes back up on. Without this it would be whichever one the
        // client set starts on, so a restart would move the user off the workspace they were
        // working on -- the same reason the X11 backend restores _NET_ACTIVE_WINDOW here.
        match had_focus {
            Some(id) => {
                info!(%id, "focusing the client that had focus before the restart");
                state.client_set.focus_client(&id);
            }
            None => {
                if let Some(tag) = known.first() {
                    info!(%tag, "no focused client to restore: focusing the first tag");
                    state.client_set.focus_tag(tag);
                }
            }
        }

        self.refresh(state)?;
        // Also before the run loop starts, so this is the publish that puts the first layout on
        // screen rather than leaving it until whatever event happens to arrive first.
        self.publish();

        Ok(())
    }

    /// Reported left to right, which `Config::screen_order` then has the last word on.
    fn unordered_screens(&mut self) -> Result<Vec<Rect>> {
        let rects = self.shared.view().screens.clone();

        if rects.is_empty() {
            return Err(Error::NoScreens);
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

        let to = Point {
            x: origin.x + x as i32,
            y: origin.y + y as i32,
        };

        debug!(%id, x = to.x, y = to.y, "warping the pointer");
        self.ops.push(Op::WarpPointer(to));
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

        // Each insert on its own line because each one is a side effect: `||`
        // does not evaluate its right operand when the left is true, so writing
        // this as one condition stored the size and silently dropped the
        // position for every window whose size had changed -- which is every
        // window in a layout that just changed.
        let resized = self.manage.dimensions.insert(id, dimensions) != Some(dimensions);
        let moved = self.render.positions.insert(id, position) != Some(position);
        let reframed = self.render.border_widths.insert(id, border) != Some(border);

        if resized || moved || reframed {
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

    /// Where the window is going to be, which within a refresh is not where river says it is.
    ///
    /// Both halves come from the plan. River's own account of a window is a sequence behind ours
    /// -- it is what it was told last time -- so a caller that asked during a refresh, which is
    /// every caller, would be told the size the window is on its way out of. Mixing the two is
    /// worse than either: the pointer warp asks for a window's centre, and a new position offset
    /// by half of an old size lands somewhere that was never anything.
    ///
    /// A window penrose has not placed yet has no plan to read, and river's report is then the
    /// only thing there is -- the size the client asked for, which is what a manage hook wants
    /// when it centres a dialog.
    fn client_geometry(&mut self, id: WinId) -> Result<Rect> {
        let requested = {
            let view = self.shared.view();
            let window = view.windows.get(&id).ok_or(Error::UnknownClient(id))?;

            window.dimensions
        };

        let (w, h) = self
            .manage
            .dimensions
            .get(&id)
            .copied()
            .or(requested)
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
