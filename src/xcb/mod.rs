//! XCB implementations of the XConn trait and related helpers
use crate::{
    bindings::{CodeMap, KeyBindings, KeyCode, KeyEventHandler, MouseEvent, MouseState},
    geometry::{Point, Rect},
    x::{
        atom::Atom,
        event::{
            ClientEventMask, ClientMessage, ClientMessageData, ConfigureEvent, ExposeEvent,
            PointerChange, PropertyEvent,
        },
        property::{
            MapState, Prop, WindowAttributes, WindowClass, WmHints, WmNormalHints, WmState,
        },
        ClientAttr, ClientConfig, XConn, XEvent,
    },
    Error, Result, Xid,
};
use std::{cell::RefCell, collections::HashMap, fmt, process::Command};
use strum::IntoEnumIterator;
use tracing::{error, trace, warn};

pub mod conversions;
pub mod error;

pub use error::XErrorCode;

/// A generic event type returned by the xcb library
pub type XcbGenericEvent = xcb::Event<xcb::ffi::base::xcb_generic_event_t>;

const RANDR_MAJ: u32 = 1;
const RANDR_MIN: u32 = 2;

fn keycodes_from_xmodmap() -> Result<CodeMap> {
    let output = Command::new("xmodmap").arg("-pke").output()?;
    let m = String::from_utf8(output.stdout)?
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
            words.skip(1).map(move |name| (name.into(), key_code))
        })
        .collect();

    Ok(m)
}

fn parse_binding(pattern: &str, known_codes: &CodeMap) -> Result<KeyCode> {
    let mut parts: Vec<&str> = pattern.split('-').collect();
    let name = parts.remove(parts.len() - 1);

    match known_codes.get(name) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|&s| match s {
                    "A" => Ok(xcb::MOD_MASK_1),
                    "M" => Ok(xcb::MOD_MASK_4),
                    "S" => Ok(xcb::MOD_MASK_SHIFT),
                    "C" => Ok(xcb::MOD_MASK_CONTROL),
                    _ => Err(Error::UnknownModifier { name: s.to_owned() }),
                })
                .try_fold(0, |acc, v| v.map(|inner| acc | inner))?;

            trace!(?pattern, mask, code, "parsed keybinding");
            Ok(KeyCode {
                mask: mask as u16,
                code: *code,
            })
        }

        None => Err(Error::UnknownKeyName {
            name: name.to_owned(),
        }),
    }
}

/// A connection to the X server using the XCB C API
pub struct XcbConn {
    conn: xcb::Connection,
    root: Xid,
    randr_base: u8,
    atoms: RefCell<HashMap<String, Xid>>,
}

impl fmt::Debug for XcbConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XcbConn")
            .field("root", &self.root)
            .field("randr_base", &self.randr_base)
            .field("atoms", &self.atoms)
            .finish()
    }
}

impl XcbConn {
    /// Connect to the X server using the [XCB API][1]
    ///
    /// Each [XcbConn] contains and embedded [xcb Connection][2] which is used for making
    /// all api calls through to the X server. Some state is cached in the struct itself
    /// in order to prevent redundant calls through to the X server.
    ///
    /// Creating a new [XcbConn] instance will establish the underlying connection and if
    /// the `keysyms` feature is enabled, pull [KeyCode] mappings from the user
    /// system using `xmodmap`.
    ///
    /// [1]: http://rtbo.github.io/rust-xcb
    /// [2]: http://rtbo.github.io/rust-xcb/xcb/base/struct.Connection.html
    pub fn new() -> Result<Self> {
        let (conn, _) = xcb::Connection::connect(None)?;
        let mut api = Self {
            conn,
            root: Xid(0),
            randr_base: 0,
            atoms: RefCell::new(HashMap::new()),
        };
        api.init()?;

        Ok(api)
    }

