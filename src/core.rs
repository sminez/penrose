//! Core data structures and user facing functionality for the window manager
use crate::{
    bindings::{KeyBindings, MouseBindings},
    geometry::{Point, Rect},
    layout::{Layout, LayoutStack},
    stack_set::{StackSet, Workspace},
    x::XEvent,
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

impl ClientSet {
    pub(crate) fn snapshot(&self) -> ClientSnapshot {
        ClientSnapshot {
            focus: self.current_client().copied(),
            visible_clients: self.iter_visible_clients().cloned().collect(),
            hidden_clients: self.iter_hidden_clients().cloned().collect(),
            visible_tags: self
                .iter_visible_workspaces()
                .map(|w| w.tag.clone())
                .collect(),
        }
    }

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

pub(crate) struct ClientSnapshot {
    pub(crate) focus: Option<Xid>,
    pub(crate) visible_clients: HashSet<Xid>,
    pub(crate) hidden_clients: HashSet<Xid>,
    pub(crate) visible_tags: HashSet<String>,
}

impl ClientSnapshot {
    pub(crate) fn all_clients(&self) -> impl Iterator<Item = &Xid> {
        self.visible_clients
            .iter()
            .chain(self.hidden_clients.iter())
    }
}

/// The pure client state information for a single [Workspace]
pub type ClientSpace = Workspace<Xid>;

/// Mutable internal state for the window manager
// #[derive(Debug)]
pub struct State {
    pub(crate) config: Config,
    pub(crate) client_set: ClientSet,
    pub(crate) root: Xid,
    pub(crate) mouse_focused: bool,
    pub(crate) mouse_position: Option<(Point, Point)>,
    pub(crate) current_event: Option<XEvent>,
    pub(crate) mapped: HashSet<Xid>,
    pub(crate) pending_unmap: HashMap<Xid, usize>,
}

// #[derive(Debug)]
pub struct Config {
    pub normal_border: Color,
    pub focused_border: Color,
    pub border_width: u32,
    pub focus_follow_mouse: bool,
    pub default_layouts: LayoutStack,
    pub workspace_names: Vec<String>,
    pub floating_classes: Vec<String>,
    pub key_bindings: KeyBindings,
    pub mouse_bindings: MouseBindings,
    // pub manage_hook: Box<dyn ManageHook>,
    // pub event_hook: Box<dyn EventHook>,
    // pub startup_hook: Box<dyn StartupHook>,
}

pub struct WindowManager {
    pub(crate) state: State,
}

// Launch
