//! API wrapper for talking to the X server using x11rb

use crate::{
    PenroseError,
    core::{
        bindings::{
            KeyBindings, KeyCode, ModifierKey, MouseBindings, MouseButton, MouseEvent,
            MouseEventKind, MouseState,
        },
        data_types::{Point, Region, WinId},
        screen::Screen,
        xconnection::{
            Atom, XConn, XEvent, AUTO_FLOAT_WINDOW_TYPES, EWMH_SUPPORTED_ATOMS,
            UNMANAGED_WINDOW_TYPES,
        },
    },
    x11rb::{Result as X11Result, X11rbError},
    Result,
};

use x11rb::{
    connection::Connection,
    properties::WmClass,
    protocol::{
        randr::{self, ConnectionExt as _},
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux,
            ConnectionExt as _, CreateWindowAux, EventMask, GrabMode, InputFocus, ModMask,
            PropMode, StackMode, Window, WindowClass, CLIENT_MESSAGE_EVENT,
        },
        Event,
    },
    wrapper::ConnectionExt as _,
};

use strum::IntoEnumIterator;

use std::{
    collections::HashMap,
    str::FromStr,
};

/// Handles communication with an X server via the x11rb crate.
#[derive(Debug)]
pub struct X11rbConnection<C> {
    conn: C,
    root: Window,
    check_win: Window,
    atoms: HashMap<Atom, u32>,
    auto_float_types: Vec<u32>,
    dont_manage_types: Vec<u32>,
}

impl<C> X11rbConnection<C>
where
    C: Connection,
{
    pub(crate) fn new_for_connection(conn: C) -> Result<Self> {
        let root = conn.setup().roots[0].root;
        conn.prefetch_extension_information(randr::X11_EXTENSION_NAME)
            .map_err(|err| X11rbError::from(err))?;

        // Setup atoms: First send all InternAtom requests and then fetch the replies
        let atoms = Atom::iter()
            .map(|atom| Ok((atom, conn.intern_atom(false, atom.as_ref().as_bytes())?)))
            .collect::<X11Result<Vec<_>>>()?;
        let atoms = atoms.into_iter()
            .map(|(atom, cookie)| Ok((atom, cookie.reply()?.atom)))
            .collect::<X11Result<HashMap<_, _>>>()?;

        let auto_float_types = AUTO_FLOAT_WINDOW_TYPES
            .iter()
            .map(|a| *atoms.get(&a).unwrap())
            .collect();
        let dont_manage_types = UNMANAGED_WINDOW_TYPES
            .iter()
            .map(|a| *atoms.get(&a).unwrap())
            .collect();

        // Setup the RandR extension
        if conn.extension_information(randr::X11_EXTENSION_NAME)
                .map_err(|err| X11rbError::from(err))?
                .is_none() {
            return Err(X11rbError::MissingRandRSupport.into());
        }
        use randr::NotifyMask;
        let mask = NotifyMask::OUTPUT_CHANGE | NotifyMask::CRTC_CHANGE | NotifyMask::SCREEN_CHANGE;
        conn.randr_select_input(root, mask).map_err(|err| X11rbError::from(err))?;

        // Setup the check win
        let check_win = conn.generate_id().map_err(|err| X11rbError::from(err))?;
        let aux = CreateWindowAux::new().override_redirect(1);
        let (x, y, w, h, border, visual) = (0, 0, 1, 1, 0, 0);
        conn.create_window(0, check_win, root, x, y, w, h, border, WindowClass::INPUT_OUTPUT, visual, &aux)
            .map_err(|err| X11rbError::from(err))?;

        // TODO: Check the version of the RandR extension supported by the server.
        // It might be too old.
        Ok(X11rbConnection {
            conn,
            root,
            check_win,
            atoms,
            auto_float_types,
            dont_manage_types,
        })
    }

    fn known_atom(&self, atom: Atom) -> u32 {
        *self.atoms.get(&atom).unwrap()
    }

    fn window_has_type_in(&self, id: WinId, win_types: &[u32]) -> bool {
        self.conn.get_property(false, id, self.known_atom(Atom::NetWmWindowType), AtomEnum::ANY, 0, 1024)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .as_ref()
            .and_then(|reply| reply.value32())
            .and_then(|mut iter| iter.next())
            .map(|atom| win_types.contains(&atom))
            .unwrap_or(false)
    }

    fn get_atom_name(&self, atom: u32) -> Option<String> {
        // FIXME: We could scan our known atoms first. Iterating through the values of
        // a HashMap should still be faster than querying the X11 server.
        self.conn.get_atom_name(atom)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| String::from_utf8(reply.name).ok())
    }
}

