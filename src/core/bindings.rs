//! Setting up and responding to user defined key/mouse bindings
use crate::{
    Error, Result,
    core::{
        State,
        conn::{Conn, WinId},
    },
    pure::geometry::Point,
};
use penrose_keysyms::XKeySym;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt, mem,
    str::FromStr,
};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{debug, error, trace};

/// Why a key binding could not be used.
#[derive(Debug)]
pub struct KeyBindingError {
    /// The binding as it was written, e.g. `"M-S-Retrun"`.
    pub binding: String,
    /// Why it could not be used.
    pub error: Error,
}

impl fmt::Display for KeyBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}': {}", self.binding, self.error)
    }
}

/// The keybindings that parsed, along with any errors.
#[derive(Debug)]
pub struct ParsedKeyBindings<C: Conn> {
    /// The bindings.
    pub bindings: KeyBindings<C>,
    /// Errors that occurred while parsing the bindings.
    pub errors: Vec<KeyBindingError>,
}

impl<C: Conn> ParsedKeyBindings<C> {
    /// Returns the bindings or an error naming every binding that was dropped.
    pub fn into_result(self) -> Result<KeyBindings<C>> {
        if self.errors.is_empty() {
            return Ok(self.bindings);
        }

        Err(Error::InvalidKeyBindings {
            errors: self.errors,
        })
    }

    /// Returns the bindings, and logs all dropped bindings.
    pub fn log_err(self) -> KeyBindings<C> {
        for e in self.errors.iter() {
            error!(binding = %e.binding, error = %e.error, "key binding error");
        }

        self.bindings
    }
}

/// Dispatches a key press. If it matches a binding, the action is run. If it matches a key
/// sequence, waits for more keys to complete. [Conn] implementations should call this for every key
/// press they receive rather than looking bindings up themselves.
pub fn dispatch_key<C: Conn>(
    key: C::KeyBindingKey,
    bindings: &mut KeyBindings<C>,
    state: &mut State<C>,
    conn: &mut C,
) -> Result<()> {
    state.pending_keys.push(key);

    if bindings.is_prefix(&state.pending_keys) {
        let continuations = bindings.continuations(&state.pending_keys);
        trace!(pending = ?state.pending_keys, ?continuations, "waiting for the rest of a key sequence");
        return conn.capture_next_key(&continuations);
    }

    let keys = mem::take(&mut state.pending_keys);
    if keys.len() > 1 {
        conn.cancel_capture_next_key()?;
    }

    match bindings.get_mut(&keys) {
        Some(handler) => {
            trace!(?keys, "running user keybinding");
            if let Err(error) = handler.call(state, conn) {
                error!(%error, ?keys, "error running user keybinding");
                return Err(error);
            }
        }

        None if keys.len() > 1 => debug!(?keys, "no binding for this key sequence"),
        None => (),
    }

    Ok(())
}

/// Parse string format key bindings into [KeySym] based [KeyBindings], keeping the bindings that
/// parsed alongside the errors for those that did not.
///
/// A binding pattern is modifiers and a key name joined with `-`, e.g. `"M-S-semicolon"`; see
/// [KeySym::parse] for the details of one pattern. A pattern containing whitespace is a *sequence*:
/// `"M-m M-l"` runs when `M-l` is pressed after `M-m`, and neither key does anything on its own.
/// Sequences may be any length, and a sequence bound alongside a shorter binding it starts with is
/// ambiguous, so both are dropped and reported.
///
/// Parsing does not need the keymap, so this works without a running window manager or a display:
/// use it in a test to check that a config's bindings are all spelled correctly.
pub fn parse_keybindings<C>(
    str_bindings: HashMap<String, Box<dyn KeyEventHandler<C>>>,
) -> ParsedKeyBindings<C>
where
    C: Conn<KeyBindingKey = KeySym>,
{
    KeyBindings::parse(str_bindings, KeySym::parse)
}

