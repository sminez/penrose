//! API wrapper for talking to the X11 server using x11rb
//!
//! This module contains the code for talking to the X11 server using the x11rb crate, which
//! offers an implementation of the X11 protocol in safe Rust. The actual protocol bindings are
//! autogenerated from an XML spec. The XML files can be found [in its xcb-proto-{something}
//! subfolder](https://github.com/psychon/x11rb) and are useful as a reference for how the API
//! works. x11rb also [offers](https://github.com/psychon/x11rb/blob/master/doc/generated_code.md)
//! some explanation on how the XML is turned into Rust code.

use crate::{
    common::{
        bindings::{KeyCode, MouseState},
        geometry::{Point, Region},
        Xid,
    },
    core::screen::Screen,
    x11rb::{atom::Atoms, Error},
    xconnection::{
        self, Atom, ClientAttr, ClientConfig, ClientEventMask, ClientMessage, ClientMessageKind,
        Prop, Result, WindowAttributes, WindowState, WmHints, WmNormalHints, XAtomQuerier,
        XClientConfig, XClientHandler, XClientProperties, XConn, XEvent, XEventHandler, XState,
    },
};

use std::{convert::TryFrom, str::FromStr};

use x11rb::{
    connection::Connection,
    protocol::{
        randr::{self, ConnectionExt as _},
        xproto::{
            AtomEnum, ButtonIndex, ChangeWindowAttributesAux, ClientMessageData,
            ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, EventMask,
            Grab, GrabMode, InputFocus, MapState, ModMask, PropMode, StackMode, WindowClass,
            CLIENT_MESSAGE_EVENT,
        },
    },
    wrapper::ConnectionExt as _,
    CURRENT_TIME,
};

const RANDR_VER: (u32, u32) = (1, 2);

/// Handles communication with an X server via the x11rb crate.
#[derive(Debug)]
pub struct X11rbConnection<C: Connection> {
    conn: C,
    root: Xid,
    check_win: Xid,
    atoms: Atoms,
}

impl<C: Connection> X11rbConnection<C> {
    /// Create a new X11rbConnection wrapping the given X11 server connection
    pub fn new_for_connection(conn: C) -> Result<Self> {
        let root = conn.setup().roots[0].root;
        conn.prefetch_extension_information(randr::X11_EXTENSION_NAME)?;
        let atoms = Atoms::new(&conn)?;

        if conn
            .extension_information(randr::X11_EXTENSION_NAME)?
            .is_none()
        {
            return Err(Error::Randr("RandR not supported".to_string()).into());
        }
        let randr_ver = conn
            .randr_query_version(RANDR_VER.0, RANDR_VER.1)?
            .reply()?;
        let (maj, min) = (randr_ver.major_version, randr_ver.minor_version);
        if (maj, min) != RANDR_VER {
            return Err(Error::Randr(format!(
                "penrose requires RandR version >= {}.{}: detected {}.{}\nplease update RandR to a newer version",
                RANDR_VER.0, RANDR_VER.1, maj, min
            )).into());
        }

        use randr::NotifyMask;
        let mask = NotifyMask::OUTPUT_CHANGE | NotifyMask::CRTC_CHANGE | NotifyMask::SCREEN_CHANGE;
        conn.randr_select_input(root, mask)?;

        let check_win = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            check_win,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new().override_redirect(1),
        )?;

        Ok(Self {
            conn,
            root,
            check_win,
            atoms,
        })
    }

    /// The root window ID
    pub fn root(&self) -> Xid {
        self.root
    }

    /// Get a handle to the underlying connection.
    pub fn connection(&self) -> &C {
        &self.conn
    }
}

impl<C: Connection> XAtomQuerier for X11rbConnection<C> {
    fn atom_name(&self, atom: Xid) -> Result<String> {
        // Is the atom already known?
        if let Some(atom) = self.atoms.atom_name(atom) {
            return Ok(atom.as_ref().to_string());
        }

        // Nope, ask the X11 server
        let reply = self.conn.get_atom_name(atom)?.reply()?;
        let name = String::from_utf8(reply.name).map_err(Error::from)?;
        Ok(name)
    }

    fn atom_id(&self, name: &str) -> Result<Xid> {
        if let Ok(known) = Atom::from_str(name) {
            return Ok(self.atoms.known_atom(known));
        }

        Ok(self.conn.intern_atom(false, name.as_bytes())?.reply()?.atom)
    }
}

impl<C: Connection> XClientConfig for X11rbConnection<C> {
    fn configure_client(&self, id: Xid, data: &[ClientConfig]) -> Result<()> {
        let mut aux = ConfigureWindowAux::new();
        for conf in data.iter() {
            match conf {
                ClientConfig::BorderPx(px) => aux = aux.border_width(*px),
                ClientConfig::Position(region) => {
                    let (x, y, w, h) = region.values();
                    aux = aux.x(x as i32).y(y as i32).width(w).height(h);
                }
                ClientConfig::StackAbove => aux = aux.stack_mode(StackMode::ABOVE),
            }
        }
        self.conn.configure_window(id, &aux)?;
        Ok(())
    }

