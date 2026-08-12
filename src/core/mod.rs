//! Core data structures and user facing functionality for the window manager
use crate::{
    Color, Error, Result,
    core::conn::{Conn, ConnExt},
    pure::{Diff, ScreenClients, Snapshot, StackSet, Workspace, geometry::Rect},
};
use anymap::{AnyMap, any::Any};
use nix::sys::signal::{SigHandler, Signal, signal};
use std::{any::TypeId, cell::RefCell, cmp::Ordering, collections::HashSet, fmt, sync::Arc};
use tracing::{Level, debug, error, info, span, trace};

pub mod bindings;
pub mod conn;
pub mod hooks;
pub mod layout;

use bindings::{KeyBindings, MouseBindings, MouseState};
use hooks::{EventHook, LayoutHook, ManageHook, StateHook};
use layout::LayoutStack;

pub use conn::WinId;

/// The pure client state information for the window manager
pub type ClientSet = StackSet<WinId>;

/// The pure client state information for a single [Workspace]
pub type ClientSpace = Workspace<WinId>;

/// Mutable internal state for the window manager
#[derive(Debug)]
pub struct State<C>
where
    C: Conn,
{
    /// The user defined configuration options for running the main window manager logic
    pub config: Config<C>,
    /// The pure window manager state
    pub client_set: StackSet<WinId>,
    /// Additional state for the [Conn]
    pub conn_state: C::State,
    pub(crate) extensions: AnyMap,
    pub(crate) root: WinId,
    pub(crate) current_event: Option<C::Event>,
    pub(crate) diff: Diff<WinId>,
    pub(crate) running: bool,
    pub(crate) held_mouse_state: Option<MouseState>,
    pub(crate) pending_keys: Vec<<C as Conn>::KeyBindingKey>,
}

