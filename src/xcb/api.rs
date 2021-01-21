//! A wrapper around the underlying xcb api layer that only exposes Penrose types
use crate::{
    core::{
        bindings::{KeyCode, KeyCodeMask, KeyCodeValue, MouseEvent, MouseState},
        data_types::{Point, PropVal, Region, WinAttr, WinConfig, WinId, WinType},
        helpers::spawn_for_output,
        screen::Screen,
        xconnection::{Atom, XEvent},
    },
    xcb::{Result, XcbApi, XcbError, XcbGenericEvent},
};
use strum::*;

use std::{collections::HashMap, convert::TryFrom, fmt, str::FromStr};

#[cfg(feature = "keysyms")]
use crate::{core::bindings::KeyPress, draw::KeyPressParseAttempt, penrose_keysyms::XKeySym};

/// A reverse lookup of KeyCode mask and value to key as a String using XKeySym mappings
pub type ReverseCodeMap = HashMap<(KeyCodeMask, KeyCodeValue), String>;

#[cfg(feature = "serde")]
fn default_conn() -> xcb::Connection {
    let (conn, _) = xcb::Connection::connect(None).expect("unable to connect using XCB");
    conn
}

/**
 * Use `xmodmap -pke` to determine the user's current keymap to allow for mapping X KeySym values
 * to their string representation on the user's system.
 *
 * # Panics
 * This function will panic if it is unable to fetch keycodes using the xmodmap
 * binary on your system or if the output of `xmodmap -pke` is not valid
 */
