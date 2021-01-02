//! Conversions to Penrose types from XCB types
use crate::{
    core::bindings::{KeyCode, ModifierKey, MouseButton, MouseEvent, MouseEventKind, MouseState},
    xcb::{Result, XcbError},
};

use strum::IntoEnumIterator;

use std::convert::TryFrom;

type XcbGeneric = xcb::Event<xcb::ffi::base::xcb_generic_event_t>;

impl ModifierKey {
    fn was_held(&self, mask: u16) -> bool {
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

impl From<xcb::KeyPressEvent> for KeyCode {
    fn from(e: xcb::KeyPressEvent) -> Self {
        Self {
            mask: e.state(),
            code: e.detail(),
        }
    }
}

impl From<&xcb::KeyPressEvent> for KeyCode {
    fn from(e: &xcb::KeyPressEvent) -> Self {
        Self {
            mask: e.state(),
            code: e.detail(),
        }
    }
}

impl TryFrom<XcbGeneric> for KeyCode {
    type Error = XcbError;

    fn try_from(e: XcbGeneric) -> Result<Self> {
        let r = e.response_type();
        if r == xcb::KEY_PRESS as u8 {
            let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&e) };
            Ok(key_press.into())
        } else {
            Err(XcbError::Raw("not an xcb key press".into()))
        }
    }
}

impl TryFrom<u8> for MouseButton {
    type Error = XcbError;

    fn try_from(n: u8) -> Result<Self> {
        match n {
            1 => Ok(Self::Left),
            2 => Ok(Self::Middle),
            3 => Ok(Self::Right),
            4 => Ok(Self::ScrollUp),
            5 => Ok(Self::ScrollDown),
            _ => Err(XcbError::UnknownMouseButton(n)),
        }
    }
}

impl MouseState {
    fn from_detail_and_state(detail: u8, state: u16) -> Result<Self> {
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

impl TryFrom<XcbGeneric> for MouseEvent {
    type Error = XcbError;

    fn try_from(raw: XcbGeneric) -> Result<Self> {
        let (detail, state, id, rx, ry, x, y, kind) = data_from_event(raw)?;
        let state = MouseState::from_detail_and_state(detail, state)?;
        Ok(MouseEvent::new(id, rx, ry, x, y, state, kind))
    }
}

#[allow(clippy::type_complexity)]
fn data_from_event(raw: XcbGeneric) -> Result<(u8, u16, u32, i16, i16, i16, i16, MouseEventKind)> {
    Ok(match raw.response_type() {
        xcb::BUTTON_PRESS => {
            let e: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&raw) };
            (
                e.detail(),
                e.state(),
                e.event(),
                e.root_x(),
                e.root_y(),
                e.event_x(),
                e.event_y(),
                MouseEventKind::Press,
            )
        }

        xcb::BUTTON_RELEASE => {
            let e: &xcb::ButtonReleaseEvent = unsafe { xcb::cast_event(&raw) };
            (
                e.detail(),
                e.state(),
                e.event(),
                e.root_x(),
                e.root_y(),
                e.event_x(),
                e.event_y(),
                MouseEventKind::Release,
            )
        }

        xcb::MOTION_NOTIFY => {
            let e: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&raw) };
            (
                e.detail(),
                e.state(),
                e.event(),
                e.root_x(),
                e.root_y(),
                e.event_x(),
                e.event_y(),
                MouseEventKind::Motion,
            )
        }
        _ => {
            return Err(XcbError::Raw(
                "not an xcb button press/release or motion notify".into(),
            ))
        }
    })
}
