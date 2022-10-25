//! Core data structures and user facing functionality for the window manager
use crate::{
    pure::{Diff, StackSet, Workspace},
    x::{XConn, XConnExt, XEvent},
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
use tracing::{error, span, trace, Level};

pub mod actions;
pub mod bindings;
pub mod handle;
pub mod hooks;
pub mod layout;

use bindings::{KeyBindings, MouseBindings};
use hooks::{EventHook, ManageHook, StateHook};
use layout::LayoutStack;

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
    pub config: Config<X>,
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
}

pub struct Config<X>
where
    X: XConn,
{
    pub normal_border: Color,
    pub focused_border: Color,
    pub border_width: u32,
    pub focus_follow_mouse: bool,
    pub default_layouts: LayoutStack,
    pub workspace_names: Vec<String>,
    pub floating_classes: Vec<String>,
    pub startup_hook: Option<Box<dyn StateHook<X>>>,
    pub event_hook: Option<Box<dyn EventHook<X>>>,
    pub manage_hook: Option<Box<dyn ManageHook<X>>>,
    pub refresh_hook: Option<Box<dyn StateHook<X>>>,
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
            .field("workspace_names", &self.workspace_names)
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
            normal_border: "#3c3836".try_into().expect("valid hex code"),
            focused_border: "#cc241d".try_into().expect("valid hex code"),
            border_width: 2,
            focus_follow_mouse: true,
            default_layouts: LayoutStack::default(),
            workspace_names: strings(&["1", "2", "3", "4", "5", "6", "7", "8", "9"]),
            floating_classes: strings(&["dmenu", "dunst"]),
            startup_hook: None,
            event_hook: None,
            manage_hook: None,
            refresh_hook: None,
        }
    }
}

impl<X> Config<X>
where
    X: XConn,
{
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
}

/// A top level struct holding all of the state required to run as an X11 window manager.
///
/// This allows for final configuration to be carried out before entering the main event
/// loop.
pub struct WindowManager<X>
where
    X: XConn,
{
    x: X,
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
        let client_set = StackSet::try_new(
            config.default_layouts.clone(),
            config.workspace_names.iter(),
            x.screen_details()?,
        )?;

        let ss = client_set.snapshot(vec![]);
        let diff = Diff::new(ss.clone(), ss);

        let state = State {
            config,
            client_set,
            extensions: AnyMap::new(),
            root: x.root(),
            mapped: HashSet::new(),
            pending_unmap: HashMap::new(),
            current_event: None,
            diff,
        };

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
    pub fn run(mut self) -> Result<()> {
        trace!("registering SIGCHILD signal handler");
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) } {
            panic!("unable to set signal handler: {}", e);
        }

        self.grab()?;

        if let Some(mut h) = self.state.config.startup_hook.take() {
            trace!("running user startup hook");
            if let Err(e) = h.call(&mut self.state, &self.x) {
                error!(%e, "error returned from user startup hook");
            }
        }

        self.x.modify_and_refresh(&mut self.state, |_| ())?;

        loop {
            match self.x.next_event() {
                Ok(event) => {
                    let span = span!(target: "penrose", Level::DEBUG, "XEvent", %event);
                    let _enter = span.enter();
                    trace!(details = ?event, "event details");
                    self.state.current_event = Some(event.clone());

                    self.handle_xevent(event)?;
                    self.x.flush();

                    self.state.current_event = None;
                }

                Err(e) => error!(%e, "Error pulling next x event"),
            }
        }
    }

    fn grab(&self) -> Result<()> {
        trace!("grabbing key and mouse bindings");
        let key_codes: Vec<_> = self.key_bindings.keys().copied().collect();
        let mouse_states: Vec<_> = self
            .mouse_bindings
            .keys()
            .map(|(_, state)| state.clone())
            .collect();

        self.x.grab(&key_codes, &mouse_states)
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
        if let Some(ref mut h) = hook {
            trace!("running user event hook");
            let should_run = match h.call(&event, state, x) {
                Ok(should_run) => should_run,
                Err(e) => {
                    error!(%e, "error returned from user event hook");
                    true
                }
            };

            if !should_run {
                trace!("User event hook returned false: skipping default handling");
                return Ok(());
            }
        }
        state.config.event_hook = hook;

        match &event {
            ClientMessage(m) => handle::client_message(m.clone(), state, x)?,
            ConfigureNotify(e) if e.is_root => handle::detect_screens(state, x)?,
            ConfigureNotify(_) => (),  // Not currently handled
            ConfigureRequest(_) => (), // Not currently handled
            Enter(p) => handle::enter(p.id, state, x)?,
            Expose(_) => (), // Not currently handled
            FocusIn(id) => handle::focus_in(*id, state, x)?,
            Destroy(xid) => handle::destroy(*xid, state, x)?,
            KeyPress(code) => handle::keypress(*code, key_bindings, state, x)?,
            Leave(p) => handle::leave(p.id, p.abs, state, x)?,
            MappingNotify => (), // Not currently handled
            MapRequest(xid) => handle::map_request(*xid, state, x)?,
            MouseEvent(e) => handle::mouse_event(e.clone(), mouse_bindings, state, x)?,
            PropertyNotify(_) => (), // Not currently handled
            RandrNotify => handle::detect_screens(state, x)?,
            ScreenChange => handle::screen_change(state, x)?,
            UnmapNotify(xid) => handle::unmap_notify(*xid, state, x)?,
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::geometry::Rect;

    #[test]
    fn visible_client_positions_respects_floating_clients() {
        let tags = (1..10).map(|n| n.to_string());
        let screen = Rect::new(0, 0, 200, 100);
        let mut cs = ClientSet::try_new(LayoutStack::default(), tags, vec![screen]).unwrap();

        for n in 0..4 {
            cs.insert(Xid(n));
        }

        let r = Rect::new(50, 50, 50, 50);
        cs.float_unchecked(Xid(1), r);

        let positions = cs.visible_client_positions();

        assert!(positions.contains(&(Xid(1), r)))
    }
}
