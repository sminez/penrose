use crate::data_types::{Change, Direction, Region, ResizeAction, WinId};
use crate::helpers::cycle_index;
use crate::layout::Layout;

/**
 * A Workspace represents a named set of clients that are tiled according
 * to a specific layout. Layout properties are tracked per workspace and
 * clients are referenced by ID. Workspaces are independant of monitors and
 * can be moved between monitors freely, bringing their clients with them.
 */
pub struct Workspace {
    name: &'static str,
    clients: Vec<WinId>,
    layouts: Vec<Layout>,
    current_layout: usize, // currently selected layout
    focused_client: usize, // currently focused client
}

impl Workspace {
    pub fn new(name: &'static str, layouts: Vec<Layout>) -> Workspace {
        Workspace {
            name,
            clients: vec![],
            layouts,
            current_layout: 0,
            focused_client: 0,
        }
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, id: WinId) {
        self.clients.insert(0, id);
        self.focused_client = 0;
    }

    /// Remove a target client, retaining focus at the same position in the stack
    pub fn remove_client(&mut self, id: WinId) {
        self.clients.retain(|c| *c != id);

        if self.focused_client >= self.clients.len() && self.clients.len() > 0 {
            self.focused_client -= 1;
        }
    }

    /// Remove the focused client, retaining focus at the same position in the stack
    pub fn remove_focused_client(&mut self) {
        self.remove_client(self.clients[self.focused_client]);
    }

    /// Run the current layout function, generating a list of resize actions to be
    /// applied byt the window manager.
    pub fn arrange(&self, monitor_region: &Region) -> Vec<ResizeAction> {
        let n_clients = self.clients.len();
        if n_clients > 0 {
            let layout = self.layouts[self.current_layout];
            debug!(
                "applying {} layout for {} clients on workspace '{}'",
                layout.symbol, n_clients, self.name
            );
            layout.arrange(&self.clients, monitor_region)
        } else {
            vec![]
        }
    }

    pub fn cycle_layout(&mut self, direction: Direction) {
        self.current_layout = cycle_index(self.current_layout, self.layouts.len() - 1, direction);
    }

    pub fn cycle_client(&mut self, direction: Direction) -> (WinId, WinId) {
        let previous = self.clients[self.focused_client];
        self.focused_client = cycle_index(self.focused_client, self.clients.len() - 1, direction);
        let current = self.clients[self.focused_client];

        (previous, current)
    }

    pub fn update_max_main(&mut self, change: Change) {
        self.layouts[self.current_layout].update_max_main(change);
    }

    pub fn update_main_ratio(&mut self, change: Change) {
        self.layouts[self.current_layout].update_main_ratio(change);
    }
}