impl<C> XConn for X11rbConnection<C>
where
    C: Connection,
{
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        todo!()
    }

    fn flush(&self) -> bool {
        self.conn.flush().is_ok()
    }

    fn wait_for_event(&self) -> Result<XEvent> {
        let numlock = ModMask::M2;
        loop {
            match self.conn.wait_for_event().map_err(|err| X11rbError::from(err))? {
                Event::ButtonPress(ev) => {
                    match to_mouse_state(ev.detail, ev.state) {
                        Some(state) => return Ok(XEvent::MouseEvent(MouseEvent::new(ev.event, ev.root_x, ev.root_y, ev.event_x, ev.event_y, state, MouseEventKind::Press))),
                        None => warn!("Dropping unknown mouse button event"),
                    }
                }
                Event::ButtonRelease(ev) => {
                    match to_mouse_state(ev.detail, ev.state) {
                        Some(state) => return Ok(XEvent::MouseEvent(MouseEvent::new(ev.event, ev.root_x, ev.root_y, ev.event_x, ev.event_y, state, MouseEventKind::Release))),
                        None => warn!("Dropping unknown mouse button event"),
                    }
                }
                Event::MotionNotify(ev) => {
                    let detail = 5; // FIXME see https://github.com/sminez/penrose/issues/113
                    match to_mouse_state(detail, ev.state) {
                        Some(state) => return Ok(XEvent::MouseEvent(MouseEvent::new(ev.event, ev.root_x, ev.root_y, ev.event_x, ev.event_y, state, MouseEventKind::Motion))),
                        None => warn!("Dropping unknown mouse button event"),
                    }
                }
                Event::KeyPress(ev) => {
                    let code = KeyCode {
                        mask: ev.state,
                        code: ev.detail,
                    };
                    return Ok(XEvent::KeyPress(code.ignoring_modifier(numlock.into())));
                }
                Event::MapRequest(ev) => {
                    let id = ev.window;
                    if let Some(attr) = self.conn.get_window_attributes(id)
                            .ok()
                            .and_then(|cookie| cookie.reply().ok()) {
                        return Ok(XEvent::MapRequest {
                            id,
                            ignore: attr.override_redirect,
                        });
                    }
                }
                Event::EnterNotify(ev) => {
                    return Ok(XEvent::Enter {
                        id: ev.event,
                        rpt: Point::new(ev.root_x as u32, ev.root_y as u32),
                        wpt: Point::new(ev.event_x as u32, ev.event_y as u32),
                    });
                }
                Event::LeaveNotify(ev) => {
                    return Ok(XEvent::Leave {
                        id: ev.event,
                        rpt: Point::new(ev.root_x as u32, ev.root_y as u32),
                        wpt: Point::new(ev.event_x as u32, ev.event_y as u32),
                    });
                }
                Event::DestroyNotify(ev) => return Ok(XEvent::Destroy { id: ev.window }),
                Event::RandrScreenChangeNotify(_) => return Ok(XEvent::ScreenChange),
                Event::ConfigureNotify(ev) => return Ok(XEvent::ConfigureNotify {
                    id: ev.window,
                    r: Region::new(
                        ev.x as u32,
                        ev.y as u32,
                        ev.width as u32,
                        ev.height as u32,
                    ),
                    is_root: ev.window == self.root,
                }),
                Event::ClientMessage(ev) => {
                    if let Some(name) = self.get_atom_name(ev.type_) {
                        let data = match ev.format {
                            8 => ev.data.as_data8().iter().map(|&d| d as usize).collect(),
                            16 => ev.data.as_data16().iter().map(|&d| d as usize).collect(),
                            32 => ev.data.as_data32().iter().map(|&d| d as usize).collect(),
                            _ => unreachable!("ClientMessageEvent.format should really be an enum..."),
                        };
                        return Ok(XEvent::ClientMessage {
                            id: ev.window,
                            dtype: name.to_string(),
                            data,
                        });
                    }
                }
                Event::PropertyNotify(ev) => {
                    let is_root = ev.window == self.root;
                    if !is_root || ev.atom == self.known_atom(Atom::WmName) || ev.atom == self.known_atom(Atom::NetWmName) {
                        if let Some(name) = self.get_atom_name(ev.atom) {
                            return Ok(XEvent::PropertyNotify {
                                id: ev.window,
                                atom: name,
                                is_root,
                            });
                        }
                    }
                }
                // NOTE: Ignoring other event types
                _ => {}
            }
        }
    }

    fn current_outputs(&self) -> Vec<Screen> {
        fn current_outputs_impl(conn: &impl Connection, win: WinId) -> X11Result<Vec<Screen>> {
            let resources = conn.randr_get_screen_resources(win)?.reply()?;
            // Send queries for all CRTCs
            let crtcs = resources.crtcs.iter()
                .map(|c| conn.randr_get_crtc_info(*c, 0).map_err(|err| err.into()))
                .collect::<X11Result<Vec<_>>>()?;
            // Get the replies and construct screens
            let screens = crtcs.into_iter()
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
        current_outputs_impl(&self.conn, self.root).unwrap()
    }

    fn cursor_position(&self) -> Point {
        self.conn.query_pointer(self.root)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|reply| Point::new(reply.root_x as u32, reply.root_y as u32))
            .unwrap_or_else(|| Point::new(0, 0))
    }

    fn position_window(&self, id: WinId, r: Region, border: u32, stack_above: bool) {
        let (x, y, w, h) = r.values();
        let mut aux = ConfigureWindowAux::new()
            .border_width(border)
            .x(x as i32)
            .y(y as i32)
            .width(w)
            .height(h);
        if stack_above {
            aux = aux.stack_mode(StackMode::ABOVE);
        }
        self.conn.configure_window(id, &aux).unwrap();
    }

    fn raise_window(&self, id: WinId) {
        let aux = ConfigureWindowAux::new()
            .stack_mode(StackMode::ABOVE);
        self.conn.configure_window(id, &aux).unwrap();
    }

    fn mark_new_window(&self, id: WinId) {
        let mask = EventMask::ENTER_WINDOW
            | EventMask::LEAVE_WINDOW
            | EventMask::PROPERTY_CHANGE
            | EventMask::STRUCTURE_NOTIFY;
        let aux = ChangeWindowAttributesAux::new().event_mask(mask);
        self.conn.change_window_attributes(id, &aux).unwrap();
    }

    fn map_window(&self, id: WinId) {
        self.conn.map_window(id).unwrap();
    }

    fn unmap_window(&self, id: WinId) {
        self.conn.unmap_window(id).unwrap();
    }

    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()> {
        let atom = self.intern_atom(atom_name)?;
        let event = ClientMessageEvent {
            response_type: CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: id,
            type_: self.known_atom(Atom::WmProtocols),
            data: [atom, x11rb::CURRENT_TIME, 0, 0, 0].into(),
        };
        self.conn.send_event(false, id, EventMask::NO_EVENT, &event)
            .map_err(|e| X11rbError::from(e).into())
            .map(std::mem::drop)
    }

    fn focused_client(&self) -> WinId {
        self.conn.get_input_focus()
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|reply| reply.focus)
            .unwrap_or(0)
    }

    fn focus_client(&self, id: WinId) {
        // FIXME: This ignores WM_PROTOCOLS and WM_HINTS (same for crate::xcb::api::mark_focused_window())
        self.conn.set_input_focus(InputFocus::PARENT, id, x11rb::CURRENT_TIME).unwrap();

        let atom = self.known_atom(Atom::NetActiveWindow);
        self.conn.change_property32(PropMode::REPLACE, id, atom, AtomEnum::WINDOW, &[id]).unwrap();
    }

    fn set_client_border_color(&self, id: WinId, color: u32) {
        let aux = ChangeWindowAttributesAux::new().border_pixel(color);
        self.conn.change_window_attributes(id, &aux).unwrap();
    }

    fn grab_keys(&self, key_bindings: &KeyBindings<Self>, mouse_bindings: &MouseBindings<Self>) {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = [0, ModMask::M2.into()];

        // grab keys
        for key in key_bindings.keys() {
            for m in modifiers.iter() {
                self.conn.grab_key(
                    false,           // don't pass grabbed events through to the client
                    self.root,
                    key.mask | m,    // modifiers to grab
                    key.code,        // keycode to grab
                    GrabMode::ASYNC, // don't lock the pointer input while grabbing
                    GrabMode::ASYNC, // don't lock the keyboard input while grabbing
                ).unwrap();
            }
        }

        // grab mouse buttons
        let mask = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::BUTTON_MOTION;
        let mask = std::convert::TryInto::<u16>::try_into(u32::from(mask)).unwrap();
        for (_, state) in mouse_bindings.keys() {
            for m in modifiers.iter() {
                self.conn.grab_button(
                    false,            // don't pass grabbed events through to the client
                    self.root,
                    mask,             // which events are reported to the client
                    GrabMode::ASYNC,  // don't lock the pointer input while grabbing
                    GrabMode::ASYNC,  // don't lock the keyboard input while grabbing
                    x11rb::NONE,      // don't confine the cursor to a specific window
                    x11rb::NONE,      // don't change the cursor type
                    state.button().into(), // the button to grab
                    state.mask() | m, // modifiers to grab
                ).unwrap();
            }
        }

        let mask = EventMask::PROPERTY_CHANGE
            | EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::BUTTON_MOTION;
        let aux = ChangeWindowAttributesAux::new().event_mask(mask);
        self.conn.change_window_attributes(self.root, &aux).unwrap();

        self.conn.flush().unwrap();
    }

    fn set_wm_properties(&self, workspaces: &[&str]) {
        let wm_name = "penrose";
        let check = self.known_atom(Atom::NetSupportingWmCheck);
        for &win in [self.check_win, self.root].iter() {
            self.conn.change_property32(PropMode::REPLACE, win, check, AtomEnum::WINDOW, &[self.check_win]).unwrap();
            self.conn.change_property8(PropMode::REPLACE, win, AtomEnum::WM_NAME, AtomEnum::STRING, wm_name.as_bytes()).unwrap();
        }

        // EWMH support
        let supported = EWMH_SUPPORTED_ATOMS
            .iter()
            .map(|a| self.known_atom(*a))
            .collect::<Vec<u32>>();
        let net_supported = self.known_atom(Atom::NetSupported);
        self.conn.change_property32(PropMode::REPLACE, self.root, net_supported, AtomEnum::ATOM, &supported).unwrap();
        self.update_desktops(workspaces);
        self.conn.delete_property(self.root, self.known_atom(Atom::NetClientList)).unwrap();
    }

    fn update_desktops(&self, workspaces: &[&str]) {
        let num_desktops = self.known_atom(Atom::NetNumberOfDesktops);
        let desktop_names = self.known_atom(Atom::NetDesktopNames);
        let workspace_names = workspaces.join("\0");
        self.conn.change_property32(PropMode::REPLACE, self.root, num_desktops, AtomEnum::CARDINAL, &[workspaces.len() as u32]).unwrap();
        self.conn.change_property8(PropMode::REPLACE, self.root, desktop_names, AtomEnum::STRING, workspace_names.as_bytes()).unwrap();
    }

    fn update_known_clients(&self, clients: &[WinId]) {
        let list = self.known_atom(Atom::NetClientList);
        let list_stacking = self.known_atom(Atom::NetClientListStacking);
        self.conn.change_property32(PropMode::REPLACE, self.root, list, AtomEnum::WINDOW, clients).unwrap();
        self.conn.change_property32(PropMode::REPLACE, self.root, list_stacking, AtomEnum::WINDOW, clients).unwrap();
    }

    fn set_current_workspace(&self, wix: usize) {
        let desktop = self.known_atom(Atom::NetCurrentDesktop);
        self.conn.change_property32(PropMode::REPLACE, self.root, desktop, AtomEnum::CARDINAL, &[wix as u32]).unwrap();
    }

    fn set_root_window_name(&self, name: &str) {
        self.conn.change_property8(PropMode::REPLACE, self.root, AtomEnum::WM_NAME, AtomEnum::STRING, name.as_bytes()).unwrap();
    }

    fn set_client_workspace(&self, id: WinId, wix: usize) {
        let desktop = self.known_atom(Atom::NetWmDesktop);
        self.conn.change_property32(PropMode::REPLACE, id, desktop, AtomEnum::CARDINAL, &[wix as u32]).unwrap();
    }

    fn toggle_client_fullscreen(&self, id: WinId, client_is_fullscreen: bool) {
        let fullscreen = [self.known_atom(Atom::NetWmStateFullscreen)];
        let data: &[u32] = if client_is_fullscreen {
            &fullscreen
        } else {
            &[]
        };
        let wm_state = self.known_atom(Atom::NetWmState);
        self.conn.change_property32(PropMode::REPLACE, id, wm_state, AtomEnum::ATOM, &data).unwrap();
    }

    fn window_should_float(&self, id: WinId, floating_classes: &[&str]) -> bool {
        if WmClass::get(&self.conn, id)
                .ok()
                .and_then(|cookie| cookie.reply_unchecked().ok().flatten())
                .as_ref()
                .and_then(|class| std::str::from_utf8(class.class()).ok())
                .map(|class| floating_classes.contains(&class))
                .unwrap_or(false) {
            return true;
        }
        self.window_has_type_in(id, &self.auto_float_types)
    }

    fn is_managed_window(&self, id: WinId) -> bool {
        !self.window_has_type_in(id, &self.dont_manage_types)
    }

    fn window_geometry(&self, id: WinId) -> Result<Region> {
        fn window_geometry_impl(conn: &impl Connection, id: WinId) -> X11Result<Region> {
            let geo = conn.get_geometry(id)?.reply()?;
            Ok(Region::new(geo.x as _, geo.y as _, geo.width as _, geo.height as _))
        }
        Ok(window_geometry_impl(&self.conn, id)?)
    }

    fn warp_cursor(&self, id: Option<WinId>, screen: &Screen) {
        let (x, y, id) = match id {
            Some(id) => {
                let (_, _, w, h) = match self.window_geometry(id) {
                    Ok(region) => region.values(),
                    Err(e) => {
                        error!("error fetching window details while warping cursor: {}", e);
                        return;
                    }
                };
                ((w / 2), (h / 2), id)
            }
            None => {
                let (x, y, w, h) = screen.region(true).values();
                ((x + w / 2), (y + h / 2), self.root)
            }
        };
        self.conn.warp_pointer(x11rb::NONE, id, 0, 0, 0, 0, x as _, y as _).unwrap();
    }

    fn query_for_active_windows(&self) -> Vec<WinId> {
        match self.conn.query_tree(self.root)
                .ok()
                .and_then(|cookie| cookie.reply().ok()) {
            Some(reply) => {
                let mut children = reply.children;
                children.retain(|&id| !self.window_has_type_in(id, &self.dont_manage_types));
                children
            }
            None => Vec::new(),
        }
    }

    fn str_prop(&self, id: u32, name: &str) -> Result<String> {
        fn str_prop_impl(conn: &impl Connection, id: u32, atom: u32) -> X11Result<Vec<u8>> {
            Ok(conn.get_property(false, id, atom, AtomEnum::ANY, 0, 1024)?
                .reply()?
                .value)
        }
        let atom = self.intern_atom(name)?;
        let value = str_prop_impl(&self.conn, id, atom)?;
        Ok(String::from_utf8(value)?)
    }

    fn atom_prop(&self, id: u32, name: &str) -> Result<u32> {
        fn atom_prop_impl(conn: &impl Connection, id: u32, atom: u32) -> X11Result<Option<u32>> {
            Ok(conn.get_property(false, id, atom, AtomEnum::ANY, 0, 1024)?
                .reply()?
                .value32()
                .and_then(|mut iter| iter.next()))
        }
        // FIXME: If this API only supports atoms from Atom, then why does it get a &str?
        let atom = Atom::from_str(name)?;
        let value = atom_prop_impl(&self.conn, id, self.known_atom(atom))?;
        value.ok_or(PenroseError::X11rb(X11rbError::MissingProp(atom, id)))
    }

    fn intern_atom(&self, atom: &str) -> Result<u32> {
        if let Ok(known) = Atom::from_str(atom) {
            Ok(self.known_atom(known))
        } else {
            fn intern_atom_impl(conn: &impl Connection, atom: &str) -> X11Result<u32> {
                Ok(conn.intern_atom(false, atom.as_bytes())?
                    .reply()?
                    .atom)
            }
            Ok(intern_atom_impl(&self.conn, atom)?)
        }
    }

    fn cleanup(&self) {
        // FIXME I *think* that penrose::xcb does not actually do anything here. It tries to send a
        // couple of requests, but there is no flush() afterwards. There is a very high chance that
        // the requests are not actually sent. Thus, this function is left empty.
    }
}

fn to_mouse_state(detail: u8, state: u16) -> Option<MouseState> {
    fn is_held(key: &ModifierKey, mask: u16) -> bool {
        mask & u16::from(*key) > 0
    }

    let button = match detail {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        4 => Some(MouseButton::ScrollUp),
        5 => Some(MouseButton::ScrollDown),
        _ => None,
    }?;
    let modifiers = ModifierKey::iter().filter(|m| is_held(m, state)).collect();
    Some(MouseState { button, modifiers })
}