/// Parse string format key bindings into [KeySym] based [KeyBindings]. Returns an [Error] if any
/// fail to parse. See [parse_keybindings] for more details.
#[deprecated(
    since = "0.4.1",
    note = "bindings are parsed to keysyms without running xmodmap: use parse_keybindings"
)]
pub fn parse_keybindings_with_xmodmap<C>(
    str_bindings: HashMap<String, Box<dyn KeyEventHandler<C>>>,
) -> Result<KeyBindings<C>>
where
    C: Conn<KeyBindingKey = KeySym>,
{
    parse_keybindings(str_bindings).into_result()
}

/// Some action to be run by a user key binding
pub trait KeyEventHandler<C>: Send
where
    C: Conn,
{
    /// Call this handler with the current window manager state
    fn call(&mut self, state: &mut State<C>, conn: &mut C) -> Result<()>;
}

impl<C: Conn> fmt::Debug for Box<dyn KeyEventHandler<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyEventHandler").finish()
    }
}

impl<F, C> KeyEventHandler<C> for F
where
    F: FnMut(&mut State<C>, &mut C) -> Result<()> + Send,
    C: Conn,
{
    fn call(&mut self, state: &mut State<C>, conn: &mut C) -> Result<()> {
        (self)(state, conn)
    }
}

/// User defined key bindings, keyed by keypress sequence.
pub struct KeyBindings<C: Conn> {
    bindings: HashMap<Vec<C::KeyBindingKey>, Box<dyn KeyEventHandler<C>>>,
    /// Every sequence which begins a binding without being one itself.
    prefixes: HashSet<Vec<C::KeyBindingKey>>,
}

impl<C: Conn> fmt::Debug for KeyBindings<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyBindings")
            .field("bindings", &self.bindings)
            .field("prefixes", &self.prefixes)
            .finish()
    }
}

impl<C: Conn> KeyBindings<C> {
    /// Parse string format key bindings using the given parse function, collecting any failures.
    /// Bindings which overlap are all dropped and reported as errors.
    ///
    /// A binding pattern containing whitespace is a *sequence*: `"M-m M-l"` matches when `M-l` is
    /// pressed after `M-m`, and neither key does anything on its own.
    pub fn parse<F>(
        input_bindings: HashMap<String, Box<dyn KeyEventHandler<C>>>,
        mut parse_key: F,
    ) -> ParsedKeyBindings<C>
    where
        F: FnMut(&str) -> Result<C::KeyBindingKey>,
    {
        type BindingsWithString<C> =
            HashMap<Vec<<C as Conn>::KeyBindingKey>, (String, Box<dyn KeyEventHandler<C>>)>;

        let mut errors = Vec::new();
        let mut bindings = BindingsWithString::<C>::new();
        let mut duplicates: HashSet<Vec<C::KeyBindingKey>> = HashSet::new();

        for (binding, handler) in input_bindings {
            let keys: Result<Vec<C::KeyBindingKey>> =
                binding.split_whitespace().map(&mut parse_key).collect();

            match keys {
                Err(error) => errors.push(KeyBindingError { binding, error }),
                Ok(keys) if keys.is_empty() => errors.push(KeyBindingError {
                    error: Error::EmptyKeyBinding,
                    binding,
                }),
                Ok(keys) => {
                    if duplicates.contains(&keys) {
                        errors.push(KeyBindingError {
                            error: Error::DuplicateKeyBinding,
                            binding,
                        });
                    } else if let Some((previous, _)) = bindings.remove(&keys) {
                        errors.push(KeyBindingError {
                            error: Error::DuplicateKeyBinding,
                            binding,
                        });
                        errors.push(KeyBindingError {
                            error: Error::DuplicateKeyBinding,
                            binding: previous,
                        });
                        duplicates.insert(keys);
                    } else {
                        bindings.insert(keys, (binding, handler));
                    }
                }
            }
        }

        let overlapped_prefixes: Vec<Vec<C::KeyBindingKey>> = bindings
            .keys()
            .flat_map(|keys| (1..keys.len()).map(|n| keys[..n].to_vec()))
            .filter(|prefix| bindings.contains_key(prefix))
            .collect();

        for prefix in overlapped_prefixes {
            for (_, (binding, _)) in bindings.extract_if(|keys, _| keys.starts_with(&prefix)) {
                errors.push(KeyBindingError {
                    error: Error::KeyBindingPrefixOverlap,
                    binding,
                });
            }
        }

        let prefixes = bindings
            .keys()
            .flat_map(|keys| (1..keys.len()).map(|n| keys[..n].to_vec()))
            .collect();

        let bindings = bindings.into_iter().map(|(k, (_, h))| (k, h)).collect();

        errors.sort_by(|a, b| a.binding.cmp(&b.binding));

        ParsedKeyBindings {
            bindings: KeyBindings { bindings, prefixes },
            errors,
        }
    }

