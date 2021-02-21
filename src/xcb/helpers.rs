//! XCB based helper functions
use crate::core::bindings::{CodeMap, KeyCode};

/**
 * Convert user friendly key bindings into X keycodes.
 *
 * Allows the user to define their keybindings using the gen_keybindings macro
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
pub fn parse_key_binding(pattern: String, known_codes: &CodeMap) -> Option<KeyCode> {
    let mut parts: Vec<&str> = pattern.split('-').collect();
    match known_codes.get(parts.remove(parts.len() - 1)) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|&s| match s {
                    "A" => xcb::MOD_MASK_1,
                    "M" => xcb::MOD_MASK_4,
                    "S" => xcb::MOD_MASK_SHIFT,
                    "C" => xcb::MOD_MASK_CONTROL,
                    _ => panic!("invalid key binding prefix: {}", s),
                })
                .fold(0, |acc, v| acc | v);

            trace!(?pattern, mask, code, "parsed keybinding");
            Some(KeyCode {
                mask: mask as u16,
                code: *code,
            })
        }
        None => None,
    }
}
