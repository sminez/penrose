//! Setting up and responding to user defined key/mouse bindings
use crate::{
    Error, Result,
    core::{
        State,
        conn::{Conn, WinId},
    },
    pure::geometry::Point,
    x::XConn,
};
#[cfg(feature = "keysyms")]
use penrose_keysyms::XKeySym;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt, mem,
    process::Command,
};
use strum::{EnumIter, IntoEnumIterator};
use tracing::{debug, error, trace};

/// Run the xmodmap command to dump the system keymap table.
///
/// This is done in a form that we can load in and convert back to key
/// codes. This lets the user define key bindings in the way that they
/// would expect while also ensuring that it is easy to debug any odd
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

/// The keybindings that parsed, along with any errors that occurred.
#[derive(Debug)]
pub struct ParsedKeyBindings<C: Conn> {
    pub bindings: KeyBindings<C>,
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

/// Parse string format key bindings using the given parse function, collecting any failures.
///
/// A key string containing whitespace is a *sequence*: `"M-m M-l"` fires when `M-l` is pressed
/// after `M-m`, and neither key does anything on its own. Sequences may be any length. Binding
/// both a chord and something shorter that it starts with is ambiguous, so the longer binding
/// is dropped and reported, as is any sequence described more than once.
pub fn parse_keybindings<S, C, F>(
    str_bindings: HashMap<S, Box<dyn KeyEventHandler<C>>>,
    mut parse: F,
) -> ParsedKeyBindings<C>
where
    S: AsRef<str>,
    C: Conn,
    F: FnMut(&str) -> Result<C::KeyBindingKey>,
{
    let mut errors = Vec::new();
    let mut parsed = Vec::new();

    for (s, handler) in str_bindings {
        let binding = s.as_ref().to_owned();
        let keys: Result<Vec<C::KeyBindingKey>> =
            s.as_ref().split_whitespace().map(&mut parse).collect();

        match keys {
            Err(error) => errors.push(KeyBindingError { binding, error }),
            Ok(keys) if keys.is_empty() => errors.push(KeyBindingError {
                error: Error::Custom("no keys in binding".to_owned()),
                binding,
            }),
            Ok(keys) => parsed.push((binding, keys, handler)),
        }
    }

    // Shortest first, so which spelling of a duplicated sequence survives is the tidiest one
    // rather than whatever the map happened to iterate first.
    parsed.sort_by(|(a, ..), (b, ..)| (a.len(), a).cmp(&(b.len(), b)));

    let mut named = Claimed::<C>::new();

    for (binding, keys, handler) in parsed {
        match named.get(&keys) {
            Some((owner, _)) => errors.push(KeyBindingError {
                error: Error::Custom(format!("duplicate of '{owner}'")),
                binding,
            }),
            None => {
                named.insert(keys, (binding, handler));
            }
        }
    }

    // Pressing the shorter binding fires it rather than waiting to see whether a longer one
    // was meant, so the longer ones could never be reached.
    let shadowed: Vec<_> = named
        .keys()
        .filter_map(|keys| {
            let (by, _) = (1..keys.len()).find_map(|n| named.get(&keys[..n]))?;
            Some((keys.clone(), by.clone()))
        })
        .collect();

    for (keys, by) in shadowed {
        if let Some((binding, _)) = named.remove(&keys) {
            errors.push(KeyBindingError {
                error: Error::Custom(format!("shadowed by '{by}'")),
                binding,
            });
        }
    }

    errors.sort_by(|a, b| a.binding.cmp(&b.binding));

    ParsedKeyBindings {
        bindings: KeyBindings::new(named.into_iter().map(|(k, (_, h))| (k, h)).collect()),
        errors,
    }
}

/// Each parsed key sequence, the binding which claimed it, and what it runs.
type Claimed<C> = HashMap<Vec<<C as Conn>::KeyBindingKey>, (String, Box<dyn KeyEventHandler<C>>)>;

/// Run the binding a key press completes, waiting for more keys if it begins a longer one.
///
/// [Conn] implementations should call this for every key press they receive rather than
/// looking bindings up themselves, so that chorded bindings work on every backend.
///
/// A key press which continues no binding abandons the sequence in progress rather than
/// leaving the window manager in a state where the user's bindings have silently stopped
/// working.
pub fn dispatch_key<C: Conn>(
    key: C::KeyBindingKey,
    bindings: &mut KeyBindings<C>,
    state: &mut State<C>,
    conn: &mut C,
) -> Result<()> {
    state.pending_keys.push(key);

    if bindings.is_prefix(&state.pending_keys) {
        trace!(pending = ?state.pending_keys, "waiting for the rest of a key sequence");
        return conn.capture_next_key();
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

/// Parse string format key bindings into [KeyCode] based [KeyBindings] using
/// the command line `xmodmap` utility.
///
/// Every binding must be usable: if any are not, this returns an error naming all of them
/// rather than only the first. See [parse_keybindings_with_xmodmap_or_log] to keep the
/// bindings that did parse instead.
///
/// See [keycodes_from_xmodmap] for details of how `xmodmap` is used.
pub fn parse_keybindings_with_xmodmap<S, X>(
    str_bindings: HashMap<S, Box<dyn KeyEventHandler<X>>>,
) -> Result<KeyBindings<X>>
where
    S: AsRef<str>,
    X: XConn,
{
    xmodmap_bindings(str_bindings)?.into_result()
}

/// Parse string format key bindings as [parse_keybindings_with_xmodmap] does, but keep going
/// when one of them cannot be used.
///
/// Bindings which fail are logged at error level and dropped; everything else still works.
/// This only fails if `xmodmap` itself could not be run, in which case there is nothing to
/// parse against and no partial result to keep.
pub fn parse_keybindings_with_xmodmap_or_log<S, X>(
    str_bindings: HashMap<S, Box<dyn KeyEventHandler<X>>>,
) -> Result<KeyBindings<X>>
where
    S: AsRef<str>,
    X: XConn,
{
    Ok(xmodmap_bindings(str_bindings)?.log_errors())
}

fn xmodmap_bindings<S, X>(
    str_bindings: HashMap<S, Box<dyn KeyEventHandler<X>>>,
) -> Result<ParsedKeyBindings<X>>
where
    S: AsRef<str>,
    X: XConn,
{
    let m = keycodes_from_xmodmap()?;

    Ok(parse_keybindings(str_bindings, |k| parse_binding(k, &m)))
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

/// User defined key bindings, keyed by the sequence of key presses which run them.
///
/// Built by [parse_keybindings], which is where a sequence bound twice, or shadowed by a
/// shorter one it starts with, is reported and dropped.
pub struct KeyBindings<C: Conn> {
    bindings: HashMap<Vec<C::KeyBindingKey>, Box<dyn KeyEventHandler<C>>>,
    /// Every sequence which begins a binding without being one itself.
    prefixes: HashSet<Vec<C::KeyBindingKey>>,
}

impl<C: Conn> fmt::Debug for KeyBindings<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyBindings")
            .field("bindings", &self.bindings)
            .finish()
    }
}

impl<C: Conn> KeyBindings<C> {
    /// Index a map of key sequences for dispatch.
    ///
    /// Private so that [parse_keybindings] is the only way to obtain a `KeyBindings`, which
    /// is what makes "no sequence bound twice, none shadowed by a shorter one" a property of
    /// the type rather than of remembering to check. A sequence which is both a binding and
    /// the start of a longer one can only ever run as the shorter of the two, so it is not
    /// treated as a prefix and the longer one is unreachable.
    fn new(bindings: HashMap<Vec<C::KeyBindingKey>, Box<dyn KeyEventHandler<C>>>) -> Self {
        let prefixes = bindings
            .keys()
            .flat_map(|keys| (1..keys.len()).map(|n| keys[..n].to_vec()))
            .filter(|prefix| !bindings.contains_key(prefix))
            .collect();

        Self { bindings, prefixes }
    }

    /// The number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether there are no bindings at all.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// The key sequence which runs each binding.
    pub fn sequences(&self) -> impl Iterator<Item = &[C::KeyBindingKey]> {
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

    /// Whether more keys are needed before this sequence can run anything.
    fn is_prefix(&self, keys: &[C::KeyBindingKey]) -> bool {
        // Checked first so that a config with no chords in it does no work at all here.
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

    #[derive(Default)]
    struct TestConn {
        captures: Vec<&'static str>,
    }

    impl MockXConn for TestConn {
        fn mock_screen_details(&mut self) -> Result<Vec<Rect>> {
            Ok(vec![Rect::new(0, 0, 1000, 800)])
        }

        fn mock_capture_next_key(&mut self) -> Result<()> {
            self.captures.push("capture");
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

    fn key(code: u8) -> KeyCode {
        KeyCode { mask: 0, code }
    }

    /// Each key description is a single letter, mapping to its position in the alphabet.
    /// Anything else fails to parse.
    fn parse(k: &str) -> Result<KeyCode> {
        match k.as_bytes() {
            [c @ b'a'..=b'z'] => Ok(key(c - b'a' + 1)),
            _ => Err(Error::UnknownKeyName { name: k.to_owned() }),
        }
    }

    fn parsed(patterns: &[&'static str], log: &Log) -> ParsedKeyBindings<TestConn> {
        let raw = patterns
            .iter()
            .map(|p| ((*p).to_string(), record(log, p)))
            .collect();

        parse_keybindings(raw, parse)
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
    fn every_failure_is_reported_not_just_the_first() {
        let (log, _rx) = channel();

        assert_eq!(
            dropped(&parsed(&["a", "nope", "b", "also-nope"], &log)),
            vec!["also-nope", "nope"]
        );
    }

    #[test]
    fn bindings_that_parse_are_kept() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a", "nope", "b"], &log);

        // Dropping only what could not be used is the point: one typo should not cost the
        // user every other binding they wrote.
        let mut kept: Vec<u8> = parsed.bindings.sequences().map(|k| k[0].code).collect();
        kept.sort_unstable();
        assert_eq!(kept, vec![1, 2]);
    }

    #[test]
    fn all_or_error_names_every_failure() {
        let (log, _rx) = channel();
        let err = parsed(&["a", "nope", "also-nope"], &log)
            .into_result()
            .expect_err("failures to be reported");

        let msg = err.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("also-nope"), "{msg}");
        // The underlying error is worth having, not just the name of the binding.
        assert!(msg.contains("is not a known key name"), "{msg}");
    }

    #[test]
    fn all_or_error_yields_the_bindings_when_everything_parses() {
        let (log, _rx) = channel();
        let bindings = parsed(&["a", "b"], &log)
            .into_result()
            .expect("no failures");

        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn log_errors_keeps_what_parsed() {
        let (log, _rx) = channel();

        assert_eq!(parsed(&["a", "nope"], &log).log_errors().len(), 1);
    }

    // --- grouping chords ---

    #[test]
    fn only_the_leading_key_of_a_chord_is_grabbed() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a b", "a c", "z"], &log);

        // The rest of a sequence arrives through the capture rather than through a grab, so
        // only the first key of each binding needs grabbing.
        let mut grabbed: Vec<u8> = parsed
            .bindings
            .leading_keys()
            .iter()
            .map(|k| k.code)
            .collect();
        grabbed.sort_unstable();

        assert_eq!(grabbed, vec![1, 26]);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    }

    #[test]
    fn a_chord_shadowed_by_a_shorter_binding_is_dropped() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a", "a b"], &log);

        // Pressing "a" fires it immediately, so "a b" could never be reached.
        assert_eq!(dropped(&parsed), vec!["a b"]);
        assert_eq!(parsed.bindings.len(), 1);
    }

    #[test]
    fn a_chord_shadowed_partway_through_is_dropped() {
        let (log, _rx) = channel();

        assert_eq!(dropped(&parsed(&["a b", "a b c"], &log)), vec!["a b c"]);
    }

    #[test]
    fn chords_can_be_any_length() {
        let (log, _rx) = channel();
        let parsed = parsed(&["a b c", "a b d"], &log);

        // Nothing has to be declared for depth: a sequence is just a longer map key.
        assert_eq!(parsed.bindings.len(), 2);
        assert_eq!(parsed.bindings.leading_keys().len(), 1);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    }

    #[test]
    fn two_descriptions_of_one_key_sequence_keep_the_first_by_name() {
        let (log, _rx) = channel();

        // Whitespace makes these distinct keys in the map the user wrote, but the same key
        // sequence once parsed. Modifier order does the same thing: "M-S-j" and "S-M-j".
        let parsed = parsed(&["a", " a "], &log);

        assert_eq!(dropped(&parsed), vec![" a "]);
        assert_eq!(parsed.bindings.len(), 1);
    }

    #[test]
    fn which_duplicate_is_dropped_does_not_depend_on_map_ordering() {
        // The bindings arrive in a HashMap, so the only thing keeping this stable across runs
        // is sorting them: without it either description could win.
        for _ in 0..20 {
            let (log, _rx) = channel();
            let parsed = parsed(&["a", " a ", "a  "], &log);

            assert_eq!(dropped(&parsed), vec![" a ", "a  "]);
        }
    }

    #[test]
    fn a_duplicate_names_the_binding_that_won() {
        let (log, _rx) = channel();
        let err = parsed(&["a", " a "], &log)
            .into_result()
            .expect_err("duplicate to be reported");

        // Quoted, because otherwise a duplicate caused by stray whitespace is invisible.
        assert!(err.to_string().contains("' a ': duplicate of 'a'"), "{err}");
    }

    #[test]
    fn duplicated_chords_are_reported_too() {
        let (log, _rx) = channel();

        assert_eq!(dropped(&parsed(&["a b", "a  b"], &log)), vec!["a  b"]);
    }

    #[test]
    fn a_config_without_chords_has_no_prefixes() {
        let (log, _rx) = channel();

        // What makes the prefix check free for the configs that never use a chord.
        assert!(parsed(&["a", "b"], &log).bindings.prefixes.is_empty());
    }

    #[test]
    fn a_hand_built_map_resolves_shadowing_rather_than_hanging() {
        let (log, rx) = channel();
        let mut map = HashMap::new();
        map.insert(vec![key(1)], record(&log, "a"));
        map.insert(vec![key(1), key(2)], record(&log, "a b"));

        // Parsing rejects this pair, but nothing stops it being built by hand. Treating "a"
        // as a prefix would leave it waiting for a key that can never make it run.
        let mut bindings = KeyBindings::new(map);
        let mut conn = TestConn::default();
        let mut state = test_state(&mut conn);

        dispatch_key(key(1), &mut bindings, &mut state, &mut conn).expect("dispatch");
        assert_eq!(rx.try_iter().collect::<Vec<_>>(), vec!["a"]);
        assert!(conn.captures.is_empty());
    }

    // --- dispatching chords ---

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
        fn press(&mut self, code: u8) {
            dispatch_key(
                key(code),
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
    fn a_chord_runs_its_binding_once_completed() {
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
    fn a_key_that_continues_no_chord_abandons_it() {
        let mut wm = Running::new(&["a b", "z"]);

        wm.press(1);
        wm.press(26);

        // "z" is consumed cancelling the chord rather than running its own binding, which is
        // what stops a mistyped chord from doing something unexpected.
        assert!(wm.ran().is_empty());
        assert_eq!(wm.conn.captures, vec!["capture", "cancel"]);

        // ... and the next press behaves normally again.
        wm.press(26);
        assert_eq!(wm.ran(), vec!["z"]);
    }

    #[test]
    fn a_chord_only_claims_one_key_press() {
        let mut wm = Running::new(&["a b", "b"]);

        wm.press(1);
        wm.press(2);
        assert_eq!(wm.ran(), vec!["a b"]);

        wm.press(2);
        assert_eq!(wm.ran(), vec!["b"]);
    }

    #[test]
    fn nested_chords_dispatch_a_level_at_a_time() {
        let mut wm = Running::new(&["a b c", "a b d"]);

        wm.press(1);
        wm.press(2);
        assert!(wm.ran().is_empty());
        assert_eq!(wm.conn.captures, vec!["capture", "capture"]);

        wm.press(4);
        assert_eq!(wm.ran(), vec!["a b d"]);
    }
}
