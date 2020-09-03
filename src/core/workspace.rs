//! A Workspace is a set of displayed clients and a set of Layouts for arranging them
use crate::{
    client::Client,
    data_types::{Change, Direction, Region, ResizeAction, Ring, Selector, WinId},
    layout::{Layout, LayoutConf},
};

use std::collections::HashMap;

/**
 * A Workspace represents a named set of clients that are tiled according
 * to a specific layout. Layout properties are tracked per workspace and
 * clients are referenced by ID. Workspaces are independant of monitors and
 * can be moved between monitors freely, bringing their clients with them.
 *
 * The parent WindowManager struct tracks which client is focused from the
 * point of view of the X server by checking focus at the Workspace level
 * whenever a new Workspace becomes active.
 */
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    name: String,
    floating_clients: Ring<WinId>,
    tiled_clients: Ring<WinId>,
    focus: WorkspaceFocus,
    layouts: Ring<Layout>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WorkspaceFocus {
    Floating,
    Tiled,
}

impl Workspace {
    /// Construct a new Workspace with the given name and choice of Layouts
    pub fn new(name: impl Into<String>, layouts: Vec<Layout>) -> Workspace {
        if layouts.len() == 0 {
            panic!("{}: require at least one layout function", name.into());
        }

        Workspace {
            name: name.into(),
            floating_clients: Ring::new(Vec::new()),
            tiled_clients: Ring::new(Vec::new()),
            focus: WorkspaceFocus::Tiled,
            layouts: Ring::new(layouts),
        }
    }

    /// The name of this workspace
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// The number of clients currently on this workspace
    pub fn len(&self) -> usize {
        self.tiled_clients.len() + self.floating_clients.len()
    }

    /// Iterate over the clients on this workspace in position order
    pub fn iter(&self) -> impl Iterator<Item = &WinId> {
        self.tiled_clients.iter()
            .chain(self.floating_clients.iter())
    }

    /// Iterate over the clients on this workspace in position order
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WinId> {
        self.tiled_clients.iter_mut()
            .chain(self.floating_clients.iter_mut())
    }

    pub(crate) fn clients(&self) -> Vec<WinId> {
        self.iter()
            .cloned()
            .collect()
    }

    /// A reference to the currently focused client if there is one
    pub fn focused_client(&self) -> Option<WinId> {
        match self.focus {
            WorkspaceFocus::Tiled => {
                self.tiled_clients.focused().map(|c| *c)
            }
            WorkspaceFocus::Floating => {
                self.floating_clients.focused().map(|c| *c)
            }
        }
    }

    /// Toggles WorkspaceFocus between Tiled and Floating
    pub fn toggle_focus(&mut self) {
        match self.focus {
            WorkspaceFocus::Tiled if self.floating_clients.len() > 0 => {
                self.focus = WorkspaceFocus::Floating;
            }
            _ => {
                self.focus = WorkspaceFocus::Tiled;
            }
        }
        debug!("WorkspaceFocus::{:?}", self.focus);
    }

    pub(crate) fn toggle_client_floating(&mut self, id: WinId) {
        info!("window swapped floating {}", id);
        if self.tiled_clients.focused() == Some(&id) {
            self.tiled_clients.remove(&Selector::Focused);
            self.floating_clients.push(id);
        } else if self.floating_clients.focused() == Some(&id) {
            self.floating_clients.remove(&Selector::Focused);
            self.tiled_clients.push(id);
        } else {
            error!("unreachable");
        }
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, id: WinId) {
        match self.focus {
            WorkspaceFocus::Tiled => {
                self.tiled_clients.insert(0, id);
            }
            WorkspaceFocus::Floating => {
                self.floating_clients.insert(0, id);
            }
        }
    }

    /// Focus the client with the given id, returns an option of the previously focused
    /// client if there was one
    pub fn focus_client(&mut self, id: WinId) -> Option<WinId> {
        let prev = match self.focus {
            WorkspaceFocus::Tiled => self.tiled_clients.focused().copied(),
            WorkspaceFocus::Floating => self.floating_clients.focused().copied(),
        };

        if self.tiled_clients.focus(&Selector::Condition(&|c| *c == id)).is_none() {
            self.floating_clients.focus(&Selector::Condition(&|c| *c == id));
        }

        prev
    }

