//! Setting up and responding to user defined key/mouse bindings
use crate::{
    core::{State, Xid},
    pure::geometry::Point,
    x::XConn,
    Error, Result,
};
#[cfg(feature = "keysyms")]
use penrose_keysyms::XKeySym;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fmt, process::Command};
use strum::{EnumIter, IntoEnumIterator};
use tracing::trace;

/// Run the xmodmap command to dump the system keymap table.
///
/// This is done in a form that we can load in and convert back to key
/// codes. This lets the user define key bindings in the way that they
/// would expect while also ensuring that it is east to debug any odd
/// issues with bindings by referring the user to the xmodmap output.
///
/// # Panics
/// This function will panic if it is unable to fetch keycodes using the xmodmap
/// binary on your system or if the output of `xmodmap -pke` is not valid
pub fn keycodes_from_xmodmap() -> Result<HashMap<String, u8>> {
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

fn parse_binding(pattern: &str, known_codes: &HashMap<String, u8>) -> Result<KeyCode> {
    let mut parts: Vec<&str> = pattern.split('-').collect();
    let name = parts.remove(parts.len() - 1);

    match known_codes.get(name) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|&s| ModifierKey::try_from(s))
                .try_fold(0, |acc, v| v.map(|inner| acc | u16::from(inner)))?;

            trace!(?pattern, mask, code, "parsed keybinding");
            Ok(KeyCode { mask, code: *code })
        }

        None => Err(Error::UnknownKeyName {
            name: name.to_owned(),
        }),
    }
}

/// Parse string format key bindings into [KeyCode] based [KeyBindings] using
/// the command line `xmodmap` utility.
///
/// See [keycodes_from_xmodmap] for details of how `xmodmap` is used.
pub fn parse_keybindings_with_xmodmap<S, X>(
    str_bindings: HashMap<S, Box<dyn KeyEventHandler<X>>>,
) -> Result<KeyBindings<X>>
where
    S: AsRef<str>,
    X: XConn,
{
    let m = keycodes_from_xmodmap()?;

    str_bindings
        .into_iter()
        .map(|(s, v)| parse_binding(s.as_ref(), &m).map(|k| (k, v)))
        .collect()
}

/// Some action to be run by a user key binding
pub trait KeyEventHandler<X>
where
    X: XConn,
{
    /// Call this handler with the current window manager state
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()>;
}

impl<X: XConn> fmt::Debug for Box<dyn KeyEventHandler<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyEventHandler").finish()
    }
}

impl<F, X> KeyEventHandler<X> for F
where
    F: FnMut(&mut State<X>, &X) -> Result<()>,
    X: XConn,
{
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        (self)(state, x)
    }
}

/// User defined key bindings
pub type KeyBindings<X> = HashMap<KeyCode, Box<dyn KeyEventHandler<X>>>;

/// An action to be run in response to a mouse event
pub trait MouseEventHandler<X>
where
    X: XConn,
{
    /// Call this handler with the current window manager state and mouse state
    fn call(&mut self, evt: &MouseEvent, state: &mut State<X>, x: &X) -> Result<()>;
}

impl<X: XConn> fmt::Debug for Box<dyn MouseEventHandler<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyEventHandler").finish()
    }
}

impl<F, X> MouseEventHandler<X> for F
where
    F: FnMut(&MouseEvent, &mut State<X>, &X) -> Result<()>,
    X: XConn,
{
    fn call(&mut self, evt: &MouseEvent, state: &mut State<X>, x: &X) -> Result<()> {
        (self)(evt, state, x)
    }
}

/// User defined mouse bindings
pub type MouseBindings<X> = HashMap<(MouseEventKind, MouseState), Box<dyn MouseEventHandler<X>>>;

/// Abstraction layer for working with key presses
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPress {
    /// A raw character key
    Utf8(String),
    /// Return / enter key
    Return,
    /// Escape
    Escape,
    /// Tab
    Tab,
    /// Backspace
    Backspace,
    /// Delete
    Delete,
    /// PageUp
    PageUp,
    /// PageDown
    PageDown,
    /// Up
    Up,
    /// Down
    Down,
    /// Left
    Left,
    /// Right
    Right,
}

#[cfg(feature = "keysyms")]
impl TryFrom<XKeySym> for KeyPress {
    type Error = std::string::FromUtf8Error;

    fn try_from(s: XKeySym) -> std::result::Result<KeyPress, Self::Error> {
        Ok(match s {
            XKeySym::XK_Return | XKeySym::XK_KP_Enter | XKeySym::XK_ISO_Enter => KeyPress::Return,
            XKeySym::XK_Escape => KeyPress::Escape,
            XKeySym::XK_Tab | XKeySym::XK_ISO_Left_Tab | XKeySym::XK_KP_Tab => KeyPress::Tab,
            XKeySym::XK_BackSpace => KeyPress::Backspace,
            XKeySym::XK_Delete | XKeySym::XK_KP_Delete => KeyPress::Delete,
            XKeySym::XK_Page_Up | XKeySym::XK_KP_Page_Up => KeyPress::PageUp,
            XKeySym::XK_Page_Down | XKeySym::XK_KP_Page_Down => KeyPress::PageDown,
            XKeySym::XK_Up | XKeySym::XK_KP_Up => KeyPress::Up,
            XKeySym::XK_Down | XKeySym::XK_KP_Down => KeyPress::Down,
            XKeySym::XK_Left | XKeySym::XK_KP_Left => KeyPress::Left,
            XKeySym::XK_Right | XKeySym::XK_KP_Right => KeyPress::Right,
            s => KeyPress::Utf8(s.as_utf8_string()?),
        })
    }
}