impl<C> State<C>
where
    C: Conn,
{
    pub(crate) fn try_new(config: Config<C>, conn: &mut C) -> Result<Self> {
        let mut client_set = StackSet::try_new(
            config.default_layouts.clone(),
            config.tags.iter(),
            conn.screens(config.screen_order)?,
        )?;

        let ss = client_set.snapshot(vec![]);
        let diff = Diff::new(ss.clone(), ss);
        let conn_state = conn.initial_state();

        Ok(Self {
            config,
            client_set,
            conn_state,
            extensions: AnyMap::new(),
            root: conn.root(),
            current_event: None,
            diff,
            running: false,
            held_mouse_state: None,
            pending_keys: Vec::new(),
        })
    }

    /// The WinId of the root window for the running [WindowManager].
    pub fn root(&self) -> WinId {
        self.root
    }

    /// The set of all client windows currently mapped to a screen.
    pub fn mapped_clients(&self) -> HashSet<WinId> {
        self.client_set.clients().cloned().collect()
    }

    /// The event currently being processed.
    pub fn current_event(&self) -> Option<&C::Event> {
        self.current_event.as_ref()
    }

    /// Get access to a shared state extension.
    ///
    /// To add an extension to [State] before starting the Window Manager, see the
    /// [WindowManager::add_extension] method. To add an extension dynamically
    /// when you have access to [State], see [State::add_extension].
    ///
    /// # Errors
    /// Returns `Error::UnknownStateExtension` if there is no extension of type `E`.
    pub fn extension<E: Any>(&self) -> Result<Arc<RefCell<E>>> {
        self.extensions
            .get()
            .cloned()
            .ok_or(Error::UnknownStateExtension {
                type_id: TypeId::of::<E>(),
            })
    }

    /// Get access to a shared state extension or set it using Default.
    pub fn extension_or_default<E: Default + Any>(&mut self) -> Arc<RefCell<E>> {
        if !self.extensions.contains::<Arc<RefCell<E>>>() {
            self.add_extension(E::default());
        }

        self.extension().expect("to have defaulted if missing")
    }

    /// Remove a shared state extension entirely.
    ///
    /// Returns `None` if there is no extension of type `E` or if that extension
    /// is currently being held by another thread.
    pub fn remove_extension<E: Any>(&mut self) -> Option<E> {
        let arc: Arc<RefCell<E>> = self.extensions.remove()?;

        // If there is only one strong reference to this state then we'll be able to
        // try_unwrap it and return the underlying `E`. If not the this fails so we
        // need to store it back in the extensions anymap.
        match Arc::try_unwrap(arc) {
            Ok(rc) => Some(rc.into_inner()),
            Err(arc) => {
                self.extensions.insert(arc);
                None
            }
        }
    }

    /// Add a typed [State] extension to this State.
    pub fn add_extension<E: Any>(&mut self, extension: E) {
        self.extensions.insert(Arc::new(RefCell::new(extension)));
    }

    pub(crate) fn position_and_snapshot(&mut self, conn: &mut C) -> Snapshot<WinId> {
        let positions = self.visible_client_positions(conn);
        self.client_set.snapshot(positions)
    }

    /// Run the per-workspace layouts to get a screen position for each visible client. Floating clients
    /// are placed above stacked clients, clients per workspace are stacked in the order they are returned
    /// from the layout.
    pub(crate) fn visible_client_positions(&mut self, conn: &mut C) -> Vec<(WinId, Rect)> {
        let mut float_positions: Vec<(WinId, Rect)> = Vec::new();
        let mut positions: Vec<(WinId, Rect)> = Vec::new();

        // pop the layout hook off of `state` so that we can pass state into it
        let mut hook = self.config.layout_hook.take();

        let scs: Vec<ScreenClients<WinId>> = self
            .client_set
            .screens
            .iter()
            .map(|s| s.screen_clients(&self.client_set.floating))
            .collect();

        for (i, sc) in scs.into_iter().enumerate() {
            let ScreenClients {
                floating,
                tiling,
                tag,
                r_s,
            } = sc;

            // Sort out the floating client positions first
            for (c, r_c) in floating.iter() {
                float_positions.push((*c, r_c.applied_to(&r_s)));
            }

            // Next run layout functions for each workspace on a visible screen
            let stack_positions = match hook {
                Some(ref mut h) => {
                    let r_s = h.transform_initial_for_screen(i, r_s, self, conn);
                    let s = self.client_set.screens.iter_mut().nth(i).unwrap();
                    let initial = s.workspace.apply_layout(&tag, &tiling, r_s);
                    h.transform_positions_for_screen(i, r_s, initial, self, conn)
                }
                None => {
                    let s = self.client_set.screens.iter_mut().nth(i).unwrap();
                    s.workspace.apply_layout(&tag, &tiling, r_s)
                }
            };

            positions.extend(stack_positions.into_iter().rev());
        }

        float_positions.reverse();
        positions.extend(float_positions);

        // Restore the layout hook
        self.config.layout_hook = hook;

        positions
    }
}

/// How screen indices are assigned: an ordering over the rects a backend reports.
///
/// Backends report screens in whatever order they please -- the X server has its own, and river
/// makes no promise at all -- so which monitor is screen 0 is a preference rather than a fact.
/// This is `xmonad-contrib`'s `ScreenComparator`, and it is why no backend needs a wrapper to
/// reorder them.
///
/// [left_to_right] and [top_to_bottom] cover the usual cases; anything else is a function of your
/// own. Counting from the other end is `.reverse()` on one of these:
///
/// ```
/// # use penrose::{core::left_to_right, pure::geometry::Rect};
/// fn right_to_left(a: &Rect, b: &Rect) -> std::cmp::Ordering {
///     left_to_right(a, b).reverse()
/// }
/// ```
pub type ScreenComparator = fn(&Rect, &Rect) -> Ordering;

/// Screen 0 is the leftmost, ties broken top to bottom.
pub fn left_to_right(a: &Rect, b: &Rect) -> Ordering {
    (a.x, a.y).cmp(&(b.x, b.y))
}

/// Screen 0 is the topmost, ties broken left to right.
pub fn top_to_bottom(a: &Rect, b: &Rect) -> Ordering {
    (a.y, a.x).cmp(&(b.y, b.x))
}

