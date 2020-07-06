use crate::client::Client;
use crate::data_types::{Change, ColorScheme, Direction, Region, ResizeAction, WinId};
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
    clients: Vec<Client>,
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

    pub fn focused_client(&self) -> Option<&Client> {
        if self.clients.len() > 0 {
            Some(&self.clients[self.cix])
        } else {
            None
        }
    }

    pub fn focused_client_mut(&mut self) -> Option<&mut Client> {
        if self.clients.len() > 0 {
            Some(&mut self.clients[self.cix])
        } else {
            None
        }
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, c: Client) {
        self.clients.insert(0, c);
        self.cix = 0;
    }

    /// Remove a target client, retaining focus at the same position in the stack
    pub fn remove_client(&mut self, id: WinId) -> Option<Client> {
        let mut ix = None;

        for (i, c) in self.clients.iter().enumerate() {
            if c.id == id {
                ix = Some(i)
            }
        }

        match ix {
            None => None,
            Some(i) => {
                let removed = Some(self.clients.remove(i));
                if self.cix >= self.clients.len() && self.clients.len() > 0 {
                    self.cix -= 1;
                }

                removed
            }
        }
    }

    /// Remove the focused client, retaining focus at the same position in the stack
    pub fn remove_focused_client(&mut self) -> Option<Client> {
        self.remove_client_by_index(self.cix)
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

    pub fn cycle_client(
        &mut self,
        direction: Direction,
        conn: &xcb::Connection,
        color_scheme: &ColorScheme,
    ) {
        if self.clients.len() > 0 {
            self.clients[self.cix].unfocus(conn, color_scheme);
            self.cix = cycle_index(self.cix, self.clients.len() - 1, direction);
            self.clients[self.cix].focus(conn, color_scheme);
        }
    }

    pub fn update_max_main(&mut self, change: Change) {
        self.layouts[self.lix].update_max_main(change);
    }

    pub fn update_main_ratio(&mut self, change: Change, step: f32) {
        self.layouts[self.lix].update_main_ratio(change, step);
    }

    /// Place this workspace's windows onto a screen
    pub fn map_clients(&self, conn: &xcb::Connection) {
        for c in self.clients.iter() {
            xcb::map_window(conn, c.id);
        }
    }

    /// Remove this workspace's windows from a screen
    pub fn unmap_clients(&self, conn: &xcb::Connection) {
        for c in self.clients.iter() {
            xcb::unmap_window(conn, c.id);
        }
    }

    pub fn focus_client(&mut self, id: WinId, conn: &xcb::Connection, color_scheme: &ColorScheme) {
        for c in self.clients.iter_mut() {
            match (c.id == id, c.is_focused) {
                (true, false) => c.focus(conn, color_scheme),
                (false, true) => c.unfocus(conn, color_scheme),
                (_, _) => (),
            }
        }
    }

    fn remove_client_by_index(&mut self, ix: usize) -> Option<Client> {
        if self.clients.len() > 0 {
            Some(self.clients.remove(ix))
        } else {
            None
        }
    }
}
