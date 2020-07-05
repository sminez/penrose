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
    cix: usize, // currently selected layout
    lix: usize, // currently focused client
}

impl Workspace {
    pub fn new(name: &'static str, layouts: Vec<Layout>) -> Workspace {
        Workspace {
            name,
            clients: vec![],
            layouts,
            cix: 0,
            lix: 0,
        }
    }

    pub fn focused_client(&self) -> WinId {
        self.clients[self.cix]
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, id: WinId) {
        self.clients.insert(0, id);
        self.cix = 0;
    }

    /// Remove a target client, retaining focus at the same position in the stack
    pub fn remove_client(&mut self, id: WinId) {
        self.clients.retain(|c| *c != id);

        if self.cix >= self.clients.len() && self.clients.len() > 0 {
            self.cix -= 1;
        }
    }

    /// Remove the focused client, retaining focus at the same position in the stack
    pub fn remove_focused_client(&mut self) {
        self.remove_client(self.focused_client());
    }

    /// Run the current layout function, generating a list of resize actions to be
    /// applied byt the window manager.
    pub fn arrange(&self, monitor_region: &Region) -> Vec<ResizeAction> {
        let n_clients = self.clients.len();
        if n_clients > 0 {
            let layout = self.layouts[self.lix];
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
        self.lix = cycle_index(self.lix, self.layouts.len() - 1, direction);
    }

    pub fn cycle_client(&mut self, direction: Direction) -> Option<(WinId, WinId)> {
        if self.clients.len() == 0 {
            return None;
        }

        let previous = self.clients[self.cix];
        self.cix = cycle_index(self.cix, self.clients.len() - 1, direction);
        let current = self.clients[self.cix];

        Some((previous, current))
    }

    pub fn update_max_main(&mut self, change: Change) {
        self.layouts[self.lix].update_max_main(change);
    }

    pub fn update_main_ratio(&mut self, change: Change) {
        self.layouts[self.lix].update_main_ratio(change);
    }
}
