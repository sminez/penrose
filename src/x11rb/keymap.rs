//! The server's mapping between keycodes and keysyms.
//!
//! Bindings name keysyms, which say what a key means, while the X server grabs and reports
//! keycodes, which say where a key is. This is the table that relates the two, and both
//! directions of lookup are needed: which keys to grab for a binding, and which binding a
//! press belongs to.
use crate::{Result, core::bindings::KeySym};
use x11rb::{connection::Connection, protocol::xproto::ConnectionExt as _};

/// The keysyms bound to each keycode, as reported by `GetKeyboardMapping`.
///
/// A keycode has several keysyms, one per level: the first is the key on its own, the
/// second is the key with shift, and further levels come from the layout's groups. So on a
/// US layout the `;` key holds `semicolon` and `colon`, and either name refers to it.
#[derive(Debug, Default)]
pub(crate) struct Keymap {
    min_keycode: u8,
    per_keycode: usize,
    keysyms: Vec<u32>,
}

impl Keymap {
    /// Read the current mapping from the server.
    pub(crate) fn fetch(conn: &impl Connection) -> Result<Self> {
        let setup = conn.setup();
        let (min_keycode, max_keycode) = (setup.min_keycode, setup.max_keycode);
        let count = max_keycode - min_keycode + 1;

        let reply = conn.get_keyboard_mapping(min_keycode, count)?.reply()?;

        Ok(Self {
            min_keycode,
            per_keycode: reply.keysyms_per_keycode as usize,
            keysyms: reply.keysyms,
        })
    }

    /// Every keysym `code` can produce, one per level, in level order.
    ///
    /// Trailing `NoSymbol` entries are dropped: the reply is a rectangular table, so a key
    /// with fewer levels than the widest one is padded with them.
    pub(crate) fn syms_for(&self, code: u8) -> &[u32] {
        if self.per_keycode == 0 || code < self.min_keycode {
            return &[];
        }

        let start = (code - self.min_keycode) as usize * self.per_keycode;
        let Some(syms) = self.keysyms.get(start..start + self.per_keycode) else {
            return &[];
        };

        let end = syms
            .iter()
            .rposition(|&s| s != NO_SYMBOL)
            .map_or(0, |i| i + 1);

        &syms[..end]
    }

    /// Every keycode which can produce `keysym`, at any level.
    ///
    /// This is a list rather than a single keycode because a keysym can appear on more than
    /// one key - a layout may repeat it, or it may sit at a different level of another key -
    /// and a binding on it should fire from any of them.
    pub(crate) fn keycodes_for(&self, keysym: u32) -> Vec<u8> {
        if keysym == NO_SYMBOL {
            return vec![];
        }

        self.keycodes()
            .filter(|&code| self.syms_for(code).contains(&keysym))
            .collect()
    }

    /// Which of `bound` a press of `code` refers to, if any.
    ///
    /// A key can be named by any of its levels, so `"S-semicolon"` and `"S-colon"` are both
    /// ways of writing shift plus the `;` key and both need to work. The levels are checked
    /// in order and the first one that is actually bound wins, so a binding is only matched
    /// by a name the user wrote.
    pub(crate) fn bound_sym(&self, code: u8, mask: u16, bound: &[KeySym]) -> Option<KeySym> {
        self.syms_for(code)
            .iter()
            .map(|&keysym| KeySym { mask, keysym })
            .find(|k| bound.contains(k))
    }

    /// The keysym a press of `code` produces on its own, ignoring modifiers.
    ///
    /// Used for a key which is not bound at all, which happens while the keyboard is
    /// captured for a key sequence.
    pub(crate) fn unmodified_sym(&self, code: u8, mask: u16) -> Option<KeySym> {
        self.syms_for(code)
            .first()
            .map(|&keysym| KeySym { mask, keysym })
    }

    fn keycodes(&self) -> impl Iterator<Item = u8> + '_ {
        let count = self
            .keysyms
            .len()
            .checked_div(self.per_keycode)
            .unwrap_or(0);

        (0..count).filter_map(move |i| u8::try_from(self.min_keycode as usize + i).ok())
    }
}

/// `XCB_NO_SYMBOL`: the entry for a level a key does not have.
const NO_SYMBOL: u32 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Three keys from a US layout: `a`/`A`, `;`/`:`, and Return, which has one level. The
    /// table is rectangular, so Return is padded with NoSymbol.
    fn keymap() -> Keymap {
        Keymap {
            min_keycode: 8,
            per_keycode: 2,
            keysyms: vec![
                0x61, 0x41, // keycode 8:  a, A
                0x3b, 0x3a, // keycode 9:  semicolon, colon
                0xff0d, NO_SYMBOL, // keycode 10: Return
            ],
        }
    }

    #[test]
    fn syms_are_returned_in_level_order_without_padding() {
        assert_eq!(keymap().syms_for(9), &[0x3b, 0x3a]);
        assert_eq!(keymap().syms_for(10), &[0xff0d]);
    }

    #[test]
    fn an_unmapped_keycode_has_no_syms() {
        // Out of range in either direction, since keycodes start at min_keycode rather than
        // at zero.
        assert!(keymap().syms_for(0).is_empty());
        assert!(keymap().syms_for(200).is_empty());
    }

    #[test]
    fn a_keysym_at_any_level_names_its_key() {
        // colon is only reachable with shift, and naming it still has to find the key it is
        // on, since that is the key which needs grabbing.
        assert_eq!(keymap().keycodes_for(0x3b), vec![9]);
        assert_eq!(keymap().keycodes_for(0x3a), vec![9]);
    }

    #[test]
    fn every_key_carrying_a_keysym_is_returned() {
        let map = Keymap {
            min_keycode: 8,
            per_keycode: 2,
            keysyms: vec![0xff0d, NO_SYMBOL, 0x61, 0x41, 0xff0d, NO_SYMBOL],
        };

        // Return on two keys, as with the main and keypad enter: a binding on it should
        // fire from either, which is what a single keycode lookup got wrong.
        assert_eq!(map.keycodes_for(0xff0d), vec![8, 10]);
    }

    #[test]
    fn no_symbol_never_matches() {
        // Padding is not a key, so a caller asking for NoSymbol would otherwise grab every
        // key that has fewer levels than the widest one.
        assert!(keymap().keycodes_for(NO_SYMBOL).is_empty());
    }

    #[test]
    fn a_press_resolves_to_the_bound_level() {
        let map = keymap();
        let shift = 1;
        let semicolon = KeySym {
            mask: shift,
            keysym: 0x3b,
        };
        let colon = KeySym {
            mask: shift,
            keysym: 0x3a,
        };

        // The same press, resolved against different configs: whichever spelling the user
        // bound is the one they get back.
        assert_eq!(map.bound_sym(9, shift, &[semicolon]), Some(semicolon));
        assert_eq!(map.bound_sym(9, shift, &[colon]), Some(colon));

        // Bound with the wrong modifiers is not bound.
        assert_eq!(map.bound_sym(9, 0, &[semicolon]), None);
    }

    #[test]
    fn an_unbound_press_still_resolves_to_its_first_level() {
        // Nothing is bound, which is the case while capturing the rest of a key sequence:
        // the press still has to arrive so that it can end the sequence.
        assert_eq!(
            keymap().unmodified_sym(9, 0),
            Some(KeySym {
                mask: 0,
                keysym: 0x3b
            })
        );
    }
}
