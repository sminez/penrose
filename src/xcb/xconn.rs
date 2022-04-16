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
    common::{
        bindings::{KeyCode, MouseState},
        geometry::{Point, Region},
        Xid,
    },
    core::{manager::WindowManager, screen::Screen},
    xcb::{Api, Error},
    xconnection::{
        Atom, ClientAttr, ClientConfig, ClientMessage, ClientMessageKind, Prop, Result,
        WindowState, XConn, XEvent, XEventHandler,
    },
};
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/**
 * Handles communication with an X server via the XCB library.
 *
 * XcbConnection is a minimal implementation that does not make use of the full asyc capabilities
 * of the underlying C XCB library.
 **/
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct XcbConnection {
    check_win: Xid,
    api: Api,
}

impl XcbConnection {
    /// Establish a new connection to the running X server. Fails if unable to connect
    pub fn new() -> Result<Self> {
        let api = Api::new()?;
        let check_win = api.check_window();
        api.set_randr_notify_mask()?;

        Ok(Self { check_win, api })
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

crate::__xcb_impl_xatom_querier!(XcbConnection);
crate::__xcb_impl_xclientconfig!(XcbConnection);
crate::__xcb_impl_xclienthandler!(XcbConnection);
crate::__xcb_impl_xclientproperties!(XcbConnection);
crate::__xcb_impl_xeventhandler!(XcbConnection);
crate::__xcb_impl_xstate!(XcbConnection);

impl XConn for XcbConnection {
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        Ok(self.api.hydrate()?)
    }

    fn init(&self) -> Result<()> {
        Ok(self
            .api
            .set_client_attributes(self.api.root(), &[ClientAttr::RootEventMask])
            .map_err(|e| Error::Raw(format!("Unable to set root window event mask: {}", e)))?)
    }

    fn check_window(&self) -> Xid {
        self.check_win
    }

    fn cleanup(&self) -> Result<()> {
        self.api.ungrab_keys()?;
        self.api.ungrab_mouse_buttons()?;
        let net_name = Atom::NetActiveWindow.as_ref();
        self.api.delete_prop(self.api.root(), net_name)?;
        self.api.destroy_client(self.check_win)?;
        self.api.flush();

        Ok(())
    }

    fn grab_keys(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        self.api.grab_keys(key_codes)?;
        self.api.grab_mouse_buttons(mouse_states)?;
        self.flush();

        Ok(())
    }
}