    fn init(&mut self) -> Result<()> {
        self.root = match self.conn.get_setup().roots().next() {
            Some(r) => Xid(r.root()),
            None => return Err(Error::NoScreens),
        };
        self.randr_base = self
            .conn
            .get_extension_data(xcb::randr::id())
            .ok_or_else(|| Error::Randr("unable to fetch extension data".into()))?
            .first_event();

        // Make sure we have new enough RandR so we can use 'get_screen_resources'
        // See https://github.com/sminez/penrose/issues/115 for more details
        let cookie = xcb::randr::query_version(&self.conn, RANDR_MAJ, RANDR_MIN);
        let reply = cookie.get_reply()?;
        let (maj, min) = (reply.major_version(), reply.minor_version());
        if (maj, min) != (RANDR_MAJ, RANDR_MIN) {
            return Err(Error::Randr(format!(
                "penrose requires RandR version >= {}.{}: detected {}.{}\nplease update RandR to a newer version",
                RANDR_MAJ, RANDR_MIN, maj, min
            )));
        }

        self.atoms = RefCell::new(HashMap::with_capacity(Atom::iter().count()));

        for a in Atom::iter() {
            self.intern_atom(a.as_ref())?;
        }

        Ok(())
    }

    pub fn parse_keybindings_with_xmodmap<S, E>(
        &self,
        str_bindings: HashMap<S, Box<dyn KeyEventHandler<Self, E>>>,
    ) -> Result<KeyBindings<Self, E>>
    where
        S: AsRef<str>,
    {
        let m = keycodes_from_xmodmap()?;

        str_bindings
            .into_iter()
            .map(|(s, v)| parse_binding(s.as_ref(), &m).map(|k| (k, v)))
            .collect()
    }

    /// Fetch the id value of a known [Atom] variant.
    ///
    /// This operation is expected to always succeed as known atoms should
    /// either be interned on init of the implementing struct or statically
    /// assigned a value in the implementation.
    pub fn known_atom(&self, atom: Atom) -> Xid {
        *self
            .atoms
            .borrow()
            .get(atom.as_ref())
            .expect("All Atom variants to be interned on init")
    }

    pub fn check_window(&self) -> Xid {
        let id = self.conn.generate_id();
        xcb::create_window(
            &self.conn,
            0,
            id,
            *self.root,
            0,
            0,
            1,
            1,
            0,
            0,
            0,
            &[(xcb::CW_OVERRIDE_REDIRECT, 1)],
        );
        self.conn.flush();

        Xid(id)
    }

    pub fn destroy_client(&self, client: Xid) -> Result<()> {
        Ok(xcb::destroy_window_checked(&self.conn, *client).request_check()?)
    }

    fn generic_xcb_to_xevent(&self, event: XcbGenericEvent) -> Result<Option<XEvent>> {
        let xcb_response_type_mask: u8 = 0x7F;
        let numlock = xcb::MOD_MASK_2 as u16;

        let etype = event.response_type() & xcb_response_type_mask;

        // Need to apply the randr_base mask as well which doesn't seem to work in 'match'
        if etype == self.randr_base + xcb::randr::NOTIFY {
            return Ok(Some(XEvent::RandrNotify));
        } else if etype == self.randr_base + xcb::randr::SCREEN_CHANGE_NOTIFY {
            return Ok(Some(XEvent::ScreenChange));
        }

        let evt = match etype {
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

                Some(XEvent::MapRequest(Xid(id)))
            }

            xcb::ENTER_NOTIFY => {
                let e: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Enter(PointerChange {
                    id: Xid(e.event()),
                    abs: Point::new(e.root_x() as u32, e.root_y() as u32),
                    relative: Point::new(e.event_x() as u32, e.event_y() as u32),
                }))
            }

