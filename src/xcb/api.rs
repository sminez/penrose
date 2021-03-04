//! A wrapper around the underlying xcb api layer that only exposes Penrose types
use crate::{
    core::{
        bindings::{KeyCode, KeyCodeMask, KeyCodeValue, MouseEvent, MouseState},
        data_types::{Point, Region, WinType},
        helpers::spawn_for_output,
        screen::Screen,
        xconnection::{
            Atom, ClientAttr, ClientConfig, ClientEventMask, ClientMessage, ClientMessageData,
            ClientMessageKind, ConfigureEvent, ExposeEvent, MapState, PointerChange, Prop,
            PropertyEvent, WindowAttributes, WindowClass, WindowState, WmHints, WmNormalHints,
            XAtomQuerier, XEvent, Xid,
        },
    },
    xcb::{Result, XErrorCode, XcbError, XcbGenericEvent},
};
use strum::*;

use std::{collections::HashMap, convert::TryFrom, fmt, str::FromStr};

#[cfg(feature = "keysyms")]
use crate::core::{bindings::KeyPress, xconnection::KeyPressParseAttempt};
#[cfg(feature = "keysyms")]
use penrose_keysyms::XKeySym;

/// A reverse lookup of KeyCode mask and value to key as a String using XKeySym mappings
pub type ReverseCodeMap = HashMap<(KeyCodeMask, KeyCodeValue), String>;

const RANDR_MAJ: u32 = 1;
const RANDR_MIN: u32 = 2;

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
    root: Xid,
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

impl XAtomQuerier for Api {
    fn atom_name(&self, atom: Xid) -> crate::core::xconnection::Result<String> {
        Ok(self.atom_name(atom)?)
    }

    fn atom_id(&self, name: &str) -> crate::core::xconnection::Result<Xid> {
        Ok(self.atom(name)?)
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

        // Make sure we have new enough RandR so we can use 'get_screen_resources'
        // See https://github.com/sminez/penrose/issues/115 for more details
        let cookie = xcb::randr::query_version(&self.conn, RANDR_MAJ, RANDR_MIN);
        let reply = cookie.get_reply()?;
        let (maj, min) = (reply.major_version(), reply.minor_version());
        if (maj, min) != (RANDR_MAJ, RANDR_MIN) {
            return Err(XcbError::Randr(format!(
                "penrose requires RandR version >= {}.{}: detected {}.{}\nplease update RandR to a newer version",
                RANDR_MAJ, RANDR_MIN, maj, min
            )));
        }

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

    /// Fetch the name of an X atom id
    pub fn atom_name(&self, atom: Xid) -> Result<String> {
        Ok(xcb::get_atom_name(&self.conn, atom)
            .get_reply()?
            .name()
            .to_string())
    }

    /// List the X window properties set on the requested client by name
    pub fn list_props(&self, id: Xid) -> Result<Vec<String>> {
        xcb::list_properties(&self.conn, id)
            .get_reply()?
            .atoms()
            .iter()
            .map(|a| self.atom_name(*a))
            .collect::<Result<Vec<String>>>()
    }

    /// Get a handle on the underlying xcb connection
    pub fn conn(&self) -> &xcb::Connection {
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

    /// Fetch the requested property for the target window
    pub fn get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
        let atom = self.atom(name)?;
        let cookie = xcb::get_property(&self.conn, false, id, atom, xcb::ATOM_ANY, 0, 1024);
        let r = cookie.get_reply()?;
        let prop_type = self.atom_name(r.type_())?;

        Ok(match prop_type.as_ref() {
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

            "WM_HINTS" => Prop::WmHints(
                WmHints::try_from_bytes(r.value())
                    .map_err(|e| XcbError::InvalidPropertyData(e.to_string()))?,
            ),

            "WM_SIZE_HINTS" => Prop::WmNormalHints(
                WmNormalHints::try_from_bytes(r.value())
                    .map_err(|e| XcbError::InvalidPropertyData(e.to_string()))?,
            ),

            // Default to returning the raw bytes as u32s which the user can then
            // convert as needed if the prop type is not one we recognise
            // NOTE: I _really_ don't like this about the rust-xcb api...
            _ => Prop::Bytes(match r.format() {
                8 => r.value::<u8>().iter().map(|b| *b as u32).collect(),
                16 => r.value::<u16>().iter().map(|b| *b as u32).collect(),
                32 => r.value::<u32>().to_vec(),
                _ => {
                    return Err(XcbError::InvalidPropertyData(format!(
                        "prop type for {} was {} which claims to have a data format of {}",
                        name,
                        prop_type,
                        r.type_()
                    )))
                }
            }),
        })
    }

    /// Fetch the `WindowAttributes` data for a target client id
    pub fn get_window_attributes(&self, id: Xid) -> Result<WindowAttributes> {
        let win_attrs = xcb::get_window_attributes(&self.conn, id).get_reply()?;
        let override_redirect = win_attrs.override_redirect();
        let map_state = match win_attrs.map_state() {
            0 => MapState::Unmapped,
            1 => MapState::UnViewable,
            2 => MapState::Viewable,
            s => return Err(XcbError::Raw(format!("invalid map state: {}", s))),
        };
        let window_class = match win_attrs.class() {
            0 => WindowClass::CopyFromParent,
            1 => WindowClass::InputOutput,
            2 => WindowClass::InputOnly,
            c => return Err(XcbError::Raw(format!("invalid window class: {}", c))),
        };

        Ok(WindowAttributes::new(
            override_redirect,
            map_state,
            window_class,
        ))
    }

    /// Grab control of all keyboard input
    pub fn grab_keyboard(&self) -> Result<()> {
        xcb::grab_keyboard(
            &self.conn,
            true,
            self.root(),
            xcb::CURRENT_TIME,
            xcb::GRAB_MODE_ASYNC as u8,
            xcb::GRAB_MODE_ASYNC as u8,
        )
        .get_reply()?;

        Ok(())
    }

    /// Release keyboard input
    pub fn ungrab_keyboard(&self) -> Result<()> {
        xcb::ungrab_keyboard_checked(&self.conn, xcb::CURRENT_TIME).request_check()?;

        Ok(())
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
        } else if etype == self.randr_base + xcb::randr::SCREEN_CHANGE_NOTIFY {
            return Ok(Some(XEvent::ScreenChange));
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
                    .map(|r| XEvent::MapRequest(id, r.override_redirect()))
            }

            xcb::ENTER_NOTIFY => {
                let e: &xcb::EnterNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Enter(PointerChange {
                    id: e.event(),
                    abs: Point::new(e.root_x() as u32, e.root_y() as u32),
                    relative: Point::new(e.event_x() as u32, e.event_y() as u32),
                }))
            }