    /// The number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// The key sequence which runs each binding.
    #[cfg(test)]
    pub(crate) fn sequences(&self) -> impl Iterator<Item = &[C::KeyBindingKey]> {
        self.bindings.keys().map(Vec::as_slice)
    }

    /// The keys which can begin a binding, and so are the ones that need grabbing. The rest
    /// of a sequence arrives through [Conn::capture_next_key] rather than through a grab.
    pub fn leading_keys(&self) -> Vec<C::KeyBindingKey> {
        self.bindings
            .keys()
            .filter_map(|keys| keys.first().copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// The keys which would extend `prefix` into a binding.
    ///
    /// This is what a backend needs in order to listen for the rest of a sequence, so it is
    /// only ever asked for a prefix of some binding, and so is never empty.
    fn continuations(&self, prefix: &[C::KeyBindingKey]) -> Vec<C::KeyBindingKey> {
        self.bindings
            .keys()
            .filter(|keys| keys.len() > prefix.len() && keys.starts_with(prefix))
            .map(|keys| keys[prefix.len()])
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Whether more keys are needed before this sequence can run anything.
    fn is_prefix(&self, keys: &[C::KeyBindingKey]) -> bool {
        // Checked first so that a config with no sequences in it does no work at all here.
        !self.prefixes.is_empty() && self.prefixes.contains(keys)
    }

    fn get_mut(&mut self, keys: &[C::KeyBindingKey]) -> Option<&mut Box<dyn KeyEventHandler<C>>> {
        self.bindings.get_mut(keys)
    }
}

/// An action to be run in response to a mouse event
pub trait MouseEventHandler<C>: Send
where
    C: Conn,
{
    /// Called when the [MouseState] associated with this handler is seen with a button press or
    /// release.
    fn on_mouse_event(&mut self, evt: &MouseEvent, state: &mut State<C>, x: &mut C) -> Result<()>;

    /// Called when the [ModifierKey]s associated with this handler are seen when the mouse is
    /// moving.
    fn on_motion(
        &mut self,
        evt: &MotionNotifyEvent,
        state: &mut State<C>,
        conn: &mut C,
    ) -> Result<()>;
}

impl<C: Conn> fmt::Debug for Box<dyn MouseEventHandler<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MouseEventHandler").finish()
    }
}

impl<F, C> MouseEventHandler<C> for F
where
    F: FnMut(&mut State<C>, &C) -> Result<()> + Send,
    C: Conn,
{
    fn on_mouse_event(
        &mut self,
        evt: &MouseEvent,
        state: &mut State<C>,
        conn: &mut C,
    ) -> Result<()> {
        if evt.kind == MouseEventKind::Press {
            (self)(state, conn)
        } else {
            Ok(())
        }
    }

    fn on_motion(&mut self, _: &MotionNotifyEvent, _: &mut State<C>, _: &mut C) -> Result<()> {
        Ok(())
    }
}

/// Convert a [KeyEventHandler] to a [MouseEventHandler] that runs on button `Press` events.
///
/// This allows for running pre-existing simple key handlers that do not care about press/release
/// or motion behaviour as a simplified mouse event handler.
///
/// ## Example
/// ```rust
/// use penrose::builtin::actions::floating::sink_all;
/// use penrose::core::bindings::{click_handler, MouseEventHandler};
/// use penrose::x11rb::RustConn;
///
/// let handler: Box<dyn MouseEventHandler<RustConn>> =  click_handler(sink_all());
/// ```
pub fn click_handler<C: Conn + 'static>(
    kh: Box<dyn KeyEventHandler<C>>,
) -> Box<dyn MouseEventHandler<C>> {
    Box::new(MouseWrapper { inner: kh })
}