            xcb::LEAVE_NOTIFY => {
                let e: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Leave(PointerChange {
                    id: Xid(e.event()),
                    abs: Point::new(e.root_x() as u32, e.root_y() as u32),
                    relative: Point::new(e.event_x() as u32, e.event_y() as u32),
                }))
            }

            xcb::FOCUS_IN => {
                let e: &xcb::FocusInEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::FocusIn(Xid(e.event())))
            }

            xcb::DESTROY_NOTIFY => {
                let e: &xcb::DestroyNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Destroy(Xid(e.window())))
            }

            xcb::CONFIGURE_NOTIFY => {
                let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::ConfigureNotify(ConfigureEvent {
                    id: Xid(e.window()),
                    r: Rect {
                        x: e.x() as u32,
                        y: e.y() as u32,
                        w: e.width() as u32,
                        h: e.height() as u32,
                    },
                    is_root: e.window() == *self.root,
                }))
            }

            xcb::CONFIGURE_REQUEST => {
                let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::ConfigureRequest(ConfigureEvent {
                    id: Xid(e.window()),
                    r: Rect {
                        x: e.x() as u32,
                        y: e.y() as u32,
                        w: e.width() as u32,
                        h: e.height() as u32,
                    },
                    is_root: e.window() == *self.root,
                }))
            }

            xcb::EXPOSE => {
                let e: &xcb::ExposeEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Expose(ExposeEvent {
                    id: Xid(e.window()),
                    r: Rect {
                        x: e.x() as u32,
                        y: e.y() as u32,
                        w: e.width() as u32,
                        h: e.height() as u32,
                    },
                    count: e.count() as usize,
                }))
            }

            xcb::UNMAP_NOTIFY => {
                let e: &xcb::UnmapNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::UnmapNotify(Xid(e.window())))
            }

            xcb::CLIENT_MESSAGE => {
                let e: &xcb::ClientMessageEvent = unsafe { xcb::cast_event(&event) };
                xcb::xproto::get_atom_name(&self.conn, e.type_())
                    .get_reply()
                    .map_err(Error::from)
                    .and_then(|a| {
                        Ok(ClientMessage::new(
                            Xid(e.window()),
                            ClientEventMask::NoEventMask,
                            a.name(),
                            match e.format() {
                                8 => ClientMessageData::try_from(e.data().data8()),
                                16 => ClientMessageData::try_from(e.data().data16()),
                                32 => ClientMessageData::try_from(e.data().data32()),
                                _ => unreachable!(
                                    "ClientMessageEvent.format should really be an enum..."
                                ),
                            }
                            .map_err(|_| Error::InvalidClientMessage { format: e.format() })?,
                        ))
                    })
                    .map(XEvent::ClientMessage)
                    .ok()
            }

            xcb::PROPERTY_NOTIFY => {
                let e: &xcb::PropertyNotifyEvent = unsafe { xcb::cast_event(&event) };
                xcb::xproto::get_atom_name(&self.conn, e.atom())
                    .get_reply()
                    .ok()
                    .map(|a| {
                        XEvent::PropertyNotify(PropertyEvent {
                            id: Xid(e.window()),
                            atom: a.name().to_string(),
                            is_root: e.window() == *self.root,
                        })
                    })
            }

            0 => {
                let e: &xcb::GenericError = unsafe { xcb::cast_event(&event) };
                return Err(Error::from(e));
            }

            // NOTE: ignoring other event types
            _ => None,
        };

        Ok(evt)
    }
}

impl XConn for XcbConn {
    fn root(&self) -> Xid {
        self.root
    }

    fn intern_atom(&self, name: &str) -> Result<Xid> {
        if let Some(atom) = self.atoms.borrow().get(name) {
            return Ok(*atom);
        }

        trace!(name, "interning atom");
        let atom = Xid(xcb::intern_atom(&self.conn, false, name)
            .get_reply()?
            .atom());

        self.atoms.borrow_mut().insert(name.to_owned(), atom);

        Ok(atom)
    }

    fn atom_name(&self, atom: Xid) -> Result<String> {
        let name = xcb::get_atom_name(&self.conn, *atom)
            .get_reply()?
            .name()
            .to_string();

        Ok(name)
    }

    // logic taken from https://github.com/rtbo/rust-xcb/blob/master/examples/randr_crtc_info.rs
    fn screen_details(&self) -> Result<Vec<Rect>> {
        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let check_win = self.check_window();
        let resources = xcb::randr::get_screen_resources(&self.conn, *check_win);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        let rects = resources
            .get_reply()?
            .crtcs()
            .iter()
            .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
            .map(|r| Rect {
                x: r.x() as u32,
                y: r.y() as u32,
                w: r.width() as u32,
                h: r.height() as u32,
            })
            .collect();

        self.destroy_client(check_win)?;

        Ok(rects)
    }

    fn cursor_position(&self) -> Result<Point> {
        let p = xcb::query_pointer(&self.conn, *self.root)
            .get_reply()
            .map(|reply| Point::new(reply.root_x() as u32, reply.root_y() as u32))?;

        Ok(p)
    }

    fn grab(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, xcb::MOD_MASK_2 as u16];
        let mode = xcb::GRAB_MODE_ASYNC as u8;
        let mask = (xcb::EVENT_MASK_BUTTON_PRESS
            | xcb::EVENT_MASK_BUTTON_RELEASE
            | xcb::EVENT_MASK_BUTTON_MOTION) as u16;

