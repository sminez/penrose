//! Core data structures and user facing functionality for the window manager
use crate::{
    bindings::{KeyBindings, KeyCodeMask, MouseBindings},
    geometry::Point,
    layout::LayoutStack,
    stack_set::{StackSet, Workspace},
    Color,
};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

/// An X11 ID for a given resource
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
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

/// The pure client state information for a single [Workspace]
pub type ClientSpace = Workspace<Xid>;

/// Mutable internal state for the window manager
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MutableState {
    /// The pure stacking information for known clients
    pub client_set: ClientSet,
    /// The set of all currently mapped clients
    pub mapped: HashSet<Xid>,
    /// The number of expected Unmap events per client
    pub pending_unmap: HashMap<Xid, usize>,
    // dragging: Option<Fn(Point, Point) -> Result<()>>, // ?? need to look at this one
    pub num_lock_mask: KeyCodeMask,
    // extensible_state: HashMap<String, Box<dyn ExtensibleState>>,
}

pub struct ReadOnlyState<X> {
    config: Config<X>,
    root: Xid,
    mouse_focused: bool,
    mouse_position: Option<(Point, Point)>,
    // current_event: Option<Event>,
}

pub struct Config<X> {
    normal_border: Color,
    focused_border: Color,
    border_width: u32,
    focus_follow_mouse: bool,
    default_layouts: LayoutStack,
    workspace_names: Vec<String>,
    key_bindings: KeyBindings<X>,
    mouse_bindings: MouseBindings<X>,
    // manage_hook: Box<dyn ManageHook>,
    // event_hook: Box<dyn EventHook>,
    // startup_hook: Box<dyn StartupHook>,
}

pub struct WindowManager<X> {
    conn: X,
    mut_state: MutableState,
    ro_state: ReadOnlyState<X>,
}

// Launch