    fn set_client_attributes(&self, id: Xid, data: &[ClientAttr]) -> Result<()> {
        let client_event_mask = EventMask::ENTER_WINDOW
            | EventMask::LEAVE_WINDOW
            | EventMask::PROPERTY_CHANGE
            | EventMask::STRUCTURE_NOTIFY;

        let root_event_mask = EventMask::PROPERTY_CHANGE
            | EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::BUTTON_MOTION;

        let mut aux = ChangeWindowAttributesAux::new();
        for conf in data.iter() {
            match conf {
                ClientAttr::BorderColor(c) => aux = aux.border_pixel(*c),
                ClientAttr::ClientEventMask => aux = aux.event_mask(client_event_mask),
                ClientAttr::RootEventMask => aux = aux.event_mask(root_event_mask),
            }
        }
        self.conn.change_window_attributes(id, &aux)?;
        Ok(())
    }

    fn get_window_attributes(&self, id: Xid) -> Result<WindowAttributes> {
        let win_attrs = self.conn.get_window_attributes(id)?.reply()?;
        let override_redirect = win_attrs.override_redirect;
        let map_state = match win_attrs.map_state {
            MapState::UNMAPPED => crate::xconnection::MapState::Unmapped,
            MapState::UNVIEWABLE => crate::xconnection::MapState::UnViewable,
            MapState::VIEWABLE => crate::xconnection::MapState::Viewable,
            _ => {
                return Err(xconnection::Error::Raw(format!(
                    "invalid map state: {:?}",
                    win_attrs.map_state
                )))
            }
        };
        let window_class = match win_attrs.class {
            WindowClass::COPY_FROM_PARENT => crate::xconnection::WindowClass::CopyFromParent,
            WindowClass::INPUT_OUTPUT => crate::xconnection::WindowClass::InputOutput,
            WindowClass::INPUT_ONLY => crate::xconnection::WindowClass::InputOnly,
            _ => {
                return Err(xconnection::Error::Raw(format!(
                    "invalid window class: {:?}",
                    win_attrs.class
                )))
            }
        };

        Ok(WindowAttributes::new(
            override_redirect,
            map_state,
            window_class,
        ))
    }
}

impl<C: Connection> XClientHandler for X11rbConnection<C> {
    fn map_client(&self, id: Xid) -> Result<()> {
        self.conn.map_window(id)?;
        Ok(())
    }

    fn unmap_client(&self, id: Xid) -> Result<()> {
        self.conn.unmap_window(id)?;
        Ok(())
    }

    fn focus_client(&self, id: Xid) -> Result<()> {
        self.conn
            .set_input_focus(InputFocus::PARENT, id, CURRENT_TIME)?;

        self.change_prop(
            self.root,
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![id]),
        )
    }

    fn destroy_client(&self, id: Xid) -> Result<()> {
        self.conn.destroy_window(id)?;
        Ok(())
    }

    fn kill_client(&self, id: Xid) -> Result<()> {
        self.conn.kill_client(id)?;
        Ok(())
    }
}

impl<C: Connection> XClientProperties for X11rbConnection<C> {
    fn get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
        let atom = self.atom_id(name)?;
        let r = self
            .conn
            .get_property(false, id, atom, AtomEnum::ANY, 0, 1024)?
            .reply()?;
        let prop_type = self.atom_name(r.type_)?;

