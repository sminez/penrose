//! Core data structures and user facing functionality for the window manager
use crate::{
    bindings::{KeyBindings, MouseBindings},
    geometry::Rect,
    handle,
    hooks::{EventHook, ManageHook, StateHook},
    layout::{Layout, LayoutStack},
    pure::{StackSet, Workspace},
    x::{XConn, XConnExt, XEvent},
    Color, Result,
};
use nix::sys::signal::{signal, SigHandler, Signal};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    ops::Deref,
};
use tracing::{error, span, trace, Level};

/// An X11 ID for a given resource
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

/// The pure client state information for the window manager
pub type ClientSet = StackSet<Xid>;

impl ClientSet {
    /// Run the per-workspace layouts to get a screen position for each visible client. Floating clients
    /// are placed above stacked clients, clients per workspace are stacked in the order they are returned
    /// from the layout.
    pub(crate) fn visible_client_positions(&mut self) -> Vec<(Xid, Rect)> {
        let mut positions: Vec<(Xid, Rect)> = self
            .iter_visible_clients()
            .flat_map(|c| self.floating.get(c).map(|r| (*c, *r)))
            .collect();

        for s in self.iter_screens_mut() {
            let r = s.visible_rect();
            let tag = &s.workspace.tag;
            let true_stack = s.workspace.stack.as_ref();
            let tiling = true_stack
                .and_then(|st| st.from_filtered(|c| !positions.iter().any(|(cc, _)| cc == c)));

            // TODO: if this supports using X state for determining layout position in future then this
            //       will be fallible and needs to fall back to a default layout.
            let (_, stack_positions) = s.workspace.layouts.layout_workspace(tag, &tiling, r);

            positions.extend(stack_positions.into_iter().rev());
        }

        positions
    }
}

/// The pure client state information for a single [Workspace]
pub type ClientSpace = Workspace<Xid>;

/// Mutable internal state for the window manager
#[derive(Debug)]
pub struct State<X, E = ()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    pub config: Config<X, E>,
    pub client_set: ClientSet,
    pub extension: E,
    pub(crate) root: Xid,
    pub(crate) mapped: HashSet<Xid>,
    pub(crate) pending_unmap: HashMap<Xid, usize>,
    pub(crate) current_event: Option<XEvent>,
    // pub(crate) mouse_focused: bool,
    // pub(crate) mouse_position: Option<(Point, Point)>,
}

impl<X, E> State<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    pub fn root(&self) -> Xid {
        self.root
    }

    pub fn mapped_clients(&self) -> &HashSet<Xid> {
        &self.mapped
    }

    pub fn current_event(&self) -> Option<&XEvent> {
        self.current_event.as_ref()
    }
}

pub struct Config<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    pub normal_border: Color,
    pub focused_border: Color,
    pub border_width: u32,
    pub focus_follow_mouse: bool,
    pub default_layouts: LayoutStack,
    pub workspace_names: Vec<String>,
    pub floating_classes: Vec<String>,
    pub event_hook: Option<Box<dyn EventHook<X, E>>>,
    pub manage_hook: Option<Box<dyn ManageHook<X>>>,
    pub refresh_hook: Option<Box<dyn StateHook<X, E>>>,
    pub startup_hook: Option<Box<dyn StateHook<X, E>>>,
}

impl<X, E> fmt::Debug for Config<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
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

impl<X, E> Default for Config<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
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
            event_hook: None,
            manage_hook: None,
            refresh_hook: None,
            startup_hook: None,
        }
    }
}

pub struct WindowManager<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    x: X,
    state: State<X, E>,
    key_bindings: KeyBindings<X, E>,
    mouse_bindings: MouseBindings<X, E>,
}

impl<X> WindowManager<X, ()>
where
    X: XConn,
{
    pub fn new(
        config: Config<X, ()>,
        key_bindings: KeyBindings<X, ()>,
        mouse_bindings: MouseBindings<X, ()>,
        x: X,
    ) -> Result<Self> {
        Self::new_with_state_extension(config, key_bindings, mouse_bindings, x, ())
    }
}

impl<X, E> WindowManager<X, E>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    pub fn new_with_state_extension(
        config: Config<X, E>,
        key_bindings: KeyBindings<X, E>,
        mouse_bindings: MouseBindings<X, E>,
        x: X,
        extension: E,
    ) -> Result<Self> {
        let client_set = StackSet::try_new(
            config.default_layouts.clone(),
            config.workspace_names.iter(),
            x.screen_details()?,
        )?;

        let state = State {
            config,
            client_set,
            extension,
            root: x.root(),
            mapped: HashSet::new(),
            pending_unmap: HashMap::new(),
            current_event: None,
        };

        Ok(Self {
            x,
            state,
            key_bindings,
            mouse_bindings,
        })
    }

    pub fn handle_xevent(&mut self, event: XEvent) -> Result<()> {
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
            if !h.call(&event, state, x) {
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
            Enter(p) => handle::enter(p.id, p.abs, state, x)?,
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

    pub fn run(mut self) -> Result<()> {
        trace!("registering SIGCHILD signal handler");
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) } {
            panic!("unable to set signal handler: {}", e);
        }

        self.grab()?;
        self.x.refresh(&mut self.state)?;

        if let Some(mut h) = self.state.config.startup_hook.take() {
            trace!("running user startup hook");
            h.call(&mut self.state, &self.x);
        }

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
}
