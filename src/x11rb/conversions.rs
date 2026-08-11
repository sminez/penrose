//! Conversions to Penrose types from X11rb types
use crate::{
    Error, Result, WinId,
    core::bindings::{
        KeyCode, ModifierKey, MotionNotifyEvent, MouseButton, MouseEvent, MouseEventKind,
        MouseState,
    },
    pure::geometry::{Point, Rect},
    x::{
        XConn, XEvent,
        event::{
            ClientEventMask, ClientMessage, ClientMessageData, ConfigureEvent, ExposeEvent,
            PointerChange, PropertyEvent, ResizeRequestEvent,
        },
    },
    x11rb::Conn,
};
use strum::IntoEnumIterator;
use tracing::warn;
use x11rb::{
    connection::Connection,
    protocol::{
        ErrorKind, Event,
        xproto::{ClientMessageEvent, KeyButMask, ModMask},
    },
    x11_utils::X11Error,
};

pub(crate) fn convert_event<C: Connection + Send>(
    conn: &mut Conn<C>,
    event: Event,
) -> Result<Option<XEvent>> {
    match event {
        Event::RandrNotify(_) => Ok(Some(XEvent::RandrNotify)),

        Event::RandrScreenChangeNotify(_) => Ok(Some(XEvent::ScreenChange)),

        Event::ButtonPress(event) => Ok(to_mouse_state(event.detail, event.state).map(|state| {
            XEvent::MouseEvent(MouseEvent::new(
                WinId(event.child),
                event.root_x,
                event.root_y,
                event.event_x,
                event.event_y,
                state,
                MouseEventKind::Press,
            ))
        })),

        Event::ButtonRelease(event) => Ok(to_mouse_state(event.detail, event.state).map(|state| {
            XEvent::MouseEvent(MouseEvent::new(
                WinId(event.child),
                event.root_x,
                event.root_y,
                event.event_x,
                event.event_y,
                state,
                MouseEventKind::Release,
            ))
        })),

        // NOTE: the '1' here is not actually used
        Event::MotionNotify(event) => Ok(to_mouse_state(1, event.state).map(|state| {
            XEvent::MotionNotify(MotionNotifyEvent::new(
                WinId(event.child),
                event.root_x,
                event.root_y,
                event.event_x,
                event.event_y,
                state.modifiers,
            ))
        })),

        Event::KeyPress(event) => {
            if conn.capturing_next_key {
                // whole keyboard is captured, so modifier keys also arrive.
                if conn.modifier_keycodes.contains(&event.detail) {
                    return Ok(None);
                }
                // ungrab the keyboard since a key press has arrived.
                conn.end_capture()?;
            }

            let code = KeyCode {
                mask: event.state.into(),
                code: event.detail,
            };
            let numlock = ModMask::M2;
            Ok(Some(XEvent::KeyPress(
                code.ignoring_modifier(numlock.into()),
            )))
        }

        Event::MapRequest(event) => Ok(Some(XEvent::MapRequest(WinId(event.window)))),

        Event::UnmapNotify(event) => Ok(Some(XEvent::UnmapNotify(WinId(event.window)))),

        Event::EnterNotify(event) => Ok(Some(XEvent::Enter(PointerChange {
            id: WinId(event.event),
            abs: Point::new(event.root_x as i32, event.root_y as i32),
            relative: Point::new(event.event_x as i32, event.event_y as i32),
            same_screen: event.same_screen_focus == 0,
        }))),

        Event::LeaveNotify(event) => Ok(Some(XEvent::Leave(PointerChange {
            id: WinId(event.event),
            abs: Point::new(event.root_x as i32, event.root_y as i32),
            relative: Point::new(event.event_x as i32, event.event_y as i32),
            same_screen: event.same_screen_focus == 0,
        }))),

        Event::DestroyNotify(event) => Ok(Some(XEvent::Destroy(WinId(event.window)))),

        Event::ConfigureNotify(event) => Ok(Some(XEvent::ConfigureNotify(ConfigureEvent {
            id: WinId(event.window),
            r: Rect::new(
                event.x as i32,
                event.y as i32,
                event.width as u32,
                event.height as u32,
            ),
            is_root: event.window == *conn.root(),
        }))),

        Event::ConfigureRequest(event) => Ok(Some(XEvent::ConfigureRequest(ConfigureEvent {
            id: WinId(event.window),
            r: Rect::new(
                event.x as i32,
                event.y as i32,
                event.width as u32,
                event.height as u32,
            ),
            is_root: event.window == *conn.root(),
        }))),

        Event::ResizeRequest(event) => Ok(Some(XEvent::ResizeRequest(ResizeRequestEvent {
            id: WinId(event.window),
            width: event.width as u32,
            height: event.height as u32,
        }))),

        Event::Expose(event) => Ok(Some(XEvent::Expose(ExposeEvent {
            id: WinId(event.window),
            r: Rect::new(
                event.x as i32,
                event.y as i32,
                event.width as u32,
                event.height as u32,
            ),
            count: event.count as usize,
        }))),

        Event::ClientMessage(event) => Ok(Some(to_client_message(conn, event)?)),

        Event::PropertyNotify(event) => Ok(Some(XEvent::PropertyNotify(PropertyEvent {
            id: WinId(event.window),
            atom: conn.atom_name(WinId(event.atom))?,
            is_root: event.window == *conn.root(),
        }))),

        // Map known error codes that we know how to handle into penrose Errors
        Event::Error(X11Error {
            error_kind: ErrorKind::Window,
            error_code: 3,
            bad_value,
            ..
        }) => Err(Error::UnknownClient(WinId(bad_value))),

        // Other errors are returned directly
        Event::Error(e) => Err(Error::X11rbX11Error(e)),

        // Ignoring other event types
        _ => Ok(None),
    }
}

fn to_mouse_state(detail: u8, state: KeyButMask) -> Option<MouseState> {
    fn is_held(key: &ModifierKey, mask: u16) -> bool {
        mask & u16::from(*key) > 0
    }
    let button = match detail {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        4 => MouseButton::ScrollUp,
        5 => MouseButton::ScrollDown,
        _ => {
            warn!(button = detail, "dropping unknown mouse button event");
            return None;
        }
    };
    let state = u16::from(state);
    let modifiers = ModifierKey::iter().filter(|m| is_held(m, state)).collect();
    Some(MouseState { button, modifiers })
}

fn to_client_message<C: Connection + Send>(
    conn: &mut Conn<C>,
    event: ClientMessageEvent,
) -> Result<XEvent> {
    let name = conn.atom_name(WinId(event.type_))?;
    let data = match event.format {
        8 => ClientMessageData::from(event.data.as_data8()),
        16 => ClientMessageData::from(event.data.as_data16()),
        32 => ClientMessageData::from(event.data.as_data32()),
        format => return Err(Error::InvalidClientMessage { format }),
    };

    Ok(XEvent::ClientMessage(ClientMessage::new(
        WinId(event.window),
        ClientEventMask::NoEventMask,
        name,
        data,
    )))
}