        for m in modifiers.iter() {
            for k in key_codes.iter() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_key
                xcb::grab_key_checked(
                    &self.conn, // xcb connection to X11
                    false,      // don't pass grabbed events through to the client
                    *self.root, // the window to grab: in this case the root window
                    k.mask | m, // modifiers to grab
                    k.code,     // keycode to grab
                    mode,       // don't lock pointer input while grabbing
                    mode,       // don't lock keyboard input while grabbing
                )
                .request_check()?;
            }
        }

        for m in modifiers.iter() {
            for state in mouse_states.iter() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_button
                xcb::grab_button_checked(
                    &self.conn,       // xcb connection to X11
                    false,            // don't pass grabbed events through to the client
                    *self.root,       // the window to grab: in this case the root window
                    mask,             // which events are reported to the client
                    mode,             // don't lock pointer input while grabbing
                    mode,             // don't lock keyboard input while grabbing
                    xcb::NONE,        // don't confine the cursor to a specific window
                    xcb::NONE,        // don't change the cursor type
                    state.button(),   // the button to grab
                    state.mask() | m, // modifiers to grab
                )
                .request_check()?;
            }
        }

        self.flush();

        Ok(())
    }

    fn next_event(&self) -> Result<XEvent> {
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

    fn flush(&self) {
        self.conn.flush();
    }

    fn float_location(&self, client: Xid) -> Result<Rect> {
        let res = xcb::get_geometry(&self.conn, *client).get_reply()?;

        Ok(Rect {
            x: res.x() as u32,
            y: res.y() as u32,
            w: res.width() as u32,
            h: res.height() as u32,
        })
    }

    fn map(&self, client: Xid) -> Result<()> {
        xcb::map_window_checked(&self.conn, *client).request_check()?;

        Ok(())
    }

    fn unmap(&self, client: Xid) -> Result<()> {
        xcb::unmap_window_checked(&self.conn, *client).request_check()?;

        Ok(())
    }

    fn kill(&self, client: Xid) -> Result<()> {
        xcb::kill_client_checked(&self.conn, *client).request_check()?;

        Ok(())
    }

    fn focus(&self, client: Xid) -> Result<()> {
        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            &self.conn,                    // xcb connection to X11
            xcb::INPUT_FOCUS_PARENT as u8, // focus the parent when focus is lost
            *client,                       // window to focus
            xcb::CURRENT_TIME,             // event time (0 == current time)
        );

        self.set_prop(
            self.root,
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![client]),
        )
    }

    fn get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>> {
        let atom = *self.intern_atom(prop_name)?;
        let cookie = xcb::get_property(&self.conn, false, *client, atom, xcb::ATOM_ANY, 0, 1024);
        let r = cookie.get_reply()?;
        let prop_type = self.atom_name(Xid(r.type_()))?;

        let p = match prop_type.as_ref() {
            "ATOM" => Prop::Atom(
                r.value()
                    .iter()
                    .map(|a| self.atom_name(*a))
                    .collect::<Result<Vec<String>>>()?,
            ),

            "CARDINAL" => Prop::Cardinal(r.value()[0]),

            "STRING" => Prop::UTF8String(
                String::from_utf8_lossy(r.value())
                    .trim_matches('\0')
                    .split('\0')
                    .map(|s| s.to_string())
                    .collect(),
            ),

            "UTF8_STRING" => Prop::UTF8String(
                String::from_utf8(r.value().to_vec())?
                    .trim_matches('\0')
                    .split('\0')
                    .map(|s| s.to_string())
                    .collect(),
            ),

            "WINDOW" => Prop::Window(r.value().to_vec()),

            "WM_HINTS" => Prop::WmHints(WmHints::try_from_bytes(r.value())?),

            "WM_SIZE_HINTS" => Prop::WmNormalHints(WmNormalHints::try_from_bytes(r.value())?),

            // Default to returning the raw bytes as u32s which the user can then
            // convert as needed if the prop type is not one we recognise
            // NOTE: I _really_ don't like this about the rust-xcb api...
            _ => Prop::Bytes(match r.format() {
                8 => r.value::<u8>().iter().map(|b| *b as u32).collect(),
                16 => r.value::<u16>().iter().map(|b| *b as u32).collect(),
                32 => r.value::<u32>().to_vec(),
                _ => {
                    error!(
                        "prop type for {} was {} which claims to have a data format of {}",
                        prop_name,
                        prop_type,
                        r.type_()
                    );

                    return Ok(None);
                }
            }),
        };

        Ok(Some(p))
    }

    fn get_window_attributes(&self, client: Xid) -> Result<WindowAttributes> {
        let win_attrs = xcb::get_window_attributes(&self.conn, *client).get_reply()?;
        let override_redirect = win_attrs.override_redirect();
        let map_state = match win_attrs.map_state() {
            0 => MapState::Unmapped,
            1 => MapState::UnViewable,
            2 => MapState::Viewable,
            s => panic!("got invalid map state from x server: {s}"),
        };
        let window_class = match win_attrs.class() {
            0 => WindowClass::CopyFromParent,
            1 => WindowClass::InputOutput,
            2 => WindowClass::InputOnly,
            c => panic!("got invalid window class from x server: {c}"),
        };

        let wa = WindowAttributes::new(override_redirect, map_state, window_class);

        Ok(wa)
    }

    fn set_wm_state(&self, client: Xid, wm_state: WmState) -> Result<()> {
        let mode = xcb::PROP_MODE_REPLACE as u8;
        let a = *self.known_atom(Atom::WmState);
        let state = match wm_state {
            WmState::Withdrawn => 0,
            WmState::Normal => 1,
            WmState::Iconic => 3,
        };

        let cookie = xcb::change_property_checked(&self.conn, mode, *client, a, a, 32, &[state]);
        match cookie.request_check().map_err(Error::from) {
            // The window is already gone
            Err(Error::XcbKnown(XErrorCode::BadWindow)) => (),
            other => other?,
        }

        Ok(())
    }

    fn set_prop(&self, client: Xid, name: &str, val: Prop) -> Result<()> {
        let mode = xcb::PROP_MODE_REPLACE as u8;
        let a = self.intern_atom(name)?;

        let (ty, data) = match val {
            Prop::Atom(atoms) => (
                xcb::xproto::ATOM_ATOM,
                atoms
                    .iter()
                    .map(|a| self.intern_atom(a).map(|id| *id))
                    .collect::<Result<Vec<u32>>>()?,
            ),

            Prop::Cardinal(val) => (xcb::xproto::ATOM_CARDINAL, vec![val]),

            Prop::Window(ids) => (
                xcb::xproto::ATOM_WINDOW,
                ids.into_iter().map(|id| *id).collect(),
            ),

            Prop::UTF8String(strs) => {
                return Ok(xcb::change_property_checked(
                    &self.conn,
                    mode,
                    *client,
                    *a,
                    xcb::xproto::ATOM_STRING,
                    8,
                    strs.join("\0").as_bytes(),
                )
                .request_check()?);
            }

            // FIXME: handle changing WmHints and WmNormalHints correctly in change_prop
            Prop::Bytes(_) | Prop::WmHints(_) | Prop::WmNormalHints(_) => {
                panic!("unable to change Prop, WmHints or WmNormalHints properties");
            }
        };

        xcb::change_property_checked(&self.conn, mode, *client, *a, ty, 32, &data)
            .request_check()?;

        Ok(())
    }

    fn set_client_attributes(&self, client: Xid, attrs: &[ClientAttr]) -> Result<()> {
        let data: Vec<(u32, u32)> = attrs.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        xcb::change_window_attributes_checked(&self.conn, *client, &data).request_check()?;

        Ok(())
    }

    fn set_client_config(&self, client: Xid, data: &[ClientConfig]) -> Result<()> {
        let data: Vec<(u16, u32)> = data.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        xcb::configure_window_checked(&self.conn, *client, &data).request_check()?;

        Ok(())
    }

    fn send_client_message(&self, msg: ClientMessage) -> Result<()> {
        let (dtype, d) = (*self.intern_atom(&msg.dtype)?, msg.data().as_u32());
        let data = xcb::ClientMessageData::from_data32([d[0], d[1], d[2], d[3], d[4]]);
        let event = xcb::ClientMessageEvent::new(32, *msg.id, dtype, data);
        let mask = match msg.mask {
            ClientEventMask::NoEventMask => xcb::EVENT_MASK_NO_EVENT,
            ClientEventMask::SubstructureNotify => xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY,
        };

        xcb::send_event_checked(&self.conn, false, *msg.id, mask, &event).request_check()?;

        Ok(())
    }

    fn warp_cursor(&self, p: Point) -> Result<()> {
        // conn source target source(x y w h) dest(x y)
        xcb::warp_pointer_checked(&self.conn, 0, 0, 0, 0, 0, 0, p.x as i16, p.y as i16)
            .request_check()?;

        Ok(())
    }
}
