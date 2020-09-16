//! Setting up and responding to user defined key/mouse bindings
use crate::{
    data_types::{Point, WinId},
    Result, WindowManager,
};

use std::{collections::HashMap, convert::TryFrom};

use anyhow::anyhow;
use strum::{EnumIter, IntoEnumIterator};
use xcb;

/// Some action to be run by a user key binding
pub type FireAndForget = Box<dyn FnMut(&mut WindowManager) -> ()>;

/// An action to be run in response to a mouse event
pub type MouseEventHandler = Box<dyn FnMut(&mut WindowManager, &MouseEvent) -> ()>;

/// User defined key bindings
pub type KeyBindings = HashMap<KeyCode, FireAndForget>;

/// User defined mouse bindings
pub type MouseBindings = HashMap<MouseState, MouseEventHandler>;

pub(crate) type CodeMap = HashMap<String, u8>;

/// A key press and held modifiers
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct KeyCode {
    /// The held modifier mask
    pub mask: u16,
    /// The key code that was held
    pub code: u8,
}

impl KeyCode {
    /// Create a new KeyCode from an xcb keypress event
    pub fn from_key_press(k: &xcb::KeyPressEvent) -> KeyCode {
        KeyCode {
            mask: k.state(),
            code: k.detail(),
        }
    }

    /// Create a new KeyCode from an existing one, removing the given modifier mask
    pub fn ignoring_modifier(&self, mask: u16) -> KeyCode {
        KeyCode {
            mask: self.mask & !mask,
            code: self.code,
        }
    }
}

/// Known mouse buttons for binding actions
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
    type Error = anyhow::Error;

    fn try_from(n: u8) -> Result<Self> {
        match n {
            1 => Ok(Self::Left),
            2 => Ok(Self::Middle),
            3 => Ok(Self::Right),
            4 => Ok(Self::ScrollUp),
            5 => Ok(Self::ScrollDown),
            _ => Err(anyhow!("unknown mouse button {}", n)),
        }
    }
}

/// Known modifier keys for bindings
#[derive(Debug, EnumIter, PartialEq, Eq, Hash, Clone, Copy)]
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
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "C" => Ok(Self::Ctrl),
            "A" => Ok(Self::Alt),
            "S" => Ok(Self::Shift),
            "M" => Ok(Self::Meta),
            _ => Err(anyhow!("unknown modifier {}", s)),
        }
    }
}

/// A mouse state specification indicating the button and modifiers held
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MouseState {
    button: MouseButton,
    modifiers: Vec<ModifierKey>,
}

impl MouseState {
    /// Construct a new MouseState
    pub fn new(button: MouseButton, modifiers: Vec<ModifierKey>) -> Self {
        Self { button, modifiers }
    }

    pub(crate) fn from_event(e: &xcb::ButtonPressEvent) -> Result<Self> {
        Ok(Self {
            button: MouseButton::try_from(e.detail())?,
            modifiers: ModifierKey::iter()
                .filter(|m| m.was_held(e.state()))
                .collect(),
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
#[derive(Debug, Clone)]
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