        Ok(match prop_type.as_ref() {
            "ATOM" => Prop::Atom(
                r.value32()
                    .ok_or_else(|| Error::InvalidPropertyData(prop_type.to_string()))?
                    .map(|a| self.atom_name(a))
                    .collect::<Result<Vec<String>>>()?,
            ),

            // This uses unwrap() for symmetry with penrose::xcb (which does value()[0] to "panic")
            "CARDINAL" => Prop::Cardinal(r.value32().unwrap().next().unwrap()),

            "STRING" | "UTF8_STRING" => Prop::UTF8String(
                // FIXME: I think this should check prop.format == 8, but penrose::xcb does not
                String::from_utf8(r.value)
                    .map_err(Error::from)?
                    .trim_matches('\0')
                    .split('\0')
                    .map(|s| s.to_string())
                    .collect(),
            ),

            "WINDOW" => Prop::Window(
                r.value32()
                    .ok_or_else(|| Error::InvalidPropertyData(prop_type.to_string()))?
                    .collect(),
            ),

            "WM_HINTS" => Prop::WmHints(
                WmHints::try_from_bytes(
                    &r.value32()
                        .ok_or_else(|| Error::InvalidPropertyData(prop_type.to_string()))?
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| Error::InvalidPropertyData(e.to_string()))?,
            ),

            "WM_SIZE_HINTS" => Prop::WmNormalHints(
                WmNormalHints::try_from_bytes(
                    &r.value32()
                        .ok_or_else(|| Error::InvalidPropertyData(prop_type.to_string()))?
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| Error::InvalidPropertyData(e.to_string()))?,
            ),

            // Default to returning the raw bytes as u32s which the user can then
            // convert as needed if the prop type is not one we recognise
            _ => Prop::Bytes(match r.format {
                8 => r.value8().unwrap().map(From::from).collect(),
                16 => r.value16().unwrap().map(From::from).collect(),
                32 => r.value32().unwrap().collect(),
                _ => {
                    return Err(Error::InvalidPropertyData(format!(
                        "prop type for {} was {} which claims to have a data format of {}",
                        name, prop_type, r.type_
                    ))
                    .into())
                }
            }),
        })
    }

    fn list_props(&self, id: Xid) -> Result<Vec<String>> {
        self.conn
            .list_properties(id)?
            .reply()?
            .atoms
            .into_iter()
            .map(|a| self.atom_name(a))
            .collect()
    }

    fn delete_prop(&self, id: Xid, prop: &str) -> Result<()> {
        self.conn.delete_property(id, self.atom_id(prop)?)?;
        Ok(())
    }

    fn change_prop(&self, id: Xid, prop: &str, val: Prop) -> Result<()> {
        let a = self.atom_id(prop)?;
        let (ty, data) = match val {
            Prop::UTF8String(strs) => {
                self.conn.change_property8(
                    PropMode::REPLACE,
                    id,
                    a,
                    AtomEnum::STRING,
                    strs.join("\0").as_bytes(),
                )?;
                return Ok(());
            }

            Prop::Atom(atoms) => (
                AtomEnum::ATOM,
                atoms
                    .iter()
                    .map(|a| self.atom_id(a))
                    .collect::<Result<Vec<u32>>>()?,
            ),

            Prop::Bytes(_) => {
                return Err(Error::InvalidPropertyData(
                    "unable to change non standard props".into(),
                )
                .into())
            }

            Prop::Cardinal(val) => (AtomEnum::CARDINAL, vec![val]),

            Prop::Window(ids) => (AtomEnum::WINDOW, ids),

            // FIXME: handle changing WmHints and WmNormalHints correctly in change_prop
            Prop::WmHints(_) | Prop::WmNormalHints(_) => {
                return Err(Error::InvalidPropertyData(
                    "unable to change WmHints or WmNormalHints".into(),
                )
                .into())
            }
        };

        self.conn
            .change_property32(PropMode::REPLACE, id, a, ty, &data)?;
        Ok(())
    }

    fn set_client_state(&self, id: Xid, wm_state: WindowState) -> Result<()> {
        let mode = PropMode::REPLACE;
        let a = self.atom_id(Atom::WmState.as_ref())?;
        let state = match wm_state {
            WindowState::Withdrawn => 0,
            WindowState::Normal => 1,
            WindowState::Iconic => 3,
        };

        self.conn.change_property32(mode, id, a, a, &[state])?;
        Ok(())
    }
}

impl<C: Connection> XEventHandler for X11rbConnection<C> {
    fn flush(&self) -> bool {
        self.conn.flush().is_ok()
    }

    fn wait_for_event(&self) -> Result<XEvent> {
        loop {
            let event = self.conn.wait_for_event()?;
            if let Some(event) = super::event::convert_event(self, event)? {
                return Ok(event);
            }
        }
    }

    fn send_client_event(&self, msg: ClientMessage) -> Result<()> {
        let type_ = self.atom_id(&msg.dtype)?;
        let data = match msg.data() {
            xconnection::ClientMessageData::U8(u8s) => ClientMessageData::from(*u8s),
            xconnection::ClientMessageData::U16(u16s) => ClientMessageData::from(*u16s),
            xconnection::ClientMessageData::U32(u32s) => ClientMessageData::from(*u32s),
        };
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: msg.id,
            type_,
            data,
        };
        let mask = match msg.mask {
            ClientEventMask::NoEventMask => EventMask::NO_EVENT,
            ClientEventMask::SubstructureNotify => EventMask::SUBSTRUCTURE_NOTIFY,
        };

        self.conn.send_event(false, msg.id, mask, event)?;
        Ok(())
    }

    fn build_client_event(&self, kind: ClientMessageKind) -> Result<ClientMessage> {
        kind.as_message(self)
    }
}

impl<C: Connection> XState for X11rbConnection<C> {
    fn root(&self) -> Xid {
        self.root
    }

