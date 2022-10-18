//! Conversions to Penrose types from XCB types
use crate::{
    bindings::{KeyCode, MouseEvent, MouseEventKind, MouseState},
    geometry::Rect,
    x::{ClientAttr, ClientConfig},
    xcb::{Error, Result, XcbGenericEvent},
    Xid,
};
use std::convert::TryFrom;

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
            Err(Error::XcbUnexpectedResponseType {
                expected: vec![xcb::KEY_PRESS],
                received: r,
            })
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
            Err(Error::XcbUnexpectedResponseType {
                expected: vec![xcb::KEY_PRESS],
                received: r,
            })
        }
    }
}

impl TryFrom<XcbGenericEvent> for MouseEvent {
    type Error = Error;

    fn try_from(raw: XcbGenericEvent) -> Result<Self> {
        let (detail, state, id, rx, ry, x, y, kind) = data_from_event(raw)?;
        let state = MouseState::from_detail_and_state(detail, state)?;
        Ok(MouseEvent::new(Xid(id), rx, ry, x, y, state, kind))
    }
}

#[allow(clippy::type_complexity)]
fn data_from_event(
    raw: XcbGenericEvent,
) -> Result<(u8, u16, u32, i16, i16, i16, i16, MouseEventKind)> {
    let data = match raw.response_type() {
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

        received => {
            return Err(Error::XcbUnexpectedResponseType {
                expected: vec![xcb::BUTTON_PRESS, xcb::BUTTON_RELEASE, xcb::MOTION_NOTIFY],
                received,
            })
        }
    };

    Ok(data)
}

impl From<&ClientConfig> for Vec<(u16, u32)> {
    fn from(w: &ClientConfig) -> Vec<(u16, u32)> {
        match w {
            ClientConfig::BorderPx(px) => vec![(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, *px)],

            ClientConfig::StackAbove => {
                vec![(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]
            }

            ClientConfig::Position(rect) => {
                let Rect { x, y, w, h } = *rect;
                vec![
                    (xcb::CONFIG_WINDOW_X as u16, x),
                    (xcb::CONFIG_WINDOW_Y as u16, y),
                    (xcb::CONFIG_WINDOW_WIDTH as u16, w),
                    (xcb::CONFIG_WINDOW_HEIGHT as u16, h),
                ]
            }
        }
    }
}

impl From<&ClientAttr> for Vec<(u32, u32)> {
    fn from(w: &ClientAttr) -> Vec<(u32, u32)> {
        let client_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_STRUCTURE_NOTIFY;

        let client_unmap_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE;

        let root_mask = xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xcb::EVENT_MASK_BUTTON_MOTION;

        match w {
            ClientAttr::BorderColor(c) => vec![(xcb::CW_BORDER_PIXEL, *c)],
            ClientAttr::ClientEventMask => vec![(xcb::CW_EVENT_MASK, client_mask)],
            ClientAttr::ClientUnmapMask => vec![(xcb::CW_EVENT_MASK, client_unmap_mask)],
            ClientAttr::RootEventMask => vec![(xcb::CW_EVENT_MASK, root_mask)],
        }
    }
}
