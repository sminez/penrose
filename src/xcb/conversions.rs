//! Conversions to Penrose types from XCB types
use crate::{
    common::bindings::{KeyCode, ModifierKey, MouseButton, MouseEvent, MouseEventKind, MouseState},
    xcb::{Error, Result, XcbGenericEvent},
    xconnection::{ClientAttr, ClientConfig},
};
use std::convert::TryFrom;
use strum::IntoEnumIterator;

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

impl TryFrom<XcbGenericEvent> for KeyCode {
    type Error = Error;

    fn try_from(e: XcbGenericEvent) -> Result<Self> {
        let r = e.response_type();
        if r == xcb::KEY_PRESS as u8 {
            let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&e) };
            Ok(key_press.into())
        } else {
            Err(Error::Raw("not an xcb key press".into()))
        }
    }
}

impl TryFrom<&XcbGenericEvent> for KeyCode {
    type Error = Error;

    fn try_from(e: &XcbGenericEvent) -> Result<Self> {
        let r = e.response_type();
        if r == xcb::KEY_PRESS as u8 {
            let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(e) };
            Ok(key_press.into())
        } else {
            Err(Error::Raw("not an xcb key press".into()))
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
            _ => Err(Error::UnknownMouseButton(n)),
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

impl TryFrom<XcbGenericEvent> for MouseEvent {
    type Error = Error;

    fn try_from(raw: XcbGenericEvent) -> Result<Self> {
        let (detail, state, id, rx, ry, x, y, kind) = data_from_event(raw)?;
        let state = MouseState::from_detail_and_state(detail, state)?;
        Ok(MouseEvent::new(id, rx, ry, x, y, state, kind))
    }
}

#[allow(clippy::type_complexity)]
fn data_from_event(
    raw: XcbGenericEvent,
) -> Result<(u8, u16, u32, i16, i16, i16, i16, MouseEventKind)> {
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
            return Err(Error::Raw(
                "not an xcb button press/release or motion notify".into(),
            ))
        }
    })
}

impl From<&ClientConfig> for Vec<(u16, u32)> {
    fn from(w: &ClientConfig) -> Vec<(u16, u32)> {
        match w {
            ClientConfig::BorderPx(px) => vec![(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, *px)],
            ClientConfig::Position(region) => {
                let (x, y, w, h) = region.values();
                vec![
                    (xcb::CONFIG_WINDOW_X as u16, x),
                    (xcb::CONFIG_WINDOW_Y as u16, y),
                    (xcb::CONFIG_WINDOW_WIDTH as u16, w),
                    (xcb::CONFIG_WINDOW_HEIGHT as u16, h),
                ]
            }
            ClientConfig::StackAbove => {
                vec![(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]
            }
        }
    }
}

impl From<&ClientAttr> for Vec<(u32, u32)> {
    fn from(w: &ClientAttr) -> Vec<(u32, u32)> {
        let client_event_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_STRUCTURE_NOTIFY;

        let root_event_mask = xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xcb::EVENT_MASK_BUTTON_MOTION;

        match w {
            ClientAttr::BorderColor(c) => vec![(xcb::CW_BORDER_PIXEL, *c)],
            ClientAttr::ClientEventMask => vec![(xcb::CW_EVENT_MASK, client_event_mask)],
            ClientAttr::RootEventMask => vec![(xcb::CW_EVENT_MASK, root_event_mask)],
        }
    }
}
