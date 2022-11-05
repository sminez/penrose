//! Actions for manipulating floating windows.
use crate::{
    builtin::actions::{key_handler, modify_with},
    core::bindings::KeyEventHandler,
    x::{XConn, XConnExt},
};

/// Resize a currently floating window by a given (width, height) delta
///
/// Screen coordinates are 0-indexed from the top left corner of the sceen.
pub fn resize<X: XConn>(dw: i32, dh: i32) -> Box<dyn KeyEventHandler<X>> {
    modify_with(move |cs| {
        let id = match cs.current_client() {
            Some(&id) => id,
            None => return,
        };

        if let Some(r) = cs.floating.get_mut(&id) {
            r.apply_as_rect(&cs.screens.focus.r, |mut r| {
                r.resize(dw, dh);
                r
            });
        }
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

        if let Some(r) = cs.floating.get_mut(&id) {
            r.apply_as_rect(&cs.screens.focus.r, |mut r| {
                r.reposition(dx, dy);
                r
            });
        }
    })
}

/// Move the currently focused windo to the floating layer in its current on screen position
pub fn float_focused<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let id = match state.client_set.current_client() {
            Some(&id) => id,
            None => return Ok(()),
        };

        let r = x.client_geometry(id)?;

        x.modify_and_refresh(state, |cs| cs.float_unchecked(id, r))
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

/// Float all windows in their current tiled position
pub fn float_all<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let positions = state.client_set.visible_client_positions();

        x.modify_and_refresh(state, |cs| {
            for &(c, r) in positions.iter() {
                cs.float_unchecked(c, r);
            }
        })
    })
}

/// Sink all floating windows back into their tiled positions
pub fn sink_all<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    modify_with(|cs| cs.floating.clear())
}