/// The user specified config options for how the window manager should run
pub struct Config<C>
where
    C: Conn,
{
    /// The RGBA color to use for normal (unfocused) window borders
    pub normal_border: Color,
    /// The RGBA color to use for the focused window border
    pub focused_border: Color,
    /// The width in pixels to use for drawing window borders
    pub border_width: u32,
    /// Whether or not the mouse entering a new window should set focus
    pub focus_follow_mouse: bool,
    /// Which monitor is screen 0, and so which `focus_screen(n)` means what
    pub screen_order: ScreenComparator,
    /// The stack of layouts to use for each workspace
    pub default_layouts: LayoutStack,
    /// The ordered set of workspace tags to use on window manager startup
    pub tags: Vec<String>,
    /// Window classes that should always be assigned floating positions rather than tiled
    pub floating_classes: Vec<String>,
    /// A [StateHook] to run before entering the main event loop
    pub startup_hook: Option<Box<dyn StateHook<C>>>,
    /// A [StateHook] to run before processing each [XEvent]
    pub event_hook: Option<Box<dyn EventHook<C>>>,
    /// A [ManageHook] to run after each new window becomes managed by the window manager
    pub manage_hook: Option<Box<dyn ManageHook<C>>>,
    /// A [StateHook] to run every time the on screen X state is refreshed
    pub refresh_hook: Option<Box<dyn StateHook<C>>>,
    /// A [LayoutHook] to run when positioning clients on the screen
    pub layout_hook: Option<Box<dyn LayoutHook<C>>>,
}

impl<C> fmt::Debug for Config<C>
where
    C: Conn,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("normal_border", &self.normal_border)
            .field("focused_border", &self.focused_border)
            .field("border_width", &self.border_width)
            .field("focus_follow_mouse", &self.focus_follow_mouse)
            .field("default_layouts", &self.default_layouts)
            .field("tags", &self.tags)
            .field("floating_classes", &self.floating_classes)
            .finish()
    }
}

impl<C> Default for Config<C>
where
    C: Conn,
{
    fn default() -> Self {
        let strings = |slice: &[&str]| slice.iter().map(|s| s.to_string()).collect();

        Config {
            normal_border: "#3c3836ff".try_into().expect("valid hex code"),
            focused_border: "#cc241dff".try_into().expect("valid hex code"),
            border_width: 2,
            focus_follow_mouse: true,
            screen_order: left_to_right,
            default_layouts: LayoutStack::default(),
            tags: strings(&["1", "2", "3", "4", "5", "6", "7", "8", "9"]),
            floating_classes: strings(&["dmenu", "dunst"]),
            startup_hook: None,
            event_hook: None,
            manage_hook: None,
            refresh_hook: None,
            layout_hook: None,
        }
    }
}

impl<C> Config<C>
where
    C: Conn,
{
    /// Set the startup_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_startup_hook<H>(&mut self, hook: H)
    where
        H: StateHook<C> + 'static,
        C: 'static,
    {
        self.startup_hook = match self.startup_hook.take() {
            Some(h) => Some(hook.then_boxed(h)),
            None => Some(hook.boxed()),
        };
    }

    /// Set the event_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_event_hook<H>(&mut self, hook: H)
    where
        H: EventHook<C> + 'static,
        C: 'static,
    {
        self.event_hook = match self.event_hook.take() {
            Some(h) => Some(hook.then_boxed(h)),
            None => Some(hook.boxed()),
        };
    }

    /// Set the manage_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_manage_hook<H>(&mut self, hook: H)
    where
        H: ManageHook<C> + 'static,
        C: 'static,
    {
        self.manage_hook = match self.manage_hook.take() {
            Some(h) => Some(hook.then_boxed(h)),
            None => Some(hook.boxed()),
        };
    }

    /// Set the refresh_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_refresh_hook<H>(&mut self, hook: H)
    where
        H: StateHook<C> + 'static,
        C: 'static,
    {
        self.refresh_hook = match self.refresh_hook.take() {
            Some(h) => Some(hook.then_boxed(h)),
            None => Some(hook.boxed()),
        };
    }

    /// Set the layout_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_layout_hook<H>(&mut self, hook: H)
    where
        H: LayoutHook<C> + 'static,
        C: 'static,
    {
        self.layout_hook = match self.layout_hook.take() {
            Some(h) => Some(hook.then_boxed(h)),
            None => Some(hook.boxed()),
        };
    }
}