struct MouseWrapper<C: Conn> {
    inner: Box<dyn KeyEventHandler<C>>,
}

impl<C: Conn> MouseEventHandler<C> for MouseWrapper<C> {
    fn on_mouse_event(
        &mut self,
        evt: &MouseEvent,
        state: &mut State<C>,
        conn: &mut C,
    ) -> Result<()> {
        if evt.kind == MouseEventKind::Press {
            self.inner.call(state, conn)
        } else {
            Ok(())
        }
    }

    fn on_motion(&mut self, _: &MotionNotifyEvent, _: &mut State<C>, _: &mut C) -> Result<()> {
        Ok(())
    }
}

/// User defined mouse bindings
pub type MouseBindings<C> = HashMap<MouseState, Box<dyn MouseEventHandler<C>>>;

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

/// A keysym and the modifiers held with it: what a key binding is keyed on.
///
/// A keysym identifies what a key *means* rather than where it sits on the keyboard, so
/// unlike a [KeyCode] this is the same value on every machine regardless of layout. Which
/// physical keys can produce it is a question for the backend, answered when grabbing.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KeySym {
    /// The held modifier mask
    pub mask: KeyCodeMask,
    /// The X11 keysym value
    pub keysym: u32,
}

impl KeySym {
    /// Parse a binding pattern such as `"M-S-semicolon"`.
    ///
    /// Modifiers are `M` (meta / super), `A` (alt), `C` (control) and `S` (shift), joined to
    /// the key name with `-`. The key name is a keysym name with the `XK_` / `XF86XK_`
    /// prefix stripped, which is how `xmodmap -pke` prints them: `a`, `semicolon`,
    /// `Return`, `XF86AudioMute`.
    ///
    /// Naming a key that this keyboard cannot produce is not an error here - whether a
    /// key exists is a property of the keymap rather than of the binding - so that is
    /// reported when the binding is grabbed.
    ///
    /// ```rust
    /// # use penrose::core::bindings::KeySym;
    /// let k = KeySym::parse("M-S-semicolon").unwrap();
    /// assert_eq!(k.keysym, 0x3b);
    ///
    /// assert!(KeySym::parse("M-not-a-key").is_err());
    /// ```
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut parts: Vec<&str> = pattern.split('-').collect();
        let name = parts.remove(parts.len() - 1);

        let keysym = XKeySym::from_str(name)
            .map_err(|_| Error::UnknownKeyName {
                name: name.to_owned(),
            })?
            .keysym();

        let mask = parts
            .iter()
            .map(|&s| ModifierKey::try_from(s))
            .try_fold(0, |acc, v| v.map(|inner| acc | u16::from(inner)))?;

        trace!(?pattern, mask, keysym, "parsed keybinding");

        Ok(KeySym { mask, keysym })
    }
}

/// Known mouse buttons for binding actions
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MouseButton {
    /// 1
    #[default]
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
}

/// Data from a button press or motion-notify event
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseEventData {
    /// The ID of the window that was contained the click
    pub id: WinId,
    /// Absolute coordinate of the event
    pub rpt: Point,
    /// Coordinate of the event relative to top-left of the window itself
    pub wpt: Point,
}

/// A mouse movement or button event
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseEvent {
    /// The details of which window the event applies to and where the event occurred
    pub data: MouseEventData,
    /// The modifier and button code that was received
    pub state: MouseState,
    /// Was this press or release
    pub kind: MouseEventKind,
}

