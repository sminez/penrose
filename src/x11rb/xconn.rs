//! TODO: Docs

use crate::{
    PenroseError,
    core::{
        bindings::{KeyBindings, MouseBindings},
        data_types::{Point, PropVal, Region, WinAttr, WinConfig, WinId, WinType},
        manager::WindowManager,
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
    protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as _, EventMask,
        InputFocus, PropMode, StackMode, Window,
    },
    wrapper::ConnectionExt as _,
};

use strum::IntoEnumIterator;

use std::{
    collections::HashMap,
    str::FromStr,
};

#[derive(Debug)]
pub struct X11rbConnection<C> {
    conn: C,
    root: Window,
    atoms: HashMap<Atom, u32>,
    auto_float_types: Vec<u32>,
    dont_manage_types: Vec<u32>,
}

impl<C> X11rbConnection<C>
where
    C: Connection,
{
    pub fn new_for_connection(conn: C) -> Result<Self> {
        let root = conn.setup().roots[0].root;
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
        Ok(X11rbConnection {
            conn,
            root,
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
        todo!()
    }

    fn current_outputs(&self) -> Vec<Screen> {
        todo!()
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
        todo!()
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
        todo!()
    }

    fn set_wm_properties(&self, workspaces: &[&str]) {
        todo!()
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
        self.conn.change_property32(PropMode::REPLACE, self.root, desktop, AtomEnum::CARDINAL, &[wix as u32]).unwrap();
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
        todo!()
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
        todo!()
    }
}