    /// Remove a target client, retaining focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_client(&mut self, id: WinId) -> Option<WinId> {
        self.tiled_clients.remove(&Selector::Condition(&|c| *c == id))
            .or_else(|| self.floating_clients.remove(&Selector::Condition(&|c| *c == id)))
    }

    /// Remove the currently focused client, keeping focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_focused_client(&mut self) -> Option<WinId> {
        self.tiled_clients.remove(&Selector::Focused)
            .or_else(|| self.floating_clients.remove(&Selector::Focused))
    }

    /// Run the current layout function, generating a list of resize actions to be
    /// applied byt the window manager.
    pub(crate) fn arrange(
        &self,
        screen_region: Region,
        client_map: &HashMap<WinId, Client>,
    ) -> Vec<ResizeAction> {
        if self.tiled_clients.len() > 0 {
            let layout = self.layouts.focused().unwrap();
            let tiled_clients: Vec<&Client> = self
                .tiled_clients
                .iter()
                .map(|id| client_map.get(id).unwrap())
                .collect();
            debug!(
                "applying '{}' layout for {} clients on workspace '{}'",
                layout.symbol,
                self.tiled_clients.len(),
                self.name
            );
            layout.arrange(&tiled_clients, self.focused_client(), &screen_region)
        } else {
            vec![]
        }
    }

    /// Set the active layout by symbol name if it is available. Returns a reference to active
    /// layout if it was able to be set.
    pub fn try_set_layout(&mut self, symbol: &str) -> Option<&Layout> {
        self.layouts
            .focus(&Selector::Condition(&|l| l.symbol == symbol))
    }

    /// Cycle through the available layouts on this workspace
    pub fn cycle_layout(&mut self, direction: Direction) -> &str {
        self.layouts.cycle_focus(direction);
        self.layout_symbol()
    }

    /// The symbol of the currently used layout (passed on creation)
    pub fn layout_symbol(&self) -> &str {
        self.layouts.focused().unwrap().symbol
    }

    /**
     * The LayoutConf of the currently active Layout. Used by the WindowManager to
     * determine when and how the layout function should be applied.
     */
    pub fn layout_conf(&self) -> LayoutConf {
        self.layouts.focused().unwrap().conf
    }

    /// Cycle focus through the clients on this workspace
    pub fn cycle_client(&mut self, direction: Direction) -> Option<(WinId, WinId)> {
        let clients = match self.focus {
            WorkspaceFocus::Tiled => {
                if self.tiled_clients.len() < 2 {
                    return None; // need at least two clients to cycle
                }

                if !self.layout_conf().allow_wrapping && self.tiled_clients.would_wrap(direction) {
                    return None;
                }

                &mut self.tiled_clients
            }
            WorkspaceFocus::Floating => {
                if self.floating_clients.len() < 2 {
                    return None; // need at least two clients to cycle
                }

                if !self.layout_conf().allow_wrapping && self.floating_clients.would_wrap(direction) {
                    return None;
                }

                &mut self.floating_clients
            }
        };

        let prev = *clients.focused()?;
        let new = *clients.cycle_focus(direction)?;

        if prev != new {
            Some((prev, new))
        } else {
            None
        }
    }

    /**
     * Drag the focused client through the stack, retaining focus
     */
    pub fn drag_client(&mut self, direction: Direction) -> Option<WinId> {
        if matches!(self.focus, WorkspaceFocus::Floating) {
            return None;
        }
        if !self.layout_conf().allow_wrapping && self.tiled_clients.would_wrap(direction) {
            return None;
        }
        self.tiled_clients.drag_focused(direction).map(|c| *c)
    }

    /// Increase or decrease the number of possible clients in the main area of the current Layout
    pub fn update_max_main(&mut self, change: Change) {
        if let Some(layout) = self.layouts.focused_mut() {
            layout.update_max_main(change);
        }
    }

    /// Increase or decrease the size of the main area for the current Layout
    pub fn update_main_ratio(&mut self, change: Change, step: f32) {
        if let Some(layout) = self.layouts.focused_mut() {
            layout.update_main_ratio(change, step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::Direction;
    use crate::layout::*;

    fn test_layouts() -> Vec<Layout> {
        vec![Layout::new("t", LayoutConf::default(), mock_layout, 1, 0.6)]
    }

    fn add_n_clients(ws: &mut Workspace, n: usize) {
        for i in 0..n {
            let k = ((i + 1) * 10) as u32; // ensure win_id != index
            ws.add_client(k);
        }
    }

    #[test]
    fn ref_to_focused_client_when_empty() {
        let ws = Workspace::new("test", test_layouts());
        assert_eq!(ws.focused_client(), None);
    }

    #[test]
    fn ref_to_focused_client_when_populated() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![42, 123]);

        let c = ws.focused_client().expect("should have had a client for 0");
        assert_eq!(c, 42);

        ws.clients.cycle_focus(Direction::Forward);
        let c = ws.focused_client().expect("should have had a client for 1");
        assert_eq!(c, 123);
    }

    #[test]
    fn removing_a_client_when_present() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![13, 42]);

        let removed = ws
            .remove_client(42)
            .expect("should have had a client for id=42");
        assert_eq!(removed, 42);
    }

    #[test]
    fn removing_a_client_when_not_present() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![13]);

        let removed = ws.remove_client(42);
        assert_eq!(removed, None, "got a client by the wrong ID");
    }

    #[test]
    fn adding_a_client() {
        let mut ws = Workspace::new("test", test_layouts());
        add_n_clients(&mut ws, 3);
        let ids: Vec<WinId> = ws.clients.iter().map(|c| *c).collect();
        assert_eq!(ids, vec![30, 20, 10], "not pushing at the top of the stack")
    }

    #[test]
    fn applying_a_layout_gives_one_action_per_client() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![1, 2, 3]);
        let client_map = map! {
            1 => Client::new(1, "".into(), "".into(), 1, false),
            2 => Client::new(2, "".into(), "".into(), 1, false),
            3 => Client::new(3, "".into(), "".into(), 1, false),
        };
        let actions = ws.arrange(Region::new(0, 0, 2000, 1000), &client_map);
        assert_eq!(actions.len(), 3, "actions are not 1-1 for clients")
    }

    #[test]
    fn dragging_a_client_forward() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![1, 2, 3, 4]);
        assert_eq!(ws.focused_client(), Some(1));

        assert_eq!(ws.drag_client(Direction::Forward), Some(1));
        assert_eq!(ws.clients.as_vec(), vec![2, 1, 3, 4]);

        assert_eq!(ws.drag_client(Direction::Forward), Some(1));
        assert_eq!(ws.clients.as_vec(), vec![2, 3, 1, 4]);

        assert_eq!(ws.drag_client(Direction::Forward), Some(1));
        assert_eq!(ws.clients.as_vec(), vec![2, 3, 4, 1]);

        assert_eq!(ws.drag_client(Direction::Forward), Some(1));
        assert_eq!(ws.clients.as_vec(), vec![1, 2, 3, 4]);

        assert_eq!(ws.focused_client(), Some(1));
    }

    #[test]
    fn dragging_non_index_0_client_backward() {
        let mut ws = Workspace::new("test", test_layouts());
        ws.clients = Ring::new(vec![1, 2, 3, 4]);
        ws.focus_client(3);
        assert_eq!(ws.focused_client(), Some(3));

        assert_eq!(ws.drag_client(Direction::Backward), Some(3));
        assert_eq!(ws.clients.as_vec(), vec![1, 3, 2, 4]);

        assert_eq!(ws.drag_client(Direction::Backward), Some(3));
        assert_eq!(ws.clients.as_vec(), vec![3, 1, 2, 4]);

        assert_eq!(ws.drag_client(Direction::Backward), Some(3));
        assert_eq!(ws.clients.as_vec(), vec![1, 2, 4, 3]);

        assert_eq!(ws.drag_client(Direction::Backward), Some(3));
        assert_eq!(ws.clients.as_vec(), vec![1, 2, 3, 4]);

        assert_eq!(ws.focused_client(), Some(3));
    }
}
