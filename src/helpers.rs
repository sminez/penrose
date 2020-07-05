use crate::data_types::{CodeMap, Direction, KeyBindings, KeyCode};
use std::process;
use xcb;

// pulling out bitmasks to make the following xcb / xrandr calls easier to parse visually
const NOTIFY_MASK: u16 = xcb::randr::NOTIFY_MASK_CRTC_CHANGE as u16;
const GRAB_MODE_ASYNC: u8 = xcb::GRAB_MODE_ASYNC as u8;
const EVENT_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
)];
const MOUSE_MASK: u16 = (xcb::EVENT_MASK_BUTTON_PRESS
    | xcb::EVENT_MASK_BUTTON_RELEASE
    | xcb::EVENT_MASK_POINTER_MOTION) as u16;

/// Cycle through a set of indices, wrapping at either end
pub fn cycle_index(ix: usize, max: usize, direction: Direction) -> usize {
    match direction {
        Direction::Forward => return if ix == max { 0 } else { ix + 1 },
        Direction::Backward => return if ix == 0 { max } else { ix - 1 },
    }
}

/**
 * Run the xmodmap command to dump the system keymap table in a form
 * that we can load in and convert back to key codes. This lets the user
 * define key bindings in the way that they would expect while also
 * ensuring that it is east to debug any odd issues with bindings by
 * referring the user to the xmodmap output.
 */
pub fn keycodes_from_xmodmap() -> CodeMap {
    match process::Command::new("xmodmap").arg("-pke").output() {
        Err(e) => die!("unable to fetch keycodes via xmodmap: {}", e),
        Ok(o) => match String::from_utf8(o.stdout) {
            Err(e) => die!("invalid utf8 from xmodmap: {}", e),
            Ok(s) => s
                .lines()
                .flat_map(|l| {
                    let mut words = l.split_whitespace(); // keycode <code> = <names ...>
                    let key_code: u8 = words.nth(1).unwrap().parse().unwrap();
                    words.skip(1).map(move |name| (name.into(), key_code))
                })
                .collect::<CodeMap>(),
        },
    }
}

/**
 * Allow the user to define their keybindings using the gen_keybindings macro
 * which calls through to this. Bindings are of the form '<MOD>-<key name>'
 * with multipple modifiers being allowed, and keynames being taken from the
 * output of 'xmodmap -pke'.
 *
 * Allowed modifiers are:
 *   M - Super
 *   A - Alt
 *   C - Ctrl
 *   S - Shift
 *
 * The user friendly patterns are parsed into a modifier mask and X key code
 * pair that is then grabbed by penrose to trigger the bound action.
 */
pub fn parse_key_binding<S>(pattern: S, known_codes: &CodeMap) -> Option<KeyCode>
where
    S: Into<String>,
{
    let s = pattern.into();
    let mut parts: Vec<&str> = s.split("-").collect();
    match known_codes.get(parts.remove(parts.len() - 1)) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|s| match s {
                    &"A" => xcb::MOD_MASK_1,
                    &"M" => xcb::MOD_MASK_4,
                    &"S" => xcb::MOD_MASK_SHIFT,
                    &"C" => xcb::MOD_MASK_CONTROL,
                    &_ => die!("invalid key binding prefix: {}", s),
                })
                .fold(0, |acc, v| acc | v);

            // log!("binding '{}' as [{}, {}]", s, mask, code);
            Some(KeyCode {
                mask: mask as u16,
                code: *code,
            })
        }
        None => None,
    }
}

/**
 * Notify the X server that we are intercepting the user specified key bindings
 * and prevent them being passed through to the underlying applications. This
 * is what determines which key press events end up being sent through in the
 * main event loop for the WindowManager.
 */