impl MouseEvent {
    /// Construct a new [MouseEvent] from raw data
    pub fn new(
        id: WinId,
        rx: i16,
        ry: i16,
        ex: i16,
        ey: i16,
        state: MouseState,
        kind: MouseEventKind,
    ) -> Self {
        MouseEvent {
            data: MouseEventData {
                id,
                rpt: Point::new(rx as i32, ry as i32),
                wpt: Point::new(ex as i32, ey as i32),
            },
            state,
            kind,
        }
    }
}

/// Mouse motion with a held button and optional modifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MotionNotifyEvent {
    /// The details of which window the event applies to and where the event occurred
    pub data: MouseEventData,
    /// All [ModifierKey]s being held
    pub modifiers: Vec<ModifierKey>,
}

impl MotionNotifyEvent {
    /// Construct a new [MotionNotifyEvent] from raw data
    pub fn new(id: WinId, rx: i16, ry: i16, ex: i16, ey: i16, modifiers: Vec<ModifierKey>) -> Self {
        MotionNotifyEvent {
            data: MouseEventData {
                id,
                rpt: Point::new(rx as i32, ry as i32),
                wpt: Point::new(ex as i32, ey as i32),
            },
            modifiers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pure::geometry::Rect, x::mock::MockXConn};
    use std::sync::mpsc::{Receiver, Sender, channel};

    #[derive(Debug, Default)]
    struct TestConn {
        captures: Vec<&'static str>,
        /// The keys each capture was told to expect, as the letters they parsed from.
        continuations: Vec<Vec<u32>>,
    }

    impl MockXConn for TestConn {
        fn mock_screen_details(&mut self) -> Result<Vec<Rect>> {
            Ok(vec![Rect::new(0, 0, 1000, 800)])
        }

        fn mock_capture_next_key(&mut self, continuations: &[KeySym]) -> Result<()> {
            self.captures.push("capture");

            let mut keysyms: Vec<u32> = continuations.iter().map(|k| k.keysym).collect();
            keysyms.sort_unstable();
            self.continuations.push(keysyms);

            Ok(())
        }

        fn mock_cancel_capture_next_key(&mut self) -> Result<()> {
            self.captures.push("cancel");
            Ok(())
        }
    }

    type Log = Sender<&'static str>;

    /// A binding which records that it ran rather than doing anything.
    fn record(log: &Log, tag: &'static str) -> Box<dyn KeyEventHandler<TestConn>> {
        let log = log.clone();

        Box::new(move |_: &mut State<TestConn>, _: &mut TestConn| {
            log.send(tag).expect("log to be open");
            Ok(())
        })
    }

    fn key(keysym: u32) -> KeySym {
        KeySym { mask: 0, keysym }
    }

    /// Each key name is a single letter, mapping to its position in the alphabet.
    /// Anything else fails to parse.
    fn parse(k: &str) -> Result<KeySym> {
        match k.as_bytes() {
            [c @ b'a'..=b'z'] => Ok(key((c - b'a' + 1) as u32)),
            _ => Err(Error::UnknownKeyName { name: k.to_owned() }),
        }
    }

    fn parsed(patterns: &[&'static str], log: &Log) -> ParsedKeyBindings<TestConn> {
        let raw = patterns
            .iter()
            .map(|p| ((*p).to_string(), record(log, p)))
            .collect();

        KeyBindings::parse(raw, parse)
    }

    fn test_state(conn: &mut TestConn) -> State<TestConn> {
        State::try_new(Default::default(), conn).expect("test state")
    }

    fn dropped(parsed: &ParsedKeyBindings<TestConn>) -> Vec<&str> {
        let mut names: Vec<&str> = parsed.errors.iter().map(|e| e.binding.as_str()).collect();
        names.sort_unstable();

        names
    }

    // --- accumulating parse failures ---

    #[test]
    fn every_failure_is_reported_and_the_rest_are_kept() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a", "nope", "b", "also-nope"], &log);

        // Reporting every failure and keeping everything else are the same point: one typo
        // should cost the user that binding and nothing more.
        assert_eq!(dropped(&parsed), vec!["also-nope", "nope"]);

        let mut kept: Vec<u32> = parsed.bindings.sequences().map(|k| k[0].keysym).collect();
        kept.sort_unstable();
        assert_eq!(kept, vec![1, 2]);
    }

    // --- grouping sequences ---

    #[test]
    fn only_the_leading_key_of_a_sequence_is_grabbed() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a b", "a c", "z"], &log);

        // The rest of a sequence arrives through the capture rather than through a grab, so
        // only the first key of each binding needs grabbing.
        let mut grabbed: Vec<u32> = parsed
            .bindings
            .leading_keys()
            .iter()
            .map(|k| k.keysym)
            .collect();
        grabbed.sort_unstable();

        assert_eq!(grabbed, vec![1, 26]);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    }

