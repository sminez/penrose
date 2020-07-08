use crate::client::Client;
use crate::data_types::{Change, ColorScheme, Direction, Region, ResizeAction, Ring, WinId};
use crate::layout::Layout;
use crate::xconnection::XConn;

/**
 * A Workspace represents a named set of clients that are tiled according
 * to a specific layout. Layout properties are tracked per workspace and
 * clients are referenced by ID. Workspaces are independant of monitors and
 * can be moved between monitors freely, bringing their clients with them.
 */
pub struct Workspace {
    name: &'static str,
    clients: Ring<Client>,
    layouts: Ring<Layout>,
}

impl Workspace {
    pub fn new(name: &'static str, layouts: Vec<Layout>) -> Workspace {
        Workspace {
            name,
            clients: Ring::new(Vec::new()),
            layouts: Ring::new(layouts),
        }
    }

    /// A reference to the currently focused client if there is one
    pub fn focused_client(&self) -> Option<&Client> {
        self.clients.focused()
    }

    /// A mutable reference to the currently focused client if there is one
    pub fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.clients.focused_mut()
    }

    /// Add a new client to this workspace at the top of the stack and focus it
    pub fn add_client(&mut self, c: Client) {
        self.clients.insert(0, c);
    }

    /// Remove a target client, retaining focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_client(&mut self, id: WinId) -> Option<Client> {
        self.clients.remove_by(|c| c.id() == id)
    }

    /// Remove the currently focused client, keeping focus at the same position in the stack.
    /// Returns the removed client if there was one to remove.
    pub fn remove_focused_client(&mut self) -> Option<Client> {
        self.clients.remove_focused()
    }

    /// Run the current layout function, generating a list of resize actions to be
    /// applied byt the window manager.
    pub fn arrange(&self, monitor_region: &Region) -> Vec<ResizeAction> {
        let n_clients = self.clients.len();
        if n_clients > 0 {
            let layout = self.layouts.focused().unwrap();
            debug!(
                "applying {} layout for {} clients on workspace '{}'",
                layout.symbol, n_clients, self.name
            );
            layout.arrange(self.clients.as_vec(), monitor_region)
        } else {
            vec![]
        }
    }

    pub fn cycle_layout(&mut self, direction: Direction) {
        self.layouts.cycle_focus(direction);
    }

    pub fn cycle_client(
        &mut self,
        direction: Direction,
        conn: &dyn XConn,
        color_scheme: &ColorScheme,
    ) {
        if self.clients.len() > 1 {
            if let Some(client) = self.clients.focused_mut() {
                client.unfocus(conn, color_scheme);
            }
            self.clients.cycle_focus(direction);
            if let Some(client) = self.clients.focused_mut() {
                client.unfocus(conn, color_scheme);
            }
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

    /// Place this workspace's windows onto a screen
    pub fn map_clients(&self, conn: &dyn XConn) {
        for c in self.clients.iter() {
            debug!("mapping {} on ws {}", c.id(), self.name);
            conn.map_window(c.id());
        }
    }

    /// Remove this workspace's windows from a screen
    pub fn unmap_clients(&self, conn: &dyn XConn) {
        for c in self.clients.iter() {
            debug!("unmapping {} on ws {}", c.id(), self.name);
            conn.unmap_window(c.id());
        }
    }

    pub fn focus_client(&mut self, id: WinId, conn: &dyn XConn, color_scheme: &ColorScheme) {
        for c in self.clients.iter_mut() {
            match (c.id() == id, c.is_focused()) {
                (true, false) => c.focus(conn, color_scheme),
                (false, true) => c.unfocus(conn, color_scheme),
                (_, _) => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;
    use crate::data_types::Direction;

    #[test]
    fn ref_to_focused_client_when_empty() {
        let ws = Workspace::new("test", vec![]);
        assert_eq!(ws.focused_client(), None);
    }

    #[test]
    fn ref_to_focused_client_when_populated() {
        let mut ws = Workspace::new("test", vec![]);
        ws.clients = Ring::new(vec![
            Client::new(42, "focused first".into(), false),
            Client::new(123, "focused second".into(), false),
        ]);

        let c = ws.focused_client().expect("should have had a client for 0");
        assert_eq!(c.id(), 42);
        assert_eq!(c.class(), "focused first");

        ws.clients.cycle_focus(Direction::Forward);
        let c = ws.focused_client().expect("should have had a client for 1");
        assert_eq!(c.id(), 123);
        assert_eq!(c.class(), "focused second");
    }

    #[test]
    fn removing_a_client_when_present() {
        let mut ws = Workspace::new("test", vec![]);
        ws.clients = Ring::new(vec![
            Client::new(13, "retained".into(), false),
            Client::new(42, "removed".into(), false),
        ]);

        let removed = ws
            .remove_client(42)
            .expect("should have had a client for id=42");
        assert_eq!(removed.id(), 42);
        assert_eq!(removed.class(), "removed");
    }

    #[test]
    fn removing_a_client_when_not_present() {
        let mut ws = Workspace::new("test", vec![]);
        ws.clients = Ring::new(vec![Client::new(13, "retained".into(), false)]);

        let removed = ws.remove_client(42);
        assert_eq!(removed, None);
    }

    #[test]
    fn adding_a_client() {
        let mut ws = Workspace::new("test", vec![]);
        let c1 = Client::new(10, "first".into(), false);
        let c2 = Client::new(20, "second".into(), false);
        let c3 = Client::new(30, "third".into(), false);
        ws.add_client(c1);
        ws.add_client(c2);
        ws.add_client(c3);

        let ids: Vec<WinId> = ws.clients.iter().map(|c| c.id()).collect();
        assert_eq!(ids, vec![30, 20, 10], "not pushing at the top of the stack")
    }
}