    fn current_screens(&self) -> Result<Vec<Screen>> {
        let resources = self.conn.randr_get_screen_resources(self.root)?.reply()?;

        // Send queries for all CRTCs
        let crtcs = resources
            .crtcs
            .iter()
            .map(|c| {
                self.conn
                    .randr_get_crtc_info(*c, 0)
                    .map_err(|err| err.into())
            })
            .collect::<Result<Vec<_>>>()?;

        // Get the replies and construct screens
        let screens = crtcs
            .into_iter()
            .flat_map(|cookie| cookie.reply().ok())
            .enumerate()
            .filter(|(_, reply)| reply.width > 0)
            .map(|(i, reply)| {
                let region = Region::new(
                    reply.x as u32,
                    reply.y as u32,
                    reply.width as u32,
                    reply.height as u32,
                );
                Screen::new(region, i)
            })
            .collect();
        Ok(screens)
    }

    fn cursor_position(&self) -> Result<Point> {
        let reply = self.conn.query_pointer(self.root)?.reply()?;
        Ok(Point::new(reply.root_x as u32, reply.root_y as u32))
    }

    fn warp_cursor(&self, win_id: Option<Xid>, screen: &Screen) -> Result<()> {
        let (x, y, id) = match win_id {
            Some(id) => {
                let (_, _, w, h) = self.client_geometry(id)?.values();
                (w / 2, h / 2, id)
            }
            None => {
                let (x, y, w, h) = screen.region(true).values();
                (x + w / 2, y + h / 2, self.root)
            }
        };

        self.conn
            .warp_pointer(x11rb::NONE, id, 0, 0, 0, 0, x as i16, y as i16)?;
        Ok(())
    }

    fn client_geometry(&self, id: Xid) -> Result<Region> {
        let res = self.conn.get_geometry(id)?.reply()?;
        Ok(Region::new(
            res.x as u32,
            res.y as u32,
            res.width as u32,
            res.height as u32,
        ))
    }

    fn active_clients(&self) -> Result<Vec<Xid>> {
        Ok(self.conn.query_tree(self.root)?.reply()?.children)
    }

    fn focused_client(&self) -> Result<Xid> {
        Ok(self.conn.get_input_focus()?.reply()?.focus)
    }
}

impl<C: Connection> XConn for X11rbConnection<C> {
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        todo!()
    }

    fn init(&self) -> Result<()> {
        self.set_client_attributes(self.root(), &[ClientAttr::RootEventMask])?;
        Ok(())
    }

    fn check_window(&self) -> Xid {
        self.check_win
    }

    fn cleanup(&self) -> Result<()> {
        self.conn.ungrab_keyboard(x11rb::CURRENT_TIME)?;
        self.conn.ungrab_key(Grab::ANY, self.root, ModMask::ANY)?;
        self.conn
            .ungrab_button(ButtonIndex::ANY, self.root, ModMask::ANY)?;
        self.conn.destroy_window(self.check_win)?;
        let net_name = Atom::NetActiveWindow.as_ref();
        self.conn
            .delete_property(self.root, self.atom_id(net_name)?)?;
        self.conn.flush()?;

        Ok(())
    }

    fn grab_keys(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        self.grab_key_bindings(key_codes)?;
        self.grab_mouse_buttons(mouse_states)?;
        self.flush();

        Ok(())
    }
}

impl<C: Connection> X11rbConnection<C> {
    fn grab_key_bindings(&self, keys: &[KeyCode]) -> Result<()> {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, u16::from(ModMask::M2)];
        let mode = GrabMode::ASYNC;

        for m in modifiers.iter() {
            for k in keys.iter() {
                self.conn.grab_key(
                    false,      // don't pass grabbed events through to the client
                    self.root,  // the window to grab: in this case the root window
                    k.mask | m, // modifiers to grab
                    k.code,     // keycode to grab
                    mode,       // don't lock pointer input while grabbing
                    mode,       // don't lock keyboard input while grabbing
                )?;
            }
        }

        self.flush();
        Ok(())
    }

    fn grab_mouse_buttons(&self, states: &[MouseState]) -> Result<()> {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, u16::from(ModMask::M2)];
        let mode = GrabMode::ASYNC;
        let mask = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::BUTTON_MOTION;
        let mask = u16::try_from(u32::from(mask)).unwrap();

        for m in modifiers.iter() {
            for state in states.iter() {
                let button = state.button().into();
                self.conn.grab_button(
                    false,            // don't pass grabbed events through to the client
                    self.root,        // the window to grab: in this case the root window
                    mask,             // which events are reported to the client
                    mode,             // don't lock pointer input while grabbing
                    mode,             // don't lock keyboard input while grabbing
                    x11rb::NONE,      // don't confine the cursor to a specific window
                    x11rb::NONE,      // don't change the cursor type
                    button,           // the button to grab
                    state.mask() | m, // modifiers to grab
                )?;
            }
        }

        self.flush();
        Ok(())
    }
}