            xcb::LEAVE_NOTIFY => {
                let e: &xcb::LeaveNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Leave(PointerChange {
                    id: e.event(),
                    abs: Point::new(e.root_x() as u32, e.root_y() as u32),
                    relative: Point::new(e.event_x() as u32, e.event_y() as u32),
                }))
            }

            xcb::FOCUS_IN => {
                let e: &xcb::FocusInEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::FocusIn(e.event()))
            }

            xcb::DESTROY_NOTIFY => {
                let e: &xcb::DestroyNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Destroy(e.window()))
            }

            xcb::CONFIGURE_NOTIFY => {
                let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::ConfigureNotify(ConfigureEvent {
                    id: e.window(),
                    r: Region::new(
                        e.x() as u32,
                        e.y() as u32,
                        e.width() as u32,
                        e.height() as u32,
                    ),
                    is_root: e.window() == self.root,
                }))
            }

            xcb::CONFIGURE_REQUEST => {
                let e: &xcb::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::ConfigureRequest(ConfigureEvent {
                    id: e.window(),
                    r: Region::new(
                        e.x() as u32,
                        e.y() as u32,
                        e.width() as u32,
                        e.height() as u32,
                    ),
                    is_root: e.window() == self.root,
                }))
            }

            xcb::EXPOSE => {
                let e: &xcb::ExposeEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::Expose(ExposeEvent {
                    id: e.window(),
                    r: Region::new(
                        e.x() as u32,
                        e.y() as u32,
                        e.width() as u32,
                        e.height() as u32,
                    ),
                    count: e.count() as usize,
                }))
            }

            xcb::UNMAP_NOTIFY => {
                let e: &xcb::UnmapNotifyEvent = unsafe { xcb::cast_event(&event) };
                Some(XEvent::UnmapNotify(e.window()))
            }

            xcb::CLIENT_MESSAGE => {
                let e: &xcb::ClientMessageEvent = unsafe { xcb::cast_event(&event) };
                xcb::xproto::get_atom_name(&self.conn, e.type_())
                    .get_reply()
                    .map_err(XcbError::from)
                    .and_then(|a| {
                        Ok(ClientMessage::new(
                            e.window(),
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
                            .map_err(|_| XcbError::InvalidClientMessage(e.format()))?,
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
                            id: e.window(),
                            atom: a.name().to_string(),
                            is_root: e.window() == self.root,
                        })
                    })
            }

            0 => {
                let e: &xcb::GenericError = unsafe { xcb::cast_event(&event) };
                return Err(XcbError::from(e));
            }

            // NOTE: ignoring other event types
            _ => None,
        })
    }

    /// Hydrate this XcbApi to restore internal state following serde deserialization
    #[cfg(feature = "serde")]
    pub fn hydrate(&mut self) -> Result<()> {
        self.init()
    }

    /// Intern an atom by name, returning the corresponding id.
    ///
    /// Can fail if the atom name is not a known X atom or if there
    /// is an issue with communicating with the X server. For known
    /// atoms that are included in the [Atom] enum,
    /// the [`Api::known_atom`] method should be used instead.
    pub fn atom(&self, name: &str) -> Result<u32> {
        if let Ok(known) = Atom::from_str(name) {
            // This could be us initialising in which case self.atoms is empty
            if let Some(atom) = self.atoms.get(&known) {
                return Ok(*atom);
            }
        }

        trace!(name, "interning atom");
        Ok(xcb::intern_atom(&self.conn, false, name)
            .get_reply()?
            .atom())
    }

    /// Fetch the id value of a known [Atom] variant.
    ///
    /// This operation is expected to always succeed as known atoms should
    /// either be interned on init of the implementing struct or statically
    /// assigned a value in the implementation.
    pub fn known_atom(&self, atom: Atom) -> u32 {
        *self.atoms.get(&atom).unwrap()
    }

    /// Delete a known property from a window
    pub fn delete_prop(&self, id: Xid, prop: &str) -> Result<()> {
        Ok(xcb::delete_property_checked(&self.conn, id, self.atom(prop)?).request_check()?)
    }

    /// Replace a property value on a window.
    ///
    /// See the documentation for the C level XCB API for the correct property
    /// type for each prop.
    pub fn change_prop(&self, id: Xid, prop: &str, val: Prop) -> Result<()> {
        let mode = xcb::PROP_MODE_REPLACE as u8;
        let a = self.atom(prop)?;

        let (ty, data) = match val {
            Prop::Atom(atoms) => (
                xcb::xproto::ATOM_ATOM,
                atoms
                    .iter()
                    .map(|a| self.atom(a))
                    .collect::<Result<Vec<u32>>>()?,
            ),

            Prop::Bytes(_) => {
                return Err(XcbError::InvalidPropertyData(
                    "unable to change non standard props".into(),
                ))
            }

            Prop::Cardinal(val) => (xcb::xproto::ATOM_CARDINAL, vec![val]),

            Prop::UTF8String(strs) => {
                return Ok(xcb::change_property_checked(
                    &self.conn,
                    mode,
                    id,
                    a,
                    xcb::xproto::ATOM_STRING,
                    8,
                    strs.join("\0").as_bytes(),
                )
                .request_check()?);
            }

            Prop::Window(ids) => (xcb::xproto::ATOM_WINDOW, ids),

            // FIXME: handle changing WmHints and WmNormalHints correctly in change_prop
            Prop::WmHints(_) | Prop::WmNormalHints(_) => {
                return Err(XcbError::InvalidPropertyData(
                    "unable to change WmHints or WmNormalHints".into(),
                ))
            }
        };

        Ok(xcb::change_property_checked(&self.conn, mode, id, a, ty, 32, &data).request_check()?)
    }

    /// Set the target client's WM_STATE
    pub fn set_client_state(&self, id: Xid, wm_state: WindowState) -> Result<()> {
        let mode = xcb::PROP_MODE_REPLACE as u8;
        let a = self.known_atom(Atom::WmState);
        let state = match wm_state {
            WindowState::Withdrawn => 0,
            WindowState::Normal => 1,
            WindowState::Iconic => 3,
        };

        let cookie = xcb::change_property_checked(&self.conn, mode, id, a, a, 32, &[state]);
        Ok(match cookie.request_check().map_err(XcbError::from) {
            // The window is already gone
            Err(XcbError::XcbKnown(XErrorCode::BadWindow)) => (),
            other => other?,
        })
    }

    /// Create a new client window
    pub fn create_window(&self, ty: WinType, reg: Region, managed: bool) -> Result<Xid> {
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
            let net_name = Atom::NetWmWindowType.as_ref();
            self.change_prop(id, net_name, Prop::Atom(vec![atom.as_ref().into()]))?;
            self.map_client(id)?;
        }

        self.flush();
        Ok(id)
    }

    /// Apply a set of config options to a window
    pub fn configure_client(&self, id: Xid, conf: &[ClientConfig]) -> Result<()> {
        let data: Vec<(u16, u32)> = conf.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        Ok(xcb::configure_window_checked(&self.conn, id, &data).request_check()?)
    }

    /// Destroy the X server state for a given window
    pub fn destroy_client(&self, id: Xid) -> Result<()> {
        Ok(xcb::destroy_window_checked(&self.conn, id).request_check()?)
    }

    /// Send a [XEvent::MapRequest] for the target window
    pub fn map_client(&self, id: Xid) -> Result<()> {
        Ok(xcb::map_window_checked(&self.conn, id).request_check()?)
    }

    /// Unmap the target window
    pub fn unmap_client(&self, id: Xid) -> Result<()> {
        Ok(xcb::unmap_window_checked(&self.conn, id).request_check()?)
    }

    /// Mark the given window as currently having focus in the X server state
    pub fn focus_client(&self, id: Xid) -> Result<()> {
        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            &self.conn,                    // xcb connection to X11
            xcb::INPUT_FOCUS_PARENT as u8, // focus the parent when focus is lost
            id,                            // window to focus
            xcb::CURRENT_TIME,             // event time (0 == current time)
        );

        self.change_prop(
            self.root(),
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![id]),
        )
    }

    /// Send an event to a client
    pub fn send_client_event(&self, msg: ClientMessage) -> Result<()> {
        let (dtype, d) = (self.atom(&msg.dtype)?, msg.data().as_u32());
        let data = xcb::ClientMessageData::from_data32([d[0], d[1], d[2], d[3], d[4]]);
        let event = xcb::ClientMessageEvent::new(32, msg.id, dtype, data);
        let mask = match msg.mask {
            ClientEventMask::NoEventMask => xcb::EVENT_MASK_NO_EVENT,
            ClientEventMask::SubstructureNotify => xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY,
        };

        Ok(xcb::send_event_checked(&self.conn, false, msg.id, mask, &event).request_check()?)
    }

    /// Build a new known client event
    pub fn build_client_event(
        &self,
        kind: ClientMessageKind,
    ) -> crate::core::xconnection::Result<ClientMessage> {
        kind.as_message(self)
    }

    /// Set attributes on the target client
    pub fn set_client_attributes(&self, id: Xid, attrs: &[ClientAttr]) -> Result<()> {
        let data: Vec<(u32, u32)> = attrs.iter().flat_map::<Vec<_>, _>(|c| c.into()).collect();
        Ok(xcb::change_window_attributes_checked(&self.conn, id, &data).request_check()?)
    }

    /// Find the current size and position of the target window
    pub fn client_geometry(&self, id: Xid) -> Result<Region> {
        let res = xcb::get_geometry(&self.conn, id).get_reply()?;
        Ok(Region::new(
            res.x() as u32,
            res.y() as u32,
            res.width() as u32,
            res.height() as u32,
        ))
    }

    // logic taken from https://github.com/rtbo/rust-xcb/blob/master/examples/randr_crtc_info.rs
    /// Query the randr API for current outputs and return the details as penrose
    /// [Screen] structs.
    pub fn current_screens(&self) -> Result<Vec<Screen>> {
        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let check_win = self.check_window();
        let resources = xcb::randr::get_screen_resources(&self.conn, check_win);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        let screens = resources
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
            .collect();

        self.destroy_client(check_win)?;
        Ok(screens)
    }

    /// Query the randr API for current outputs and return the size of each screen
    pub fn screen_sizes(&self) -> Result<Vec<Region>> {
        self.current_screens()
            .map(|screens| screens.iter().map(|s| s.region(false)).collect())
    }

    /// The list of currently active clients known to the X server
    pub fn current_clients(&self) -> Result<Vec<Xid>> {
        Ok(xcb::query_tree(&self.conn, self.root)
            .get_reply()
            .map(|reply| reply.children().into())?)
    }

    /// The current (x, y) position of the cursor relative to the root window
    pub fn cursor_position(&self) -> Result<Point> {
        Ok(xcb::query_pointer(&self.conn, self.root)
            .get_reply()
            .map(|reply| Point::new(reply.root_x() as u32, reply.root_y() as u32))?)
    }

    /// Flush pending actions to the X event loop
    pub fn flush(&self) -> bool {
        self.conn.flush()
    }

    /// The client that the X server currently considers to be focused
    pub fn focused_client(&self) -> Result<Xid> {
        // xcb docs: https://www.mankier.com/3/xcb_get_input_focus
        Ok(xcb::get_input_focus(&self.conn).get_reply()?.focus())
    }

    /// Register intercepts for each given [KeyCode]
    pub fn grab_keys(&self, keys: &[&KeyCode]) -> Result<()> {
        // We need to explicitly grab NumLock as an additional modifier and then drop it later on
        // when we are passing events through to the WindowManager as NumLock alters the modifier
        // mask when it is active.
        let modifiers = &[0, xcb::MOD_MASK_2 as u16];
        let mode = xcb::GRAB_MODE_ASYNC as u8;

        for m in modifiers.iter() {
            for k in keys.iter() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_key
                xcb::grab_key_checked(
                    &self.conn, // xcb connection to X11
                    false,      // don't pass grabbed events through to the client
                    self.root,  // the window to grab: in this case the root window
                    k.mask | m, // modifiers to grab
                    k.code,     // keycode to grab
                    mode,       // don't lock pointer input while grabbing
                    mode,       // don't lock keyboard input while grabbing
                )
                .request_check()?;
            }
        }

        self.flush();
        Ok(())
    }

    /// Register intercepts for each given [MouseState]
    pub fn grab_mouse_buttons(&self, states: &[&MouseState]) -> Result<()> {
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
                xcb::grab_button_checked(
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
                )
                .request_check()?;
            }
        }

        self.flush();
        Ok(())
    }

    /// The current root window ID
    pub fn root(&self) -> Xid {
        self.root
    }

    /// The Xid being used as a check window
    pub fn check_window(&self) -> Xid {
        let id = self.conn.generate_id();
        xcb::create_window(
            &self.conn,
            0,
            id,
            self.root,
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
        id
    }

    /// Set a pre-defined notify mask for randr events to subscribe to
    pub fn set_randr_notify_mask(&self) -> Result<()> {
        let mask = (xcb::randr::NOTIFY_MASK_OUTPUT_CHANGE
            | xcb::randr::NOTIFY_MASK_CRTC_CHANGE
            | xcb::randr::NOTIFY_MASK_SCREEN_CHANGE) as u16;

        xcb::randr::select_input_checked(&self.conn, self.root, mask).request_check()?;
        self.flush();
        Ok(())
    }

    /// Drop all active intercepts for key combinations
    pub fn ungrab_keys(&self) -> Result<()> {
        Ok(xcb::ungrab_key_checked(
            &self.conn, // xcb connection to X11
            xcb::GRAB_ANY as u8,
            self.root, // the window to ungrab keys for
            xcb::MOD_MASK_ANY as u16,
        )
        .request_check()?)
    }

    /// Drop all active intercepts for mouse states
    pub fn ungrab_mouse_buttons(&self) -> Result<()> {
        Ok(xcb::ungrab_button_checked(
            &self.conn, // xcb connection to X11
            xcb::BUTTON_INDEX_ANY as u8,
            self.root, // the window to ungrab keys for
            xcb::MOD_MASK_ANY as u16,
        )
        .request_check()?)
    }

    /// Block until the next event from the X event loop is ready then return it.
    ///
    /// This method handles all of the mapping of xcb events to penrose [XEvent] instances,
    /// returning an Error when the event channel from the X server is closed.
    pub fn wait_for_event(&self) -> Result<XEvent> {
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

    /// Return the next event from the X event loop if there is one.
    ///
    /// This method handles all of the mapping of xcb events to penrose [XEvent] instances,
    /// returning None if there is no pending event and an error if the connection to the X server
    /// is closed.
    pub fn poll_for_event(&self) -> Result<Option<XEvent>> {
        if let Some(event) = self.conn.poll_for_event() {
            self.generic_xcb_to_xevent(event)
        } else {
            Ok(self.conn.has_error().map(|_| None)?)
        }
    }

    /// Move the cursor to the given (x, y) position inside the specified window.
    pub fn warp_cursor(&self, id: Xid, x: usize, y: usize) -> Result<()> {
        Ok(
            // conn source target source(x y w h) dest(x y)
            xcb::warp_pointer_checked(&self.conn, 0, id, 0, 0, 0, 0, x as i16, y as i16)
                .request_check()?,
        )
    }
}
