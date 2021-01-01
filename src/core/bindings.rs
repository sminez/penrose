//! Setting up and responding to user defined key/mouse bindings
use crate::{
    core::{
        data_types::{Point, WinId},
        manager::WindowManager,
    },
    PenroseError, Result,
};

use std::{collections::HashMap, convert::TryFrom};

use strum::{EnumIter, IntoEnumIterator};

/// Some action to be run by a user key binding
pub type FireAndForget<X> = Box<dyn FnMut(&mut WindowManager<X>)>;

/// An action to be run in response to a mouse event
pub type MouseEventHandler<X> = Box<dyn FnMut(&mut WindowManager<X>, &MouseEvent)>;

/// User defined key bindings
pub type KeyBindings<X> = HashMap<KeyCode, FireAndForget<X>>;

/// User defined mouse bindings
pub type MouseBindings<X> = HashMap<(MouseEventKind, MouseState), MouseEventHandler<X>>;

pub(crate) type CodeMap = HashMap<String, u8>;

/// A key press and held modifiers
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KeyCode {
    /// The held modifier mask
    pub mask: u16,
    /// The key code that was held
    pub code: u8,
}

impl KeyCode {
    pub(crate) fn from_key_press(k: &xcb::KeyPressEvent) -> KeyCode {
        KeyCode {
            mask: k.state(),
            code: k.detail(),
        }
    }

    pub(crate) fn ignoring_modifier(&self, mask: u16) -> KeyCode {
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
    type Error = PenroseError;

    fn try_from(n: u8) -> Result<Self> {
        match n {
            1 => Ok(Self::Left),
            2 => Ok(Self::Middle),
            3 => Ok(Self::Right),
            4 => Ok(Self::ScrollUp),
            5 => Ok(Self::ScrollDown),
            _ => Err(PenroseError::UnknownMouseButton(n)),
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
    pub(crate) fn was_held(&self, mask: u16) -> bool {
        mask & u16::from(*self) > 0
    }
}

impl From<ModifierKey> for u16 {
    fn from(m: ModifierKey) -> u16 {
        (match m {
            ModifierKey::Ctrl => xcb::MOD_MASK_CONTROL,
            ModifierKey::Alt => xcb::MOD_MASK_1,
            ModifierKey::Shift => xcb::MOD_MASK_SHIFT,
            ModifierKey::Meta => xcb::MOD_MASK_4,
        }) as u16
    }
}

impl TryFrom<&str> for ModifierKey {
    type Error = PenroseError;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "C" => Ok(Self::Ctrl),
            "A" => Ok(Self::Alt),
            "S" => Ok(Self::Shift),
            "M" => Ok(Self::Meta),
            _ => Err(PenroseError::UnknownModifier(s.into())),
        }
    }
}

/// A mouse state specification indicating the button and modifiers held
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseState {
    button: MouseButton,
    modifiers: Vec<ModifierKey>,
}

impl MouseState {
    /// Construct a new MouseState
    pub fn new(button: MouseButton, mut modifiers: Vec<ModifierKey>) -> Self {
        modifiers.sort();
        Self { button, modifiers }
    }

    pub(crate) fn from_event(detail: u8, state: u16) -> Result<Self> {
        Ok(Self {
            button: MouseButton::try_from(detail)?,
            modifiers: ModifierKey::iter().filter(|m| m.was_held(state)).collect(),
        })
    }

    pub(crate) fn mask(&self) -> u16 {
        self.modifiers
            .iter()
            .fold(0, |acc, &val| acc | u16::from(val))
    }

    pub(crate) fn button(&self) -> u8 {
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MouseEvent {
    /// The ID of the window that was contained the click
    pub id: WinId,
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
    fn new(
        id: WinId,
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

    pub(crate) fn from_press(e: &xcb::ButtonPressEvent) -> Result<Self> {
        let state = MouseState::from_event(e.detail(), e.state())?;
        Ok(Self::new(
            e.event(),
            e.root_x(),
            e.root_y(),
            e.event_x(),
            e.event_y(),
            state,
            MouseEventKind::Press,
        ))
    }

    pub(crate) fn from_release(e: &xcb::ButtonReleaseEvent) -> Result<Self> {
        let state = MouseState::from_event(e.detail(), e.state())?;
        Ok(Self::new(
            e.event(),
            e.root_x(),
            e.root_y(),
            e.event_x(),
            e.event_y(),
            state,
            MouseEventKind::Release,
        ))
    }

    pub(crate) fn from_motion(e: &xcb::MotionNotifyEvent) -> Result<Self> {
        let state = MouseState::from_event(e.detail(), e.state())?;
        Ok(Self::new(
            e.event(),
            e.root_x(),
            e.root_y(),
            e.event_x(),
            e.event_y(),
            state,
            MouseEventKind::Motion,
        ))
    }
}