pub fn code_map_from_xmodmap() -> Result<ReverseCodeMap> {
    let output = match spawn_for_output("xmodmap -pke") {
        Ok(s) => s,
        Err(e) => return Err(XcbError::Raw(e.to_string())), // failed to spawn
    };
    Ok(output
        .lines()
        .flat_map(|l| {
            let mut words = l.split_whitespace(); // keycode <code> = <names ...>
            let key_code: u8 = match words.nth(1) {
                Some(word) => match word.parse() {
                    Ok(val) => val,
                    Err(e) => panic!("{}", e),
                },
                None => panic!("unexpected output format from xmodmap -pke"),
            };
            vec![
                words.nth(1).map(move |name| ((0, key_code), name.into())),
                words.next().map(move |name| ((1, key_code), name.into())),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<HashMap<(u16, u8), String>>())
}

/// A connection to the X server using the XCB C API
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Api {
    #[cfg_attr(feature = "serde", serde(skip, default = "default_conn"))]
    conn: xcb::Connection,
    root: WinId,
    check_win: WinId,
    randr_base: u8,
    atoms: HashMap<Atom, u32>,
    #[cfg(feature = "keysyms")]
    code_map: ReverseCodeMap,
}

impl fmt::Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XCB Api")
            .field("root", &self.root)
            .field("randr_base", &self.randr_base)
            .field("atoms", &self.atoms)
            .finish()
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        self.destroy_window(self.check_win)
    }
}

impl Api {
    /// Connect to the X server using the [XCB API][1]
    ///
    /// Each [Api] contains and embedded [xcb Connection][2] which is used for making
    /// all api calls through to the X server. Some state is cached in the Api itself
    /// in order to prevent redundant calls through to the X server.
    ///
    /// Creating a new [Api] instance will establish the underlying connection and if
    /// the `keysyms` feature is enabled, pull [KeyCode] mappings from the user
    /// system using `xmodmap`.
    ///
    /// [1]: http://rtbo.github.io/rust-xcb
    /// [2]: http://rtbo.github.io/rust-xcb/xcb/base/struct.Connection.html
    pub fn new() -> Result<Self> {
        let (conn, _) = xcb::Connection::connect(None)?;
        let mut api = Self {
            conn,
            root: 0,
            check_win: 0,
            randr_base: 0,
            atoms: HashMap::new(),
            #[cfg(feature = "keysyms")]
            code_map: code_map_from_xmodmap()?,
        };
        api.init()?;

        Ok(api)
    }

    fn init(&mut self) -> Result<()> {
        self.root = match self.conn.get_setup().roots().next() {
            Some(r) => r.root(),
            None => return Err(XcbError::NoScreens),
        };
        self.randr_base = self
            .conn
            .get_extension_data(&mut xcb::randr::id())
            .ok_or_else(|| XcbError::Randr("unable to fetch extension data".into()))?
            .first_event();

        self.check_win = self.conn.generate_id();
        xcb::create_window(
            &self.conn,
            0,
            self.check_win,
            self.root,
            0,
            0,
            1,
            1,
            0,
            0,
            0,
            &[],
        );
        self.conn.flush();

        self.atoms = Atom::iter()
            .map(|atom| {
                let val = self.atom(atom.as_ref())?;
                Ok((atom, val))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(())
    }

    pub(crate) fn known_atoms(&self) -> &HashMap<Atom, u32> {
        &self.atoms
    }

    pub(crate) fn conn(&self) -> &xcb::Connection {
        &self.conn
    }

    pub(crate) fn screen(&self, ix: usize) -> Result<xcb::Screen<'_>> {
        let mut roots: Vec<_> = self.conn.get_setup().roots().collect();
        let len = roots.len();
        if ix >= len {
            Err(XcbError::UnknownScreen(ix, len - 1))
        } else {
            Ok(roots.remove(ix))
        }
    }

    pub(crate) fn get_depth<'a>(&self, screen: &'a xcb::Screen<'_>) -> Result<xcb::Depth<'a>> {
        screen
            .allowed_depths()
            .max_by(|x, y| x.depth().cmp(&y.depth()))
            .ok_or(XcbError::QueryFailed("screen depth"))
    }

    pub(crate) fn get_visual_type<'a>(&self, depth: &xcb::Depth<'a>) -> Result<xcb::Visualtype> {
        depth
            .visuals()
            .find(|v| v.class() == xcb::VISUAL_CLASS_TRUE_COLOR as u8)
            .ok_or(XcbError::QueryFailed("visual type"))
    }

    /// Poll for the next event from the underlying [XCB Connection][::xcb::Connection],
    /// returning it as an [XKeySym] if it was a user keypress, or an [XEvent] if not.
    ///
    /// If no event is currently available, None is returned.
    #[cfg(feature = "keysyms")]
    pub fn next_keypress(&self) -> Result<Option<KeyPressParseAttempt>> {
        if let Some(event) = self.conn.poll_for_event() {
            let attempt = self.attempt_to_parse_as_keypress(event);
            if let Ok(Some(_)) = attempt {
                return attempt;
            }
        }

        Ok(self.conn.has_error().map(|_| None)?)
    }

    /// Poll for the next event from the underlying [XCB Connection][::xcb::Connection],
    /// returning it as an [XKeySym] if it was a user keypress, or an [XEvent] if not.
    #[cfg(feature = "keysyms")]
    pub fn next_keypress_blocking(&self) -> Result<KeyPressParseAttempt> {
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                let attempt = self.attempt_to_parse_as_keypress(event);
                if let Ok(Some(k)) = attempt {
                    return Ok(k);
                }
            }

            if let Err(e) = self.conn.has_error() {
                return Err(e.into());
            }
        }
    }

    #[cfg(feature = "keysyms")]
    fn attempt_to_parse_as_keypress(
        &self,
        event: XcbGenericEvent,
    ) -> Result<Option<KeyPressParseAttempt>> {
        if let Ok(k) = KeyCode::try_from(&event) {
            if let Some(s) = self.code_map.get(&(k.mask, k.code)) {
                if let Ok(k) = KeyPress::try_from(XKeySym::from_str(s)?) {
                    return Ok(Some(KeyPressParseAttempt::KeyPress(k)));
                }
            }
        }

        if let Some(e) = self.generic_xcb_to_xevent(event)? {
            return Ok(Some(KeyPressParseAttempt::XEvent(e)));
        }

        Ok(None)
    }

    fn generic_xcb_to_xevent(&self, event: XcbGenericEvent) -> Result<Option<XEvent>> {
        let xcb_response_type_mask: u8 = 0x7F;
        let numlock = xcb::MOD_MASK_2 as u16;

        let etype = event.response_type() & xcb_response_type_mask;
        // Need to apply the randr_base mask as well which doesn't seem to work in 'match'
        if etype == self.randr_base + xcb::randr::NOTIFY {
            return Ok(Some(XEvent::RandrNotify));
        }

        Ok(match etype {
            xcb::BUTTON_PRESS | xcb::BUTTON_RELEASE | xcb::MOTION_NOTIFY => {
                match MouseEvent::try_from(event) {
                    Ok(m) => Some(XEvent::MouseEvent(m)),
                    Err(_) => {
                        warn!("dropping unknown mouse button event");
                        None // Drop unknown buttons
                    }
                }
            }

            xcb::KEY_PRESS => Some(XEvent::KeyPress(
                KeyCode::try_from(event)?.ignoring_modifier(numlock),
            )),

            xcb::MAP_REQUEST => {
                let e: &xcb::MapRequestEvent = unsafe { xcb::cast_event(&event) };
                let id = e.window();
                xcb::xproto::get_window_attributes(&self.conn, id)
                    .get_reply()
                    .ok()
                    .map(|r| XEvent::MapRequest {
                        id,
                        ignore: r.override_redirect(),
                    })
            }

            xcb::ENTER_NOTIFY => {
                let e: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Enter {
                    id: e.event(),
                    rpt: Point::new(e.root_x() as u32, e.root_y() as u32),
                    wpt: Point::new(e.event_x() as u32, e.event_y() as u32),
                })
            }

            xcb::LEAVE_NOTIFY => {
                let e: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Leave {
                    id: e.event(),
                    rpt: Point::new(e.root_x() as u32, e.root_y() as u32),
                    wpt: Point::new(e.event_x() as u32, e.event_y() as u32),
                })
            }

            xcb::DESTROY_NOTIFY => {
                let e: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Destroy { id: e.window() })
            }

            xcb::randr::SCREEN_CHANGE_NOTIFY => Some(XEvent::ScreenChange),

            xcb::CONFIGURE_NOTIFY => {
                let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::ConfigureNotify {
                    id: e.window(),
                    r: Region::new(
                        e.x() as u32,
                        e.y() as u32,
                        e.width() as u32,
                        e.height() as u32,
                    ),
                    is_root: e.window() == self.root,
                })
            }

            xcb::CLIENT_MESSAGE => {
                let e: &xcb::ClientMessageEvent = unsafe { xcb::cast_event(&event) };
                xcb::xproto::get_atom_name(&self.conn, e.type_())
                    .get_reply()
                    .ok()
                    .map(|a| XEvent::ClientMessage {
                        id: e.window(),
                        dtype: a.name().to_string(),
                        data: match e.format() {
                            8 => e.data().data8().iter().map(|&d| d as usize).collect(),
                            16 => e.data().data16().iter().map(|&d| d as usize).collect(),
                            32 => e.data().data32().iter().map(|&d| d as usize).collect(),
                            _ => unreachable!(
                                "ClientMessageEvent.format should really be an enum..."
                            ),
                        },
                    })
            }

            xcb::PROPERTY_NOTIFY => {
                let e: &xcb::PropertyNotifyEvent = unsafe { xcb::cast_event(&event) };
                xcb::xproto::get_atom_name(&self.conn, e.atom())
                    .get_reply()
                    .ok()
                    .and_then(|a| {
                        let atom = a.name().to_string();
                        let is_root = e.window() == self.root;
                        if is_root && !(atom == "WM_NAME" || atom == "_NET_WM_NAME") {
                            None
                        } else {
                            Some(XEvent::PropertyNotify {
                                id: e.window(),
                                atom,
                                is_root,
                            })
                        }
                    })
            }

            // NOTE: ignoring other event types
            _ => None,
        })
    }
}

