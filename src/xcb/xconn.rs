/*!
 *  API wrapper for talking to the X server using XCB
 *
 *  The crate used by penrose for talking to the X server is rust-xcb, which
 *  is a set of bindings for the C level XCB library that are autogenerated
 *  from an XML spec. The XML files can be found
 *  [here](https://github.com/rtbo/rust-xcb/tree/master/xml) and are useful
 *  as reference for how the API works. Sections have been converted and added
 *  to the documentation of the method calls and enums present in this module.
 *
 *  [EWMH](https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html)
 *  [Xlib manual](https://tronche.com/gui/x/xlib/)
 */
use crate::{
    core::{
        bindings::{KeyBindings, MouseBindings},
        data_types::{Point, Region, WinType},
        manager::WindowManager,
        screen::Screen,
        xconnection::{
            Atom, ClientAttr, ClientConfig, Prop, XClientConfig, XClientHandler, XClientProperties,
            XConn, XEvent, XEventHandler, XState, Xid,
        },
    },
    xcb::{Api, XcbError},
    Result,
};

use std::collections::HashMap;

/**
 * Handles communication with an X server via the XCB library.
 *
 * XcbConnection is a minimal implementation that does not make use of the full asyc capabilities
 * of the underlying C XCB library.
 **/
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct XcbConnection {
    api: Api,
    check_win: Xid,
}

impl XcbConnection {
    /// Establish a new connection to the running X server. Fails if unable to connect
    pub fn new() -> Result<Self> {
        let api = Api::new()?;

        api.set_randr_notify_mask()?;
        let check_win = api.create_window(WinType::CheckWin, Region::new(0, 0, 1, 1), false)?;

        Ok(Self { api, check_win })
    }

    /// Get a handle on the underlying [XCB Connection][::xcb::Connection] used by [Api]
    /// to communicate with the X server.
    pub fn xcb_connection(&self) -> &xcb::Connection {
        &self.api.conn()
    }

    /// Get a handle on the underlying [Api] to communicate with the X server.
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// Get a mutable handle on the underlying [Api] to communicate with the X server.
    pub fn api_mut(&mut self) -> &mut Api {
        &mut self.api
    }

    /// The current interned [Atom] values known to the underlying [Api] connection
    pub fn known_atoms(&self) -> &HashMap<Atom, u32> {
        &self.api.known_atoms()
    }
}

impl WindowManager<XcbConnection> {
    /// Get a handle on the underlying XCB Connection used by [Api] to communicate with the X
    /// server.
    pub fn xcb_connection(&self) -> &xcb::Connection {
        &self.conn().xcb_connection()
    }

    /// The current interned [Atom] values known to the underlying [XcbConnection]
    pub fn known_atoms(&self) -> &HashMap<Atom, u32> {
        &self.conn().known_atoms()
    }
}

impl XState for XcbConnection {
    fn root(&self) -> Xid {
        self.api.root()
    }

    fn current_screens(&self) -> Result<Vec<Screen>> {
        Ok(self.api.current_screens()?)
    }

    fn cursor_position(&self) -> Result<Point> {
        Ok(self.api.cursor_position()?)
    }

    fn warp_cursor(&self, win_id: Option<Xid>, screen: &Screen) -> Result<()> {
        let (x, y, id) = match win_id {
            Some(id) => {
                let (_, _, w, h) = self.client_geometry(id)?.values();
                ((w / 2), (h / 2), id)
            }
            None => {
                let (x, y, w, h) = screen.region(true).values();
                ((x + w / 2), (y + h / 2), self.api.root())
            }
        };

        Ok(self.api.warp_cursor(id, x as usize, y as usize)?)
    }

    fn client_geometry(&self, id: Xid) -> Result<Region> {
        Ok(self.api.client_geometry(id)?)
    }

    fn active_clients(&self) -> Result<Vec<Xid>> {
        Ok(self
            .api
            .current_clients()?
            .into_iter()
            .filter(|&id| self.is_managed_client(id))
            .collect())
    }

    fn focused_client(&self) -> Result<Xid> {
        Ok(self.api.focused_client()?)
    }

    fn atom_name(&self, atom: Xid) -> Result<String> {
        Ok(self.api.atom_name(atom)?)
    }
}

impl XEventHandler for XcbConnection {
    fn flush(&self) -> bool {
        self.api.flush()
    }

    fn wait_for_event(&self) -> Result<XEvent> {
        Ok(self.api.wait_for_event()?)
    }

    // FIXME: sending client events needs implementing
    fn send_client_event(&self, _id: Xid, _atom_name: &str, _data: &[u32]) -> Result<()> {
        todo!("work this out correctly")
    }
}

impl XClientHandler for XcbConnection {
    fn map_client(&self, id: Xid) -> Result<()> {
        Ok(self.api.map_client(id)?)
    }

    fn unmap_client(&self, id: Xid) -> Result<()> {
        Ok(self.api.unmap_client(id)?)
    }

    fn focus_client(&self, id: Xid) -> Result<()> {
        Ok(self.api.focus_client(id)?)
    }

    fn destroy_client(&self, id: Xid) -> Result<()> {
        Ok(self.api.destroy_client(id)?)
    }
}

impl XClientProperties for XcbConnection {
    fn get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
        Ok(self.api.get_prop(id, name)?)
    }

    fn list_props(&self, id: Xid) -> Result<Vec<String>> {
        Ok(self.api.list_props(id)?)
    }

    fn delete_prop(&self, id: Xid, name: &str) -> Result<()> {
        Ok(self.api.delete_prop(id, name)?)
    }

    fn change_prop(&self, id: Xid, prop: &str, val: Prop) -> Result<()> {
        Ok(self.api.change_prop(id, prop, val)?)
    }
}

impl XClientConfig for XcbConnection {
    fn configure_client(&self, id: Xid, data: &[ClientConfig]) -> Result<()> {
        Ok(self.api.configure_client(id, data)?)
    }

    fn set_client_attributes(&self, id: Xid, data: &[ClientAttr]) -> Result<()> {
        Ok(self.api.set_client_attributes(id, data)?)
    }
}

impl XConn for XcbConnection {
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        Ok(self.api.hydrate()?)
    }

    fn init(&self) -> Result<()> {
        Ok(self
            .api
            .set_client_attributes(self.api.root(), &[ClientAttr::RootEventMask])
            .map_err(|e| XcbError::Raw(format!("Unable to set root window event mask: {}", e)))?)
    }

    fn check_window(&self) -> Xid {
        self.api.check_window()
    }

    fn cleanup(&self) -> Result<()> {
        self.api.ungrab_keys()?;
        self.api.ungrab_mouse_buttons()?;
        self.api.destroy_client(self.check_win)?;
        let net_name = Atom::NetActiveWindow.as_ref();
        self.api.delete_prop(self.api.root(), net_name)?;
        self.api.flush();

        Ok(())
    }

    fn grab_keys(
        &self,
        key_bindings: &KeyBindings<Self>,
        mouse_bindings: &MouseBindings<Self>,
    ) -> Result<()> {
        self.api
            .grab_keys(&key_bindings.keys().collect::<Vec<_>>())?;
        self.api.grab_mouse_buttons(
            &mouse_bindings
                .keys()
                .map(|(_, state)| state)
                .collect::<Vec<_>>(),
        )?;
        self.flush();

        Ok(())
    }
}
