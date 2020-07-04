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
    clients: Vec<WinId>,
    layouts: Vec<Layout>,
    lix: usize, // currently selected layout
    cix: usize, // currently focused client
}

impl Workspace {
    pub fn new(layouts: Vec<Layout>) -> Workspace {
        Workspace {
            clients: vec![],
            layouts,
            lix: 0,
            cix: 0,
        }
    }

    pub fn arrange(&self, monitor_region: &Region) -> Vec<ResizeAction> {
        self.layouts[self.lix].arrange(&self.clients, monitor_region)
    }

    pub fn cycle_layout(&mut self, forward: bool) {
        self.lix = cycle_index(self.lix, self.layouts.len() - 1, forward);
    }

    pub fn cycle_client(&mut self, forward: bool) {
        self.cix = cycle_index(self.cix, self.clients.len() - 1, forward);
    }

    pub fn inc_main(&mut self) {
        self.layouts[self.lix].update_max_main(true);
    }

    pub fn dec_main(&mut self) {
        self.layouts[self.lix].update_max_main(false);
    }

    pub fn inc_ratio(&mut self) {
        self.layouts[self.lix].update_main_ratio(true);
    }

    pub fn dec_ratio(&mut self) {
        self.layouts[self.lix].update_main_ratio(false);
    }
}