/// A top level struct holding all of the state required to run as an X11 window manager.
///
/// This allows for final configuration to be carried out before entering the main event
/// loop.
#[derive(Debug)]
pub struct WindowManager<C>
where
    C: Conn,
{
    conn: C,
    /// The mutable [State] of the window manager
    pub state: State<C>,
    key_bindings: KeyBindings<C>,
    mouse_bindings: MouseBindings<C>,
}

impl<C> WindowManager<C>
where
    C: Conn,
{
    /// Construct a new [WindowManager] with the provided config and X connection.
    ///
    /// If you need to set [State] extensions, call [WindowManager::add_extension] after
    /// constructing your initial WindowManager.
    pub fn new(
        config: Config<C>,
        key_bindings: KeyBindings<C>,
        mouse_bindings: MouseBindings<C>,
        mut conn: C,
    ) -> Result<Self> {
        let state = State::try_new(config, &mut conn)?;

        Ok(Self {
            conn,
            state,
            key_bindings,
            mouse_bindings,
        })
    }

    /// Add a typed [State] extension to this WindowManager.
    pub fn add_extension<E: Any>(&mut self, extension: E) {
        self.state.add_extension(extension);
    }

    /// Start the WindowManager and run it until told to exit.
    ///
    /// Any provided startup hooks will be run after setting signal handlers and grabbing
    /// key / mouse bindings from the X server. Any set up you need to do should be run
    /// explicitly before calling this method or as part of a startup hook.
    ///
    /// ## Existing clients
    /// An attempt will be made to pull any existing clients already present into the current
    /// WindowManager state. This is done on a "best effort" basis to manage existing clients on
    /// the workspaces they were present on previously. If you are planning on making use of this
    /// functionality for more than recovering from a crash it is advised that you add EWMH hooks
    /// to your Config so that there is more information available to correctly position your
    /// existing clients.
    /// Startup hooks are run before this takes place so that there is an opportunity to handle
    /// restoring any state being held outside of the main WindowManager data structures.
    ///
    /// > **NOTE**: This is not guaranteed to preserve the stacking order or correctly handle any
    /// > clients that were on invisible workspaces / workspaces that no longer exist and that the
    /// > workspace containing the previously active client will be placed on the first available
    /// > screen.
    pub fn run(mut self) -> Result<()> {
        info!("registering SIGCHILD signal handler");
        // SAFETY: there is no previous signal handler so we are safe to set our own without needing
        //         to worry about UB from the previous handler being invalid.
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) } {
            panic!("unable to set signal handler: {}", e);
        }

        let key_codes = self.key_bindings.leading_keys();
        let mouse_states: Vec<_> = self.mouse_bindings.keys().cloned().collect();
        self.conn.grab(&key_codes, &mouse_states)?;

        if let Some(mut h) = self.state.config.startup_hook.take() {
            trace!("running user startup hook");
            if let Err(e) = h.call(&mut self.state, &mut self.conn) {
                error!(%e, "error returned from user startup hook");
            }
        }

        info!("managing existing clients");
        self.conn.manage_existing_clients(&mut self.state)?;
        self.state.running = true;

        debug!("entering main run loop");
        while self.state.running {
            match self.conn.next_event() {
                Ok(event) => {
                    let span = span!(target: "penrose", Level::INFO, "Event", %event);
                    let _enter = span.enter();
                    trace!(details = ?event, "event details");
                    self.state.current_event = Some(event.clone());

                    if let Err(e) = self.handle_event(event) {
                        error!(%e, "Error handling Event");
                    }
                    self.conn.flush();
                    self.state.current_event = None;
                }

                Err(e) => self.handle_error(e),
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: C::Event) -> Result<()> {
        let WindowManager {
            conn,
            state,
            key_bindings,
            mouse_bindings,
        } = self;

        let mut hook = state.config.event_hook.take();
        let should_run = match hook {
            Some(ref mut h) => {
                trace!("running user event hook");
                match h.call(&event, state, conn) {
                    Ok(should_run) => should_run,
                    Err(e) => {
                        error!(%e, "error returned from user event hook");
                        true
                    }
                }
            }

            None => true,
        };
        state.config.event_hook = hook;

        if !should_run {
            trace!("User event hook returned false: skipping default handling");
            return Ok(());
        }

        conn.handle_event(event, key_bindings, mouse_bindings, state)
    }

    fn handle_error(&mut self, e: Error) {
        match e {
            // If we get an error from the XConn telling us that a client ID is unknown then
            // we need to make sure that we remove any reference to it from our internal state
            Error::UnknownClient(id) => {
                debug!(%id, "Conn encountered an error due to an unknown client ID: removing client");
                self.state.client_set.remove_client(&id);
            }

            _ => error!(%e, "Unhandled error pulling next event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::{Position, test_xid_stack_set};

    fn stack_order(cs: &ClientSet) -> Vec<u32> {
        let positions = cs.visible_client_positions();
        positions.iter().map(|&(id, _)| *id).collect()
    }

    #[test]
    fn floating_client_positions_are_respected() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 0..4 {
            s.insert(WinId(n));
        }

        let r = Rect::new(50, 50, 50, 50);
        s.float_unchecked(WinId(1), r);

        let positions = s.visible_client_positions();

        assert!(positions.contains(&(WinId(1), r)), "{positions:?}")
    }

    #[test]
    fn floating_clients_stay_on_their_assigned_screen() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 0..4 {
            s.insert(WinId(n));
        }

        let r = Rect::new(50, 50, 50, 50);
        s.float_unchecked(WinId(1), r);

        let positions = s.visible_client_positions();

        assert!(positions.contains(&(WinId(1), r)), "{positions:?}");

        // If we move the client to tag 2 on the second screen then it should
        // change position and be relative to that screen instead
        s.move_client_to_tag(&WinId(1), "2");
        let positions = s.visible_client_positions();

        assert!(!positions.contains(&(WinId(1), r)), "{positions:?}");
        assert!(
            positions.contains(&(WinId(1), Rect::new(1050, 2050, 50, 50))),
            "{positions:?}"
        );
    }

    #[test]
    fn floating_windows_are_returned_last() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 1..6 {
            s.insert(WinId(n));
        }

        s.float_unchecked(WinId(2), Rect::new(0, 0, 42, 42));
        s.float_unchecked(WinId(3), Rect::new(0, 0, 69, 69));

        assert_eq!(stack_order(&s), vec![1, 4, 5, 2, 3]);
    }

    #[test]
    fn newly_added_windows_are_below_floating() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 1..6 {
            s.insert(WinId(n));
        }

        s.float_unchecked(WinId(2), Rect::new(0, 0, 42, 42));
        s.float_unchecked(WinId(3), Rect::new(0, 0, 69, 69));

        s.insert(WinId(6));

        assert_eq!(stack_order(&s), vec![1, 4, 5, 6, 2, 3]);
    }

    #[test]
    fn floating_clients_dont_break_insert_focus() {
        let mut s = test_xid_stack_set(1, 1);

        s.insert_at(Position::Focus, WinId(0));
        s.float_unchecked(WinId(0), Rect::new(0, 0, 42, 42));

        assert_eq!(s.current_client(), Some(&WinId(0)));

        // Each time we add a client it should be the focus
        // and the floating window should be stacked above
        // all others.
        let mut expected = vec![0];
        for n in 1..=5 {
            s.insert_at(Position::Focus, WinId(n));
            assert_eq!(s.current_client(), Some(&WinId(n)));

            // Tiled position ordering is reversed in visible_client_positions
            // in order to ensure that when we restack, the order returned
            // is from bottom -> top of the stack to make `restack` simpler to
            // implement.
            expected.insert(expected.len() - 1, n);
            assert_eq!(stack_order(&s), expected, "{:?}", s.current_stack());
        }
    }
}