impl XcbApi for Api {
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        self.init()
    }

    // xcb docs: https://www.mankier.com/3/xcb_intern_atom
    fn atom(&self, name: &str) -> Result<u32> {
        if let Ok(known) = Atom::from_str(name) {
            // This could be us initialising in which case self.atoms is empty
            if let Some(atom) = self.atoms.get(&known) {
                return Ok(*atom);
            }
        }

        Ok(xcb::intern_atom(&self.conn, false, name)
            .get_reply()?
            .atom())
    }

    fn known_atom(&self, atom: Atom) -> u32 {
        *self.atoms.get(&atom).unwrap()
    }

    fn delete_prop(&self, id: WinId, prop: Atom) {
        xcb::delete_property(&self.conn, id, self.known_atom(prop));
    }

    // xcb docs: https://www.mankier.com/3/xcb_get_property
    fn get_atom_prop(&self, id: WinId, atom: Atom) -> Result<u32> {
        let a = self.known_atom(atom);
        let cookie = xcb::get_property(&self.conn, false, id, a, xcb::ATOM_ANY, 0, 1024);
        let reply = cookie.get_reply()?;
        if reply.value_len() == 0 {
            Err(XcbError::MissingProp(atom, id))
        } else {
            Ok(reply.value()[0])
        }
    }

    // xcb docs: https://www.mankier.com/3/xcb_get_property
    fn get_str_prop(&self, id: WinId, name: &str) -> Result<String> {
        let a = self.atom(name)?;
        let cookie = xcb::get_property(&self.conn, false, id, a, xcb::ATOM_ANY, 0, 1024);
        Ok(String::from_utf8(cookie.get_reply()?.value().to_vec())?)
    }

    // xcb docs: https://www.mankier.com/3/xcb_change_property
    fn replace_prop(&self, id: WinId, prop: Atom, val: PropVal<'_>) {
        let mode = xcb::PROP_MODE_REPLACE as u8;
        let a = self.known_atom(prop);

        let (ty, data) = match val {
            PropVal::Atom(data) => (xcb::xproto::ATOM_ATOM, data),
            PropVal::Cardinal(data) => (xcb::xproto::ATOM_CARDINAL, data),
            PropVal::Window(data) => (xcb::xproto::ATOM_WINDOW, data),
            PropVal::Str(s) => {
                let (ty, data) = (xcb::xproto::ATOM_STRING, s.as_bytes());
                xcb::change_property(&self.conn, mode, id, a, ty, 8, data);
                return;
            }
        };

        xcb::change_property(&self.conn, mode, id, a, ty, 32, data);
    }

    fn create_window(&self, ty: WinType, reg: Region, managed: bool) -> Result<WinId> {
        let (ty, mut data, class, root, depth, visual_id) = match ty {
            WinType::CheckWin => (
                None,
                Vec::new(),
                xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                self.root,
                0,
                0,
            ),

            WinType::InputOnly => (
                None,
                Vec::new(),
                xcb::WINDOW_CLASS_INPUT_ONLY as u16,
                self.root,
                0,
                0,
            ),

            WinType::InputOutput(a) => {
                let colormap = self.conn.generate_id();
                let screen = self.screen(0)?;
                let depth = self.get_depth(&screen)?;
                let visual = self.get_visual_type(&depth)?;

                xcb::xproto::create_colormap(
                    &self.conn,
                    xcb::COLORMAP_ALLOC_NONE as u8,
                    colormap,
                    screen.root(),
                    visual.visual_id(),
                );

                (
                    Some(a),
                    vec![
                        (xcb::CW_BORDER_PIXEL, screen.black_pixel()),
                        (xcb::CW_COLORMAP, colormap),
                        (
                            xcb::CW_EVENT_MASK,
                            xcb::EVENT_MASK_EXPOSURE | xcb::EVENT_MASK_KEY_PRESS,
                        ),
                    ],
                    xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
                    screen.root(),
                    depth.depth(),
                    visual.visual_id(),
                )
            }
        };

        if !managed {
            data.push((xcb::CW_OVERRIDE_REDIRECT, 1))
        }

        let (x, y, w, h) = reg.values();
        let id = self.conn.generate_id();
        let border_width = 0;

        // xcb docs: https://www.mankier.com/3/xcb_create_window
        xcb::create_window(
            &self.conn, // xcb connection to X11
            depth,      // new window's depth
            id,         // ID to be used for referring to the window
            root,       // parent window
            x as i16,
            y as i16,
            w as u16,
            h as u16,
            border_width,
            class,
            visual_id,
            &data,
        );

        // Input only windows don't need mapping
        if let Some(atom) = ty {
            let net_name = Atom::NetWmWindowType;
            self.replace_prop(id, net_name, PropVal::Atom(&[self.known_atom(atom)]));
            self.map_window(id);
        }

        self.flush();
        Ok(id)
    }

    fn configure_window(&self, id: WinId, conf: &[WinConfig]) {
        let data: Vec<(u16, u32)> = conf.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        xcb::configure_window(&self.conn, id, &data);
    }

    fn destroy_window(&self, id: WinId) {
        xcb::destroy_window(&self.conn, id);
    }

    fn map_window(&self, id: WinId) {
        xcb::map_window(&self.conn, id);
    }

    fn mark_focused_window(&self, id: WinId) {
        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            &self.conn,                    // xcb connection to X11
            xcb::INPUT_FOCUS_PARENT as u8, // focus the parent when focus is lost
            id,                            // window to focus
            0,                             // event time (0 == current time)
        );

        self.replace_prop(id, Atom::NetActiveWindow, PropVal::Window(&[id]));
    }

    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()> {
        let atom = self.atom(atom_name)?;
        let wm_protocols = self.known_atom(Atom::WmProtocols);
        let data = xcb::ClientMessageData::from_data32([atom, xcb::CURRENT_TIME, 0, 0, 0]);
        let event = xcb::ClientMessageEvent::new(32, id, wm_protocols, data);

        xcb::send_event(&self.conn, false, id, xcb::EVENT_MASK_NO_EVENT, &event);
        Ok(())
    }

    fn set_window_attributes(&self, id: WinId, attrs: &[WinAttr]) {
        let data: Vec<(u32, u32)> = attrs.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        xcb::change_window_attributes(&self.conn, id, &data);
    }

    fn unmap_window(&self, id: WinId) {
        xcb::unmap_window(&self.conn, id);
    }

    fn window_geometry(&self, id: WinId) -> Result<Region> {
        let res = xcb::get_geometry(&self.conn, id).get_reply()?;
        Ok(Region::new(
            res.x() as u32,
            res.y() as u32,
            res.width() as u32,
            res.height() as u32,
        ))
    }

    // logic taken from https://github.com/rtbo/rust-xcb/blob/master/examples/randr_crtc_info.rs
    fn current_screens(&self) -> Result<Vec<Screen>> {
        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let resources = xcb::randr::get_screen_resources(&self.conn, self.check_win);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        Ok(resources
            .get_reply()?
            .crtcs()
            .iter()
            .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
            .enumerate()
            .map(|(i, r)| {
                let region = Region::new(
                    r.x() as u32,
                    r.y() as u32,
                    r.width() as u32,
                    r.height() as u32,
                );
                Screen::new(region, i)
            })
            .filter(|s| {
                let (_, _, w, _) = s.region(false).values();
                w > 0
            })
            .collect())
    }

    fn screen_sizes(&self) -> Result<Vec<Region>> {
        self.current_screens()
            .map(|screens| screens.iter().map(|s| s.region(false)).collect())
    }

    fn current_clients(&self) -> Result<Vec<WinId>> {
        Ok(xcb::query_tree(&self.conn, self.root)
            .get_reply()
            .map(|reply| reply.children().into())?)
    }

    fn cursor_position(&self) -> Point {
        xcb::query_pointer(&self.conn, self.root)
            .get_reply()
            .map_or_else(
                |_| Point::new(0, 0),
                |reply| Point::new(reply.root_x() as u32, reply.root_y() as u32),
            )
    }

    fn flush(&self) -> bool {
        self.conn.flush()
    }

    fn focused_client(&self) -> Result<WinId> {
        // xcb docs: https://www.mankier.com/3/xcb_get_input_focus
        Ok(xcb::get_input_focus(&self.conn).get_reply()?.focus())
    }

    fn grab_keys(&self, keys: &[&KeyCode]) {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, xcb::MOD_MASK_2 as u16];
        let mode = xcb::GRAB_MODE_ASYNC as u8;

        for m in modifiers.iter() {
            for k in keys.iter() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_key
                xcb::grab_key(
                    &self.conn, // xcb connection to X11
                    false,      // don't pass grabbed events through to the client
                    self.root,  // the window to grab: in this case the root window
                    k.mask | m, // modifiers to grab
                    k.code,     // keycode to grab
                    mode,       // don't lock pointer input while grabbing
                    mode,       // don't lock keyboard input while grabbing
                );
            }
        }
        self.flush();
    }

    fn grab_mouse_buttons(&self, states: &[&MouseState]) {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, xcb::MOD_MASK_2 as u16];
        let mode = xcb::GRAB_MODE_ASYNC as u8;
        let mask = (xcb::EVENT_MASK_BUTTON_PRESS
            | xcb::EVENT_MASK_BUTTON_RELEASE
            | xcb::EVENT_MASK_BUTTON_MOTION) as u16;

        for m in modifiers.iter() {
            for state in states.iter() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_button
                xcb::grab_button(
                    &self.conn,       // xcb connection to X11
                    false,            // don't pass grabbed events through to the client
                    self.root,        // the window to grab: in this case the root window
                    mask,             // which events are reported to the client
                    mode,             // don't lock pointer input while grabbing
                    mode,             // don't lock keyboard input while grabbing
                    xcb::NONE,        // don't confine the cursor to a specific window
                    xcb::NONE,        // don't change the cursor type
                    state.button(),   // the button to grab
                    state.mask() | m, // modifiers to grab
                );
            }
        }
        self.flush();
    }

    fn root(&self) -> WinId {
        self.root
    }

    fn set_randr_notify_mask(&self) -> Result<()> {
        let mask = (xcb::randr::NOTIFY_MASK_OUTPUT_CHANGE
            | xcb::randr::NOTIFY_MASK_CRTC_CHANGE
            | xcb::randr::NOTIFY_MASK_SCREEN_CHANGE) as u16;

        // xcb docs: https://www.mankier.com/3/xcb_randr_select_input
        xcb::randr::select_input(&self.conn, self.root, mask).request_check()?;
        self.flush();
        Ok(())
    }

    fn ungrab_keys(&self) {
        // xcb docs: https://www.mankier.com/3/xcb_ungrab_key
        xcb::ungrab_key(
            &self.conn, // xcb connection to X11
            xcb::GRAB_ANY as u8,
            self.root, // the window to ungrab keys for
            xcb::MOD_MASK_ANY as u16,
        );
    }

    fn ungrab_mouse_buttons(&self) {
        // xcb docs: https://www.mankier.com/3/xcb_ungrab_button
        xcb::ungrab_button(
            &self.conn, // xcb connection to X11
            xcb::GRAB_ANY as u8,
            self.root, // the window to ungrab keys for
            xcb::MOD_MASK_ANY as u16,
        );
    }

    // Loop until we get an event we care about or an error
    fn wait_for_event(&self) -> Result<XEvent> {
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                // Got an event but it might not be one we care about / know how to handle
                if let Some(e) = self.generic_xcb_to_xevent(event)? {
                    return Ok(e);
                }
            } else {
                // Conn returned None which _should_ mean an error
                if let Err(e) = self.conn.has_error() {
                    return Err(e.into());
                }
            }
        }
    }

    fn poll_for_event(&self) -> Result<Option<XEvent>> {
        if let Some(event) = self.conn.poll_for_event() {
            self.generic_xcb_to_xevent(event)
        } else {
            Ok(self.conn.has_error().map(|_| None)?)
        }
    }

    fn warp_cursor(&self, id: WinId, x: usize, y: usize) {
        // conn source target source(x y w h) dest(x y)
        xcb::warp_pointer(&self.conn, 0, id, 0, 0, 0, 0, x as i16, y as i16);
    }
}
