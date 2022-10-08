//! Core data structures and user facing functionality for the window manager
use crate::{
    bindings::{KeyBindings, MouseBindings},
    geometry::{Point, Rect},
    handle,
    layout::{Layout, LayoutStack},
    stack_set::{StackSet, Workspace},
    x::{XConnExt, XEvent},
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

#[derive(Debug)]
pub struct Config {
    pub normal_border: Color,
    pub focused_border: Color,
    pub border_width: u32,
    pub focus_follow_mouse: bool,
    pub default_layouts: LayoutStack,
    pub workspace_names: Vec<String>,
    pub floating_classes: Vec<String>,
    // pub manage_hook: Box<dyn ManageHook>,
    // pub event_hook: Box<dyn EventHook>,
    // pub startup_hook: Box<dyn StartupHook>,
}

pub struct WindowManager {
    pub(crate) state: State,
    pub(crate) key_bindings: KeyBindings,
    pub(crate) mouse_bindings: MouseBindings,
}

impl WindowManager {
    pub fn handle_xevent<X>(&mut self, x: &X, event: XEvent)
    where
        X: XConnExt,
    {
        let WindowManager {
            state,
            key_bindings,
            mouse_bindings,
        } = self;

        match event {
            XEvent::ClientMessage(m) => handle::client_message(m, state, x),
            XEvent::ConfigureNotify(e) => todo!(),
            XEvent::ConfigureRequest(e) => todo!(),
            XEvent::Enter(p) => todo!(),
            XEvent::Expose(e) => todo!(),
            XEvent::FocusIn(id) => todo!(),
            XEvent::Destroy(xid) => handle::destroy(xid, state, x),
            XEvent::KeyPress(code) => handle::keypress(code, key_bindings, state, x),
            XEvent::Leave(p) => todo!(),
            XEvent::MapRequest(xid) => handle::map_request(xid, state, x),
            XEvent::MouseEvent(e) => todo!(),
            XEvent::PropertyNotify(e) => todo!(),
            XEvent::RandrNotify => todo!(),
            XEvent::ScreenChange => todo!(),
            XEvent::UnmapNotify(xid) => handle::unmap_notify(xid, state, x),
            // MappingNotify for changes to keyboard mappings
        }
    }
}
