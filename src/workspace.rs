//! A Workspace is a set of displayed clients and a set of Layouts for arranging them
use crate::data_types::{Change, Direction, Region, ResizeAction, Ring, WinId};
use crate::layout::Layout;

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
#[derive(Debug)]
pub struct Workspace {
    name: &'static str,
    clients: Ring<WinId>,
    layouts: Ring<Layout>,
}

impl Workspace {
    pub fn new(name: &'static str, layouts: Vec<Layout>) -> Workspace {
        if layouts.len() == 0 {
            panic!("{}: require at least one layout function", name);
        }

        Workspace {
            name,
            clients: Ring::new(Vec::new()),
            layouts: Ring::new(layouts),
        }
    }

    /// The number of clients currently on this workspace
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Iterate over the clients on this workspace in position order
    pub fn iter(&self) -> std::slice::Iter<WinId> {
        self.clients.iter()
    }

    /// A reference to the currently focused client if there is one
    pub fn focused_client(&self) -> Option<&WinId> {
        self.clients.focused()
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, id: WinId) {
        self.clients.insert(0, id);
    }

    /// Focus the client with the given id, returns an option of the previously focused
    /// client if there was one
    pub fn focus_client(&mut self, id: WinId) -> Option<WinId> {
        if self.clients.len() == 0 {
            return None;
        }

        let prev = self.clients.focused().unwrap().clone();
        self.clients.focus_by(|c| c == &id);
        Some(prev)
    }

    /// Remove a target client, retaining focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_client(&mut self, id: WinId) -> Option<WinId> {
        self.clients.remove_by(|c| c == &id)
    }

    /// Remove the currently focused client, keeping focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_focused_client(&mut self) -> Option<WinId> {
        self.clients.remove_focused()
    }

    /// Run the current layout function, generating a list of resize actions to be
    /// applied byt the window manager.
    pub fn arrange(&self, screen_region: &Region) -> Vec<ResizeAction> {
        let n_clients = self.clients.len();
        if n_clients > 0 {
            let layout = self.layouts.focused().unwrap();
            debug!(
                "applying '{}' layout for {} clients on workspace '{}'",
                layout.symbol, n_clients, self.name
            );
            layout.arrange(self.clients.as_vec(), screen_region)
        } else {
            vec![]
        }
    }

    /// Cycle through the available layouts on this workspace
    pub fn cycle_layout(&mut self, direction: Direction) {
        self.layouts.cycle_focus(direction);
    }

    /// Cycle focus through the clients on this workspace
    pub fn cycle_client(&mut self, direction: Direction) -> Option<(WinId, WinId)> {
        if self.clients.len() <= 1 {
            return None;
        }

        let prev = self.clients.focused().unwrap().clone();
        self.clients.cycle_focus(direction);
        let new = self.clients.focused().unwrap();

        if prev != *new {
            Some((prev, *new))
        } else {
            None
        }
    }

    pub fn update_max_main(&mut self, change: Change) {
        if let Some(layout) = self.layouts.focused_mut() {
            layout.update_max_main(change);
        }
    }

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
        vec![Layout::new("t", LayoutKind::Normal, mock_layout, 1, 0.6)]
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
        assert_eq!(*c, 42);

        ws.clients.cycle_focus(Direction::Forward);
        let c = ws.focused_client().expect("should have had a client for 1");
        assert_eq!(*c, 123);
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
        add_n_clients(&mut ws, 3);
        let actions = ws.arrange(&Region::new(0, 0, 2000, 1000));
        assert_eq!(actions.len(), 3, "actions are not 1-1 for clients")
    }
}
