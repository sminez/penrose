use crate::{
    core::{
        bindings::{KeyCode, ModifierKey, MouseButton, MouseEvent, MouseEventKind, MouseState},
        data_types::{Point, Region},
        xconnection::{
            event::ClientEventMask, ClientMessage, ClientMessageData, ConfigureEvent, ExposeEvent,
            PointerChange, PropertyEvent, Result, XAtomQuerier, XError, XEvent,
        },
    },
    x11rb::{xconn::X11rbConnection, X11rbError},
};

use strum::IntoEnumIterator;

use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{ClientMessageEvent, ConnectionExt as _, ModMask},
        Event,
    },
};

pub(crate) fn convert_event<C: Connection>(
    conn: &X11rbConnection<C>,
    event: Event,
) -> Result<Option<XEvent>> {
    match event {
        Event::RandrNotify(_) => Ok(Some(XEvent::RandrNotify)),
        Event::RandrScreenChangeNotify(_) => Ok(Some(XEvent::ScreenChange)),
        Event::ButtonPress(event) => Ok(to_mouse_state(event.detail, event.state).map(|state| {
            XEvent::MouseEvent(MouseEvent::new(
                event.event,
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
                event.event,
                event.root_x,
                event.root_y,
                event.event_x,
                event.event_y,
                state,
                MouseEventKind::Release,
            ))
        })),
        // FIXME: The 5 is due to https://github.com/sminez/penrose/issues/113
        Event::MotionNotify(event) => Ok(to_mouse_state(5, event.state).map(|state| {
            XEvent::MouseEvent(MouseEvent::new(
                event.event,
                event.root_x,
                event.root_y,
                event.event_x,
                event.event_y,
                state,
                MouseEventKind::Motion,
            ))
        })),
        Event::KeyPress(event) => {
            let code = KeyCode {
                mask: event.state,
                code: event.detail,
            };
            let numlock = ModMask::M2;
            Ok(Some(XEvent::KeyPress(
                code.ignoring_modifier(numlock.into()),
            )))
        }
        Event::MapRequest(event) => {
            let attr = conn
                .connection()
                .get_window_attributes(event.window)?
                .reply()?;
            Ok(Some(XEvent::MapRequest(
                event.window,
                attr.override_redirect,
            )))
        }
        Event::EnterNotify(event) => Ok(Some(XEvent::Enter(PointerChange {
            id: event.event,
            abs: Point::new(event.root_x as u32, event.root_y as u32),
            relative: Point::new(event.event_x as u32, event.event_y as u32),
        }))),
        Event::LeaveNotify(event) => Ok(Some(XEvent::Leave(PointerChange {
            id: event.event,
            abs: Point::new(event.root_x as u32, event.root_y as u32),
            relative: Point::new(event.event_x as u32, event.event_y as u32),
        }))),
        Event::DestroyNotify(event) => Ok(Some(XEvent::Destroy(event.window))),
        Event::ConfigureNotify(event) => Ok(Some(XEvent::ConfigureNotify(ConfigureEvent {
            id: event.window,
            r: Region::new(
                event.x as u32,
                event.y as u32,
                event.width as u32,
                event.height as u32,
            ),
            is_root: event.window == conn.root(),
        }))),
        Event::ConfigureRequest(event) => Ok(Some(XEvent::ConfigureRequest(ConfigureEvent {
            id: event.window,
            r: Region::new(
                event.x as u32,
                event.y as u32,
                event.width as u32,
                event.height as u32,
            ),
            is_root: event.window == conn.root(),
        }))),
        Event::Expose(event) => Ok(Some(XEvent::Expose(ExposeEvent {
            id: event.window,
            r: Region::new(
                event.x as u32,
                event.y as u32,
                event.width as u32,
                event.height as u32,
            ),
            count: event.count as usize,
        }))),
        Event::ClientMessage(event) => Ok(Some(to_client_message(conn, event)?)),
        Event::PropertyNotify(event) => Ok(Some(XEvent::PropertyNotify(PropertyEvent {
            id: event.window,
            atom: conn.atom_name(event.atom)?,
            is_root: event.window == conn.root(),
        }))),
        Event::Error(err) => Err(X11rbError::X11Error(err).into()),

        // NOTE: Ignoring other event types
        _ => Ok(None),
    }
}

fn to_mouse_state(detail: u8, state: u16) -> Option<MouseState> {
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
    let modifiers = ModifierKey::iter().filter(|m| is_held(m, state)).collect();
    Some(MouseState { button, modifiers })
}

fn to_client_message<C: Connection>(
    conn: &X11rbConnection<C>,
    event: ClientMessageEvent,
) -> Result<XEvent> {
    let name = conn.atom_name(event.type_)?;
    let data = match event.format {
        8 => ClientMessageData::from(event.data.as_data8()),
        16 => ClientMessageData::from(event.data.as_data16()),
        32 => ClientMessageData::from(event.data.as_data32()),
        invalid => return Err(XError::InvalidClientMessageData(invalid)),
    };
    Ok(XEvent::ClientMessage(ClientMessage::new(
        event.window,
        ClientEventMask::NoEventMask,
        name,
        data,
    )))
}
