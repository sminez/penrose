use crate::data_types::{Region, ResizeAction, WinId};
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

    pub fn add_client(&mut self, id: WinId) {
        self.clients.push(id)
    }

    pub fn remove_client(&mut self, id: WinId) {
        self.clients.retain(|c| *c != id);
    }

    pub fn arrange(&self, monitor_region: &Region) -> Vec<ResizeAction> {
        let n_clients = self.clients.len();
        if n_clients > 0 {
            let layout = self.layouts[self.current_layout];
            log!(
                "applying {} layout for {} clients on workspace '{}'",
                layout.symbol,
                n_clients,
                self.name
            );
            layout.arrange(&self.clients, monitor_region)
        } else {
            vec![]
        }
    }

    pub fn cycle_layout(&mut self, forward: bool) {
        self.current_layout = cycle_index(self.current_layout, self.layouts.len() - 1, forward);
    }

    pub fn cycle_client(&mut self, forward: bool) {
        self.focused_client = cycle_index(self.focused_client, self.clients.len() - 1, forward);
    }

    pub fn inc_main(&mut self) {
        self.layouts[self.current_layout].update_max_main(true);
    }

    pub fn dec_main(&mut self) {
        self.layouts[self.current_layout].update_max_main(false);
    }

    pub fn inc_ratio(&mut self) {
        self.layouts[self.current_layout].update_main_ratio(true);
    }

    pub fn dec_ratio(&mut self) {
        self.layouts[self.current_layout].update_main_ratio(false);
    }
}