pub fn grab_keys(conn: &xcb::Connection, key_bindings: &KeyBindings) {
    let screen = conn.get_setup().roots().nth(0).unwrap();
    let root = screen.root();

    // xcb docs: https://www.mankier.com/3/xcb_randr_select_input
    let input = xcb::randr::select_input(conn, root, NOTIFY_MASK);
    match input.request_check() {
        Err(e) => die!("randr error: {}", e),
        Ok(_) => {
            for k in key_bindings.keys() {
                // xcb docs: https://www.mankier.com/3/xcb_grab_key
                xcb::grab_key(
                    conn,            // xcb connection to X11
                    false,           // don't pass grabbed events through to the client
                    root,            // the window to grab: in this case the root window
                    k.mask,          // modifiers to grab
                    k.code,          // keycode to grab
                    GRAB_MODE_ASYNC, // don't lock pointer input while grabbing
                    GRAB_MODE_ASYNC, // don't lock keyboard input while grabbing
                );
            }
        }
    }

    for mouse_button in &[1, 3] {
        // xcb docs: https://www.mankier.com/3/xcb_grab_button
        xcb::grab_button(
            conn,                   // xcb connection to X11
            false,                  // don't pass grabbed events through to the client
            root,                   // the window to grab: in this case the root window
            MOUSE_MASK,             // which events are reported to the client
            GRAB_MODE_ASYNC,        // don't lock pointer input while grabbing
            GRAB_MODE_ASYNC,        // don't lock keyboard input while grabbing
            xcb::NONE,              // don't confine the cursor to a specific window
            xcb::NONE,              // don't change the cursor type
            *mouse_button,          // the button to grab
            xcb::MOD_MASK_4 as u16, // modifiers to grab
        );
    }

    // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
    xcb::change_window_attributes(conn, root, EVENT_MASK);
    conn.flush();
}

/**
 * Intern an XCB atom by name, returning the atom ID if we are able
 */
pub fn intern_atom(conn: &xcb::Connection, name: &str) -> u32 {
    // https://www.mankier.com/3/xcb_intern_atom
    let interned_atom = xcb::intern_atom(
        conn,  // xcb connection to X11
        false, // return the atom ID even if it doesn't already exists
        name,  // name of the atom to retrieve
    );

    match interned_atom.get_reply() {
        Err(e) => die!("unable to fetch xcb atom '{}': {}", name, e),
        Ok(reply) => reply.atom(),
    }
}

/**
 * Use the xcb api to query a string property for a window by window ID and poperty name.
 * Can fail if the property name is invalid or we get a malformed response from xcb.
 */
pub fn str_prop(conn: &xcb::Connection, id: u32, name: &str) -> Result<String, String> {
    // xcb docs: https://www.mankier.com/3/xcb_get_property
    let cookie = xcb::get_property(
        conn,                    // xcb connection to X11
        false,                   // should the property be deleted
        id,                      // target window to query
        intern_atom(conn, name), // the property we want
        xcb::ATOM_ANY,           // the type of the property
        0,                       // offset in the property to retrieve data from
        1024,                    // how many 32bit multiples of data to retrieve
    );

    match cookie.get_reply() {
        Err(e) => Err(format!("unable to fetch window property: {}", e)),
        Ok(reply) => match String::from_utf8(reply.value().to_vec()) {
            Err(e) => Err(format!("invalid utf8 resonse from xcb: {}", e)),
            Ok(s) => Ok(s),
        },
    }
}

pub fn atom_prop(conn: &xcb::Connection, id: u32, name: &str) -> Result<u32, String> {
    // xcb docs: https://www.mankier.com/3/xcb_get_property
    let cookie = xcb::get_property(
        conn,                    // xcb connection to X11
        false,                   // should the property be deleted
        id,                      // target window to query
        intern_atom(conn, name), // the property we want
        xcb::ATOM_ANY,           // the type of the property
        0,                       // offset in the property to retrieve data from
        1024,                    // how many 32bit multiples of data to retrieve
    );

    match cookie.get_reply() {
        Err(e) => Err(format!("unable to fetch window property: {}", e)),
        Ok(reply) => {
            if reply.value_len() <= 0 {
                Err(format!("property '{}' was empty for id: {}", name, id))
            } else {
                Ok(reply.value()[0])
            }
        }
    }
}
