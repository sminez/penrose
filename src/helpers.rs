use crate::data_types::{CodeMap, KeyCode};
use std::process;
use xcb;

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
            Some(KeyCode {
                mask: mask as u16,
                code: *code,
            })
        }
        None => None,
    }
}

/**
 * Use the xcb api to query a string property for a window by window ID and poperty name.
 * Can fail if the property name is invalid or we get a malformed response from xcb.
 */
pub fn str_prop(conn: &xcb::Connection, id: u32, prop: &str) -> Result<String, String> {
    match xcb::intern_atom(conn, false, prop).get_reply() {
        Err(e) => Err(format!("unable to fetch xcb atom: {}", e)),
        Ok(reply) => {
            // TODO: add commment with what this xcb call is doing
            let cookie = xcb::get_property(conn, false, id, reply.atom(), xcb::ATOM_ANY, 0, 1024);
            match cookie.get_reply() {
                Err(e) => Err(format!("unable to fetch window property: {}", e)),
                Ok(reply) => match String::from_utf8(reply.value().to_vec()) {
                    Ok(s) => Ok(s),
                    Err(e) => Err(format!("invalid utf8 resonse from xcb: {}", e)),
                },
            }
        }
    }
}
