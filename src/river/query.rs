//! Queries against windows, for a river config's manage hooks.
//!
//! The X11 counterparts in [x::query][crate::x::query] read window properties, which river has
//! none of: what a window calls itself arrives as `app_id` and `title` events instead. So
//! [AppId] is the river spelling of `ClassName` and [Title] the river spelling of `Title`, and
//! there is no counterpart to `AppName` -- river has no separate instance name -- or to
//! `StringProperty`, which has no meaning here at all.
use crate::{
    Result,
    core::conn::{Conn, Query, WinId},
    river::RiverConn,
};

/// A [Query] matching a window's `app_id`, which is Wayland's answer to `WM_CLASS`.
///
/// `alacritty --class NAME` sets it under Wayland too, so class-based placement carries over
/// from an X11 config unaltered — but only the *class* half of it: X11's two strings are one
/// here, so a rule written against the instance name has nothing to match.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AppId(pub &'static str);

impl Query<RiverConn> for AppId {
    fn run(&self, id: WinId, conn: &mut RiverConn) -> Result<bool> {
        Ok(conn.window_app_id(id).as_deref() == Some(self.0))
    }
}

/// A [Query] matching a window's title.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Title(pub &'static str);

impl Query<RiverConn> for Title {
    fn run(&self, id: WinId, conn: &mut RiverConn) -> Result<bool> {
        Ok(conn.client_title(id)? == self.0)
    }
}

/// A [Query] matching a window which has a parent: a dialog, a file picker or similar.
///
/// This is as close as river gets to X11's `_NET_WM_WINDOW_TYPE`, which it has no counterpart
/// for. River's own advice for a window with a parent is that it "should generally be rendered
/// directly above" it, which is the same set of windows a config usually wants to float.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IsChild;

impl Query<RiverConn> for IsChild {
    fn run(&self, id: WinId, conn: &mut RiverConn) -> Result<bool> {
        Ok(conn.client_transient_parent(id).is_some())
    }
}