    #[test]
    fn a_sequence_and_the_shorter_binding_it_starts_with_are_both_dropped() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a", "a b", "z"], &log);

        // Pressing "a" would fire it rather than waiting for "b", so "a b" could never be
        // reached. There is no telling which was meant, so neither is kept.
        assert_eq!(dropped(&parsed), vec!["a", "a b"]);
        assert_eq!(parsed.bindings.len(), 1);
    }

    #[test]
    fn an_overlap_drops_both_bindings_and_leaves_no_prefix_behind() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a b", "a b c"], &log);

        assert_eq!(dropped(&parsed), vec!["a b", "a b c"]);
        assert!(parsed.bindings.is_empty());
        // "a" would otherwise stay a prefix with nothing behind it, so pressing it would grab
        // the keyboard and swallow the next key press before giving up.
        assert!(parsed.bindings.prefixes.is_empty());
    }

    #[test]
    fn every_pattern_for_a_duplicated_sequence_is_dropped() {
        // The bindings arrive in a HashMap, so what is reported has to be stable across runs
        // by construction rather than by luck.
        for _ in 0..20 {
            let (log, _rx) = channel();

            // Whitespace makes these distinct keys in the map the user wrote, but the same
            // key sequence once parsed. Modifier order does the same: "M-S-j" and "S-M-j".
            let parsed = parsed(&["a", " a ", "a  "], &log);
            assert!(parsed.bindings.is_empty());

            let err = parsed.into_result().expect_err("duplicates to be reported");
            let Error::InvalidKeyBindings { errors } = err else {
                panic!("expected InvalidKeyBindings, got {err:?}");
            };

            assert_eq!(
                errors
                    .iter()
                    .map(|e| e.binding.as_str())
                    .collect::<Vec<_>>(),
                vec![" a ", "a", "a  "]
            );
        }
    }

    // --- dispatching sequences ---

    /// The state of a running window manager holding the given bindings.
    struct Running {
        bindings: KeyBindings<TestConn>,
        state: State<TestConn>,
        conn: TestConn,
        ran: Receiver<&'static str>,
    }

    impl Running {
        fn new(patterns: &[&'static str]) -> Self {
            let (log, ran) = channel();
            let bindings = parsed(patterns, &log)
                .into_result()
                .expect("test bindings to parse");
            let mut conn = TestConn::default();
            let state = test_state(&mut conn);

            Self {
                bindings,
                state,
                conn,
                ran,
            }
        }

        /// Feed a key press through the same path the backend uses.
        fn press(&mut self, keysym: u32) {
            dispatch_key(
                key(keysym),
                &mut self.bindings,
                &mut self.state,
                &mut self.conn,
            )
            .expect("dispatch");
        }

        fn ran(&self) -> Vec<&'static str> {
            self.ran.try_iter().collect()
        }
    }

    #[test]
    fn a_sequence_runs_its_binding_once_completed() {
        let mut wm = Running::new(&["a b", "a c"]);

        wm.press(1);
        assert!(
            wm.ran().is_empty(),
            "the leading key runs nothing on its own"
        );

        wm.press(3);
        assert_eq!(wm.ran(), vec!["a c"]);
        // Captured for the second key press, then released once it arrived.
        assert_eq!(wm.conn.captures, vec!["capture", "cancel"]);
    }

    #[test]
    fn a_key_that_continues_no_sequence_abandons_it() {
        let mut wm = Running::new(&["a b", "z"]);

        wm.press(1);
        wm.press(26);

        // "z" is consumed cancelling the sequence rather than running its own binding, which
        // is what stops a mistyped sequence from doing something unexpected.
        assert!(wm.ran().is_empty());
        assert_eq!(wm.conn.captures, vec!["capture", "cancel"]);

        // ... and the next press behaves normally again.
        wm.press(26);
        assert_eq!(wm.ran(), vec!["z"]);
    }

    #[test]
    fn a_sequence_only_claims_one_key_press() {
        let mut wm = Running::new(&["a b", "b"]);

        wm.press(1);
        wm.press(2);
        assert_eq!(wm.ran(), vec!["a b"]);

        wm.press(2);
        assert_eq!(wm.ran(), vec!["b"]);
    }

    #[test]
    fn a_capture_is_told_which_keys_would_continue_the_sequence() {
        let mut wm = Running::new(&["a b", "a c", "z"]);

        // The backend needs these to know what to listen for: a compositor which binds keys
        // on our behalf hears nothing at all from a key it has no binding for.
        wm.press(1);
        assert_eq!(wm.conn.continuations, vec![vec![2, 3]]);
    }

    #[test]
    fn each_step_of_a_sequence_narrows_the_continuations() {
        let mut wm = Running::new(&["a b c", "a b d", "a e"]);

        wm.press(1);
        wm.press(2);

        // Only the keys still reachable are expected, so a backend never leaves a key
        // registered for a branch the user has already stepped past.
        assert_eq!(wm.conn.continuations, vec![vec![2, 5], vec![3, 4]]);
    }

    #[test]
    fn longer_sequences_dispatch_a_key_at_a_time() {
        let mut wm = Running::new(&["a b c", "a b d"]);

        wm.press(1);
        wm.press(2);
        assert!(wm.ran().is_empty());
        assert_eq!(wm.conn.captures, vec!["capture", "capture"]);

        wm.press(4);
        assert_eq!(wm.ran(), vec!["a b d"]);
    }

    // --- parsing key names ---

    #[test]
    fn modifiers_and_key_name_parse() {
        let k = KeySym::parse("M-S-semicolon").expect("valid binding");

        assert_eq!(
            k.mask,
            u16::from(ModifierKey::Meta) | u16::from(ModifierKey::Shift)
        );
        assert_eq!(k.keysym, 0x3b);
    }

    #[test]
    fn each_modifier_has_its_own_bit() {
        for (pattern, modifier) in [
            ("C-a", ModifierKey::Ctrl),
            ("A-a", ModifierKey::Alt),
            ("S-a", ModifierKey::Shift),
            ("M-a", ModifierKey::Meta),
        ] {
            let k = KeySym::parse(pattern).expect(pattern);
            assert_eq!(k.mask, u16::from(modifier), "{pattern}");
        }
    }

    #[test]
    fn a_key_named_for_the_separator_still_parses() {
        // The pattern is split on '-', so the key that is itself a '-' is the awkward case.
        let k = KeySym::parse("M-minus").expect("valid binding");

        assert_eq!(k.mask, u16::from(ModifierKey::Meta));
        assert_eq!(k.keysym, XKeySym::XK_minus.keysym());
    }

    #[test]
    fn media_keys_parse_by_name() {
        let k = KeySym::parse("XF86AudioMute").expect("valid binding");

        assert_eq!(k.mask, 0);
        assert_eq!(k.keysym, 0x1008ff12);
    }

    #[test]
    fn unknown_names_are_reported() {
        // A key name we do not know and a modifier we do not know are both errors, and each
        // names the part that was not understood.
        assert!(matches!(
            KeySym::parse("M-nope"),
            Err(Error::UnknownKeyName { name }) if name == "nope"
        ));
        assert!(matches!(
            KeySym::parse("X-a"),
            Err(Error::UnknownModifier { name }) if name == "X"
        ));
    }
}