/// A u16 X key-code bitmask
pub type KeyCodeMask = u16;

/// A u8 X key-code enum value
pub type KeyCodeValue = u8;

/// A key press and held modifiers
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KeyCode {
    /// The held modifier mask
    pub mask: KeyCodeMask,
    /// The key code that was held
    pub code: KeyCodeValue,
}

impl KeyCode {
    /// Create a new [KeyCode] from this one that removes the given mask
    pub fn ignoring_modifier(&self, mask: KeyCodeMask) -> KeyCode {
        KeyCode {
            mask: self.mask & !mask,
            code: self.code,
        }
    }
}

/// Known mouse buttons for binding actions
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MouseButton {
    /// 1
    Left,
    /// 2
    Middle,
    /// 3
    Right,
    /// 4
    ScrollUp,
    /// 5
    ScrollDown,
}

impl From<MouseButton> for u8 {
    fn from(b: MouseButton) -> u8 {
        match b {
            MouseButton::Left => 1,
            MouseButton::Middle => 2,
            MouseButton::Right => 3,
            MouseButton::ScrollUp => 4,
            MouseButton::ScrollDown => 5,
        }
    }
}

impl TryFrom<u8> for MouseButton {
    type Error = Error;

    fn try_from(n: u8) -> Result<Self> {
        match n {
            1 => Ok(Self::Left),
            2 => Ok(Self::Middle),
            3 => Ok(Self::Right),
            4 => Ok(Self::ScrollUp),
            5 => Ok(Self::ScrollDown),
            _ => Err(Error::UnknownMouseButton { button: n }),
        }
    }
}

/// Known modifier keys for bindings
#[derive(Debug, EnumIter, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ModifierKey {
    /// Control
    Ctrl,
    /// Alt
    Alt,
    /// Shift
    Shift,
    /// Meta / super / windows
    Meta,
}

impl ModifierKey {
    fn was_held(&self, mask: u16) -> bool {
        mask & u16::from(*self) > 0
    }
}

impl From<ModifierKey> for u16 {
    fn from(m: ModifierKey) -> u16 {
        (match m {
            ModifierKey::Shift => 1 << 0,
            ModifierKey::Ctrl => 1 << 2,
            ModifierKey::Alt => 1 << 3,
            ModifierKey::Meta => 1 << 6,
        }) as u16
    }
}

impl TryFrom<&str> for ModifierKey {
    type Error = Error;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        match s {
            "C" => Ok(Self::Ctrl),
            "A" => Ok(Self::Alt),
            "S" => Ok(Self::Shift),
            "M" => Ok(Self::Meta),
            _ => Err(Error::UnknownModifier { name: s.to_owned() }),
        }
    }
}

/// A mouse state specification indicating the button and modifiers held
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseState {
    /// The [MouseButton] being held
    pub button: MouseButton,
    /// All [ModifierKey]s being held
    pub modifiers: Vec<ModifierKey>,
}

impl MouseState {
    /// Construct a new MouseState
    pub fn new(button: MouseButton, mut modifiers: Vec<ModifierKey>) -> Self {
        modifiers.sort();
        Self { button, modifiers }
    }

    /// Parse raw mouse state values into a [MouseState]
    pub fn from_detail_and_state(detail: u8, state: u16) -> Result<Self> {
        Ok(Self {
            button: MouseButton::try_from(detail)?,
            modifiers: ModifierKey::iter().filter(|m| m.was_held(state)).collect(),
        })
    }

    /// The xcb bitmask for this [MouseState]
    pub fn mask(&self) -> u16 {
        self.modifiers
            .iter()
            .fold(0, |acc, &val| acc | u16::from(val))
    }

    /// The xcb button ID for this [MouseState]
    pub fn button(&self) -> u8 {
        self.button.into()
    }
}

/// The types of mouse events represented by a MouseEvent
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MouseEventKind {
    /// A button was pressed
    Press,
    /// A button was released
    Release,
    /// The mouse was moved while a button was held
    Motion,
}

/// A mouse movement or button event
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseEvent {
    /// The ID of the window that was contained the click
    pub id: Xid,
    /// Absolute coordinate of the event
    pub rpt: Point,
    /// Coordinate of the event relative to top-left of the window itself
    pub wpt: Point,
    /// The modifier and button code that was received
    pub state: MouseState,
    /// Was this press, release or motion?
    pub kind: MouseEventKind,
}

impl MouseEvent {
    /// Construct a new [MouseEvent] from raw data
    pub fn new(
        id: Xid,
        rx: i16,
        ry: i16,
        ex: i16,
        ey: i16,
        state: MouseState,
        kind: MouseEventKind,
    ) -> Self {
        MouseEvent {
            id,
            rpt: Point::new(rx as u32, ry as u32),
            wpt: Point::new(ex as u32, ey as u32),
            state,
            kind,
        }
    }
}
