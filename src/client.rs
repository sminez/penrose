use crate::data_types::{Border, ColorScheme, WinId};
use crate::helpers::intern_atom;
use xcb;

const INPUT_FOCUS_PARENT: u8 = xcb::INPUT_FOCUS_PARENT as u8;
const PROP_MODE_REPLACE: u8 = xcb::PROP_MODE_REPLACE as u8;
const ATOM_WINDOW: u32 = xcb::xproto::ATOM_WINDOW;

/**
 * Meta-data around a client window that we are handling.
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone)]
pub struct Client {
    pub id: WinId,
    wm_class: String,
    border_width: u32,
    // state flags
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
}

impl Client {
    pub fn new(id: WinId, wm_class: String, floating: bool, border_width: u32) -> Client {
        Client {
            id,
            wm_class,
            border_width,
            is_focused: false,
            is_floating: floating,
            is_fullscreen: false,
        }
    }

    pub fn focus(&mut self, conn: &xcb::Connection, scheme: &ColorScheme) {
        self.set_window_border(conn, Border::Focused, scheme);
        self.is_focused = true;

        let root = match conn.get_setup().roots().nth(0) {
            None => die!("unable to get handle for screen"),
            Some(screen) => screen.root(),
        };
        let prop = intern_atom(conn, "_NET_ACTIVE_WINDOW");

        // xcb docs: https://www.mankier.com/3/xcb_set_input_focus
        xcb::set_input_focus(
            conn,               // xcb connection to X11
            INPUT_FOCUS_PARENT, // focus the parent when focus is lost
            self.id,            // window to focus
            0,                  // current time to avoid network race conditions (0 == current time)
        );

        // xcb docs: https://www.mankier.com/3/xcb_change_property
        xcb::change_property(
            conn,              // xcb connection to X11
            PROP_MODE_REPLACE, // discard current prop and replace
            root,              // window to change prop on
            prop,              // prop to change
            ATOM_WINDOW,       // type of prop
            32,                // data format (8/16/32-bit)
            &[self.id],        // data
        );
    }

    pub fn unfocus(&mut self, conn: &xcb::Connection, scheme: &ColorScheme) {
        self.set_window_border(conn, Border::Unfocused, scheme);
        self.is_focused = false;
    }

    fn set_window_border(&mut self, conn: &xcb::Connection, border: Border, scheme: &ColorScheme) {
        let color = match border {
            Border::Urgent => scheme.urgent,
            Border::Focused => scheme.highlight,
            Border::Unfocused => scheme.fg_1,
        };
        xcb::change_window_attributes(conn, self.id, &[(xcb::CW_BORDER_PIXEL, color)]);
    }
}
