//! Core data structures and user facing functionality for the window manager
use crate::{
    pure::{geometry::Rect, Diff, ScreenClients, Snapshot, StackSet, Workspace},
    x::{
        manage_without_refresh,
        property::{MapState, WmState},
        Atom, Prop, WindowAttributes, XConn, XConnExt, XEvent,
    },
    Color, Error, Result,
};
use anymap::{any::Any, AnyMap};
use nix::sys::signal::{signal, SigHandler, Signal};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    ops::Deref,
    sync::Arc,
};
use tracing::{debug, error, info, span, trace, warn, Level};

pub mod bindings;
pub(crate) mod handle;
pub mod hooks;
pub mod layout;

use bindings::{KeyBindings, MouseBindings};
use hooks::{EventHook, LayoutHook, ManageHook, StateHook};
use layout::{Layout, LayoutStack};

/// An X11 ID for a given resource
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Xid(pub(crate) u32);

impl std::fmt::Display for Xid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Xid {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u32> for Xid {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<Xid> for u32 {
    fn from(id: Xid) -> Self {
        id.0
    }
}

/// The pure client state information for the window manager
pub type ClientSet = StackSet<Xid>;

/// The pure client state information for a single [Workspace]
pub type ClientSpace = Workspace<Xid>;

/// Mutable internal state for the window manager
#[derive(Debug)]
pub struct State<X>
where
    X: XConn,
{
    /// The user defined configuration options for running the main window manager logic
    pub config: Config<X>,
    /// The pure window manager state
    pub client_set: ClientSet,
    pub(crate) extensions: AnyMap,
    pub(crate) root: Xid,
    pub(crate) mapped: HashSet<Xid>,
    pub(crate) pending_unmap: HashMap<Xid, usize>,
    pub(crate) current_event: Option<XEvent>,
    pub(crate) diff: Diff<Xid>,
    // pub(crate) mouse_focused: bool,
    // pub(crate) mouse_position: Option<(Point, Point)>,
}

impl<X> State<X>
where
    X: XConn,
{
    pub(crate) fn try_new(config: Config<X>, x: &X) -> Result<Self> {
        let mut client_set = StackSet::try_new(
            config.default_layouts.clone(),
            config.tags.iter(),
            x.screen_details()?,
        )?;

        let ss = client_set.snapshot(vec![]);
        let diff = Diff::new(ss.clone(), ss);

        Ok(Self {
            config,
            client_set,
            extensions: AnyMap::new(),
            root: x.root(),
            mapped: HashSet::new(),
            pending_unmap: HashMap::new(),
            current_event: None,
            diff,
        })
    }

    /// The Xid of the root window for the running [WindowManager].
    pub fn root(&self) -> Xid {
        self.root
    }

    /// The set of all client windows currently mapped to a screen.
    pub fn mapped_clients(&self) -> &HashSet<Xid> {
        &self.mapped
    }

    /// The event currently being processed.
    pub fn current_event(&self) -> Option<&XEvent> {
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
            .map(Arc::clone)
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

    pub(crate) fn position_and_snapshot(&mut self, x: &X) -> Snapshot<Xid> {
        let positions = self.visible_client_positions(x);
        self.client_set.snapshot(positions)
    }

    /// Run the per-workspace layouts to get a screen position for each visible client. Floating clients
    /// are placed above stacked clients, clients per workspace are stacked in the order they are returned
    /// from the layout.
    pub(crate) fn visible_client_positions(&mut self, x: &X) -> Vec<(Xid, Rect)> {
        let mut float_positions: Vec<(Xid, Rect)> = Vec::new();
        let mut positions: Vec<(Xid, Rect)> = Vec::new();

        // pop the layout hook off of `state` so that we can pass state into it
        let mut hook = self.config.layout_hook.take();

        let scs: Vec<ScreenClients> = self
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
                    let r_s = h.transform_initial(r_s, self, x);
                    let s = self.client_set.screens.iter_mut().nth(i).unwrap();
                    let (_, initial) = s.workspace.layouts.layout_workspace(&tag, &tiling, r_s);
                    h.transform_positions(r_s, initial, self, x)
                }
                None => {
                    let s = self.client_set.screens.iter_mut().nth(i).unwrap();
                    let (_, positions) = s.workspace.layouts.layout_workspace(&tag, &tiling, r_s);
                    positions
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

/// The user specified config options for how the window manager should run
pub struct Config<X>
where
    X: XConn,
{
    /// The RGBA color to use for normal (unfocused) window borders
    pub normal_border: Color,
    /// The RGBA color to use for the focused window border
    pub focused_border: Color,
    /// The width in pixels to use for drawing window borders
    pub border_width: u32,
    /// Whether or not the mouse entering a new window should set focus
    pub focus_follow_mouse: bool,
    /// The stack of layouts to use for each workspace
    pub default_layouts: LayoutStack,
    /// The ordered set of workspace tags to use on window manager startup
    pub tags: Vec<String>,
    /// Window classes that should always be assigned floating positions rather than tiled
    pub floating_classes: Vec<String>,
    /// A [StateHook] to run before entering the main event loop
    pub startup_hook: Option<Box<dyn StateHook<X>>>,
    /// A [StateHook] to run before processing each [XEvent]
    pub event_hook: Option<Box<dyn EventHook<X>>>,
    /// A [ManageHook] to run after each new window becomes managed by the window manager
    pub manage_hook: Option<Box<dyn ManageHook<X>>>,
    /// A [StateHook] to run every time the on screen X state is refreshed
    pub refresh_hook: Option<Box<dyn StateHook<X>>>,
    /// A [LayoutHook] to run when positioning clients on the screen
    pub layout_hook: Option<Box<dyn LayoutHook<X>>>,
}

impl<X> fmt::Debug for Config<X>
where
    X: XConn,
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

impl<X> Default for Config<X>
where
    X: XConn,
{
    fn default() -> Self {
        let strings = |slice: &[&str]| slice.iter().map(|s| s.to_string()).collect();

        Config {
            normal_border: "#3c3836ff".try_into().expect("valid hex code"),
            focused_border: "#cc241dff".try_into().expect("valid hex code"),
            border_width: 2,
            focus_follow_mouse: true,
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

impl<X> Config<X>
where
    X: XConn,
{
    /// Set the startup_hook or compose it with what is already set.
    ///
    /// The new hook will run before what was there before.
    pub fn compose_or_set_startup_hook<H>(&mut self, hook: H)
    where
        H: StateHook<X> + 'static,
        X: 'static,
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
        H: EventHook<X> + 'static,
        X: 'static,
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
        H: ManageHook<X> + 'static,
        X: 'static,
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
        H: StateHook<X> + 'static,
        X: 'static,
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
        H: LayoutHook<X> + 'static,
        X: 'static,
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
pub struct WindowManager<X>
where
    X: XConn,
{
    x: X,
    /// The mutable [State] of the window manager
    pub state: State<X>,
    key_bindings: KeyBindings<X>,
    mouse_bindings: MouseBindings<X>,
}

impl<X> WindowManager<X>
where
    X: XConn,
{
    /// Construct a new [WindowManager] with the provided config and X connection.
    ///
    /// If you need to set [State] extensions, call [WindowManager::add_extension] after
    /// constructing your initial WindowManager.
    pub fn new(
        config: Config<X>,
        key_bindings: KeyBindings<X>,
        mouse_bindings: MouseBindings<X>,
        x: X,
    ) -> Result<Self> {
        let state = State::try_new(config, &x)?;

        Ok(Self {
            x,
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

        handle::mapping_notify(&self.key_bindings, &self.mouse_bindings, &self.x)?;

        if let Some(mut h) = self.state.config.startup_hook.take() {
            trace!("running user startup hook");
            if let Err(e) = h.call(&mut self.state, &self.x) {
                error!(%e, "error returned from user startup hook");
            }
        }

        manage_existing_clients(&mut self.state, &self.x)?;

        loop {
            match self.x.next_event() {
                Ok(event) => {
                    let span = span!(target: "penrose", Level::INFO, "XEvent", %event);
                    let _enter = span.enter();
                    trace!(details = ?event, "event details");
                    self.state.current_event = Some(event.clone());

                    if let Err(e) = self.handle_xevent(event) {
                        error!(%e, "Error handling XEvent");
                    }
                    self.x.flush();

                    self.state.current_event = None;
                }

                Err(e) => self.handle_error(e),
            }
        }
    }

    fn handle_xevent(&mut self, event: XEvent) -> Result<()> {
        use XEvent::*;

        let WindowManager {
            x,
            state,
            key_bindings,
            mouse_bindings,
        } = self;

        let mut hook = state.config.event_hook.take();
        let should_run = match hook {
            Some(ref mut h) => {
                trace!("running user event hook");
                match h.call(&event, state, x) {
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

        match &event {
            ClientMessage(m) => handle::client_message(m.clone(), state, x)?,
            ConfigureNotify(e) if e.is_root => handle::detect_screens(state, x)?,
            ConfigureNotify(_) => (), // Not currently handled
            ConfigureRequest(e) => handle::configure_request(e, state, x)?,
            Enter(p) => handle::enter(*p, state, x)?,
            Expose(_) => (), // Not currently handled
            FocusIn(id) => handle::focus_in(*id, state, x)?,
            Destroy(xid) => handle::destroy(*xid, state, x)?,
            KeyPress(code) => handle::keypress(*code, key_bindings, state, x)?,
            Leave(p) => handle::leave(*p, state, x)?,
            MappingNotify => handle::mapping_notify(key_bindings, mouse_bindings, x)?,
            MapRequest(xid) => handle::map_request(*xid, state, x)?,
            MouseEvent(e) => handle::mouse_event(e.clone(), mouse_bindings, state, x)?,
            PropertyNotify(_) => (), // Not currently handled
            RandrNotify => handle::detect_screens(state, x)?,
            ScreenChange => handle::screen_change(state, x)?,
            UnmapNotify(xid) => handle::unmap_notify(*xid, state, x)?,

            _ => (), // XEvent is non-exhaustive
        }

        Ok(())
    }

    fn handle_error(&mut self, e: Error) {
        match e {
            // If we get an error from the XConn telling us that a client ID is unknown then
            // we need to make sure that we remove any reference to it from our internal state
            Error::UnknownClient(id) => {
                debug!(%id, "XConn encountered an error due to an unknown client ID: removing client");
                self.state.client_set.remove_client(&id);
            }

            _ => error!(%e, "Unhandled error pulling next x event"),
        }
    }
}

// A "best effort" attempt to manage existing clients on the workspaces they were present
// on previously. This is not guaranteed to preserve the stack order or correctly handle
// any clients that were on invisible workspaces / workspaces that no longer exist.
//
// NOTE: the check for if each client is already in state is in case a startup hook has
//       pre-managed clients for us. In that case we want to avoid stomping on
//       anything that they have set up.
#[tracing::instrument(level = "info", skip(state, x))]
fn manage_existing_clients<X: XConn>(state: &mut State<X>, x: &X) -> Result<()> {
    info!("managing existing clients");

    // We're not guaranteed that workspace indices are _always_ continuous from 0..n
    // so we explicitly map tags to indices instead.
    // We also exclude hidden workspaces as those can contain windows which are
    // externally managed by a user written extension, which can lead to malformed
    // internal state for those extensions when they restart.
    let ws_map: HashMap<usize, String> = state
        .client_set
        .non_hidden_workspaces()
        .map(|w| (w.id, w.tag.clone()))
        .collect();

    let first_tag = state.client_set.ordered_tags()[0].clone();

    for id in x.existing_clients()? {
        if !state.client_set.contains(&id) && client_should_be_manged(id, x) {
            let workspace_id = match x.get_prop(id, Atom::NetWmDesktop.as_ref()) {
                Ok(Some(Prop::Cardinal(ids))) => ids[0] as usize,
                _ => 0, // we know that we always have at least one workspace
            };

            let tag = ws_map.get(&workspace_id).unwrap_or(&first_tag);
            let title = x.window_title(id)?;
            info!(%id, %title, %tag, "attempting to manage existing client");
            manage_without_refresh(id, Some(tag), state, x)?;
        }
    }

    // If EWMH is enabled then we should have this property set to tell us what the previously
    // active client was. If that client is not in the client set or the property is not set we
    // default to forcing focus to the first available tag and whatever active client we have there
    // as that is where we will have placed all existing clients.
    match x.get_prop(state.root, Atom::NetActiveWindow.as_ref()) {
        Ok(Some(Prop::Window(ids))) if state.client_set.contains(&ids[0]) => {
            let id = ids[0];
            info!(%id, "focusing _NET_ACTIVE_WINDOW client");
            state.client_set.focus_client(&id);
        }
        _ => {
            info!(%first_tag, "unable to determine an active window: focusing first tag");
            state.client_set.focus_tag(&first_tag);
        }
    };

    info!("triggering refresh");
    x.refresh(state)
}

/// For a given existing client being processed on startup, determine whether we need
/// to bring it into our internal state and manage it.
fn client_should_be_manged<X: XConn>(id: Xid, x: &X) -> bool {
    let attrs = match x.get_window_attributes(id) {
        Ok(attrs) => attrs,
        _ => {
            warn!(%id, "unable to pull window attributes for client: skipping.");
            return false;
        }
    };

    let wm_state = match x.get_wm_state(id) {
        Ok(state) => state,
        _ => {
            warn!(%id, "unable to pull wm state for client: skipping.");
            return false;
        }
    };

    info!(%id, ?attrs, ?wm_state, "processing client");

    let WindowAttributes {
        override_redirect,
        map_state,
        ..
    } = attrs;

    let viewable = map_state == MapState::Viewable;
    let iconic = wm_state == Some(WmState::Iconic);

    // This condition for determining what windows we should manage is
    // taken from the `scan` function found in both dwm and XMonad.
    !override_redirect && (viewable || iconic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::{test_xid_stack_set, Position};

    fn stack_order(cs: &ClientSet) -> Vec<u32> {
        let positions = cs.visible_client_positions();
        positions.iter().map(|&(id, _)| *id).collect()
    }

    #[test]
    fn floating_client_positions_are_respected() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 0..4 {
            s.insert(Xid(n));
        }

        let r = Rect::new(50, 50, 50, 50);
        s.float_unchecked(Xid(1), r);

        let positions = s.visible_client_positions();

        assert!(positions.contains(&(Xid(1), r)), "{positions:?}")
    }

    #[test]
    fn floating_clients_stay_on_their_assigned_screen() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 0..4 {
            s.insert(Xid(n));
        }

        let r = Rect::new(50, 50, 50, 50);
        s.float_unchecked(Xid(1), r);

        let positions = s.visible_client_positions();

        assert!(positions.contains(&(Xid(1), r)), "{positions:?}");

        // If we move the client to tag 2 on the second screen then it should
        // change position and be relative to that screen instead
        s.move_client_to_tag(&Xid(1), "2");
        let positions = s.visible_client_positions();

        assert!(!positions.contains(&(Xid(1), r)), "{positions:?}");
        assert!(
            positions.contains(&(Xid(1), Rect::new(1050, 2050, 50, 50))),
            "{positions:?}"
        );
    }

    #[test]
    fn floating_windows_are_returned_last() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 1..6 {
            s.insert(Xid(n));
        }

        s.float_unchecked(Xid(2), Rect::new(0, 0, 42, 42));
        s.float_unchecked(Xid(3), Rect::new(0, 0, 69, 69));

        assert_eq!(stack_order(&s), vec![1, 4, 5, 2, 3]);
    }

    #[test]
    fn newly_added_windows_are_below_floating() {
        let mut s = test_xid_stack_set(5, 2);

        for n in 1..6 {
            s.insert(Xid(n));
        }

        s.float_unchecked(Xid(2), Rect::new(0, 0, 42, 42));
        s.float_unchecked(Xid(3), Rect::new(0, 0, 69, 69));

        s.insert(Xid(6));

        assert_eq!(stack_order(&s), vec![1, 4, 5, 6, 2, 3]);
    }

    #[test]
    fn floating_clients_dont_break_insert_focus() {
        let mut s = test_xid_stack_set(1, 1);

        s.insert_at(Position::Focus, Xid(0));
        s.float_unchecked(Xid(0), Rect::new(0, 0, 42, 42));

        assert_eq!(s.current_client(), Some(&Xid(0)));

        // Each time we add a client it should be the focus
        // and the floating window should be stacked above
        // all others.
        let mut expected = vec![0];
        for n in 1..=5 {
            s.insert_at(Position::Focus, Xid(n));
            assert_eq!(s.current_client(), Some(&Xid(n)));

            // Tiled position ordering is reversed in visible_client_positions
            // in order to ensure that when we restack, the order returned
            // is from bottom -> top of the stack to make `restack` simpler to
            // implement.
            expected.insert(expected.len() - 1, n);
            assert_eq!(stack_order(&s), expected, "{:?}", s.current_stack());
        }
    }
}
