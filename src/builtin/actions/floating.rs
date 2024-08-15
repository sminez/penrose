//! Actions for manipulating floating windows.
use crate::{
    builtin::actions::{key_handler, modify_with},
    core::{
        bindings::{
            KeyEventHandler, MotionNotifyEvent, MouseEvent, MouseEventHandler, MouseEventKind,
        },
        State,
    },
    custom_error,
    pure::geometry::{Point, Rect},
    x::{XConn, XConnExt},
    Result, Xid,
};
use tracing::error;

/// Resize a currently floating window by a given (width, height) delta
///
/// Screen coordinates are 0-indexed from the top left corner of the sceen.
pub fn resize<X: XConn>(dw: i32, dh: i32) -> Box<dyn KeyEventHandler<X>> {
    modify_with(move |cs| {
        let id = match cs.current_client() {
            Some(&id) => id,
            None => return,
        };

        cs.floating.entry(id).and_modify(|r| {
            *r = r.apply_as_rect(&cs.screens.focus.r, |mut r| {
                r.resize(dw, dh);
                r
            });
        });
    })
}

/// Move a currently floating window by a given (x, y) delta
///
/// Screen coordinates are 0-indexed from the top left corner of the sceen.
pub fn reposition<X: XConn>(dx: i32, dy: i32) -> Box<dyn KeyEventHandler<X>> {
    modify_with(move |cs| {
        let id = match cs.current_client() {
            Some(&id) => id,
            None => return,
        };

        cs.floating.entry(id).and_modify(|r| {
            *r = r.apply_as_rect(&cs.screens.focus.r, |mut r| {
                r.reposition(dx, dy);
                r
            });
        });
    })
}

/// Move the currently focused window to the floating layer in its current on screen position
pub fn float_focused<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let id = match state.client_set.current_client() {
            Some(&id) => id,
            None => return Ok(()),
        };

        let r = x.client_geometry(id)?;

        x.modify_and_refresh(state, |cs| {
            if let Err(err) = cs.float(id, r) {
                error!(%err, %id, "unable to float requested client window");
            }
        })
    })
}

/// Sink the current window back into tiling mode if it was floating
pub fn sink_focused<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    modify_with(|cs| {
        let id = match cs.current_client() {
            Some(&id) => id,
            None => return,
        };

        cs.sink(&id);
    })
}

/// Sink the current window if it was floating, float it if it was tiled.
pub fn toggle_floating_focused<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let id = match state.client_set.current_client() {
            Some(&id) => id,
            None => return Ok(()),
        };

        let r = x.client_geometry(id)?;

        x.modify_and_refresh(state, |cs| {
            if let Err(err) = cs.toggle_floating_state(id, r) {
                error!(%err, %id, "unable to float requested client window");
            }
        })
    })
}

/// Float all windows in their current tiled position
pub fn float_all<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let positions = state.visible_client_positions(x);

        x.modify_and_refresh(state, |cs| {
            for &(id, r) in positions.iter() {
                if let Err(err) = cs.float(id, r) {
                    error!(%err, %id, "unable to float requested client window");
                }
            }
        })
    })
}

/// Sink all floating windows back into their tiled positions
pub fn sink_all<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    modify_with(|cs| cs.floating.clear())
}

#[derive(Debug, Default, Clone, Copy)]
struct ClickData {
    x_initial: i32,
    y_initial: i32,
    r_initial: Rect,
}

impl ClickData {
    fn on_motion<X: XConn>(
        &self,
        f: impl Fn(&mut Rect, i32, i32),
        id: Xid,
        rpt: Point,
        state: &mut State<X>,
        x: &X,
    ) -> Result<()> {
        let (dx, dy) = (rpt.x as i32 - self.x_initial, rpt.y as i32 - self.y_initial);

        let mut r = self.r_initial;
        (f)(&mut r, dx, dy);

        state.client_set.float(id, r)?;
        x.position_client(id, r)?;

        Ok(())
    }
}

trait ClickWrapper {
    fn data(&mut self) -> &mut Option<ClickData>;

    fn motion_fn(&self) -> impl Fn(&mut Rect, i32, i32);

    fn on_mouse_event<X: XConn>(
        &mut self,
        evt: &MouseEvent,
        state: &mut State<X>,
        x: &X,
    ) -> Result<()> {
        let id = evt.data.id;

        match evt.kind {
            MouseEventKind::Press => {
                let r_client = x.client_geometry(id)?;
                state.client_set.float(id, r_client)?;
                *self.data() = Some(ClickData {
                    x_initial: evt.data.rpt.x as i32,
                    y_initial: evt.data.rpt.y as i32,
                    r_initial: r_client,
                });
            }

            MouseEventKind::Release => *self.data() = None,
        }

        Ok(())
    }

    fn on_motion<X: XConn>(
        &mut self,
        evt: &MotionNotifyEvent,
        state: &mut State<X>,
        x: &X,
    ) -> Result<()> {
        match *self.data() {
            Some(data) => data.on_motion(self.motion_fn(), evt.data.id, evt.data.rpt, state, x),
            None => Err(custom_error!("mouse motion without held state")),
        }
    }
}

/// A simple mouse event handler for dragging a window
#[derive(Debug, Default, Clone)]
pub struct MouseDragHandler {
    data: Option<ClickData>,
}

impl MouseDragHandler {
    /// Construct a boxed [MouseEventHandler] trait object ready to be added to your bindings
    pub fn boxed_default<X: XConn>() -> Box<dyn MouseEventHandler<X>> {
        Box::<MouseDragHandler>::default()
    }
}

impl ClickWrapper for MouseDragHandler {
    fn data(&mut self) -> &mut Option<ClickData> {
        &mut self.data
    }

    fn motion_fn(&self) -> impl Fn(&mut Rect, i32, i32) {
        |r, dx, dy| r.reposition(dx, dy)
    }
}

impl<X: XConn> MouseEventHandler<X> for MouseDragHandler {
    fn on_mouse_event(&mut self, evt: &MouseEvent, state: &mut State<X>, x: &X) -> Result<()> {
        ClickWrapper::on_mouse_event(self, evt, state, x)
    }

    fn on_motion(&mut self, evt: &MotionNotifyEvent, state: &mut State<X>, x: &X) -> Result<()> {
        ClickWrapper::on_motion(self, evt, state, x)
    }
}

/// A simple mouse event handler for resizing a window
#[derive(Debug, Default, Clone)]
pub struct MouseResizeHandler {
    data: Option<ClickData>,
}

impl MouseResizeHandler {
    /// Construct a boxed [MouseEventHandler] trait object ready to be added to your bindings
    pub fn boxed_default<X: XConn>() -> Box<dyn MouseEventHandler<X>> {
        Box::<MouseResizeHandler>::default()
    }
}

impl ClickWrapper for MouseResizeHandler {
    fn data(&mut self) -> &mut Option<ClickData> {
        &mut self.data
    }

    fn motion_fn(&self) -> impl Fn(&mut Rect, i32, i32) {
        |r, dw, dh| r.resize(dw, dh)
    }
}

impl<X: XConn> MouseEventHandler<X> for MouseResizeHandler {
    fn on_mouse_event(&mut self, evt: &MouseEvent, state: &mut State<X>, x: &X) -> Result<()> {
        ClickWrapper::on_mouse_event(self, evt, state, x)
    }

    fn on_motion(&mut self, evt: &MotionNotifyEvent, state: &mut State<X>, x: &X) -> Result<()> {
        ClickWrapper::on_motion(self, evt, state, x)
    }
}
