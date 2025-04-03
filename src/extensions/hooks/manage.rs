//! Manage hooks for common manage actions
//!
//! Manage hooks should _not_ trigger a refresh directly: that is handled by penrose
//! itself when the manage hook is called.
use crate::{
    core::{
        conn::{Conn, Query},
        hooks::ManageHook,
        State,
    },
    pure::geometry::{Rect, RelativeRect},
    Result, WinId,
};

// A tuple of (query, manage hook) runs conditionally if the query holds
// for the window being managed.
impl<C, Q, H> ManageHook<C> for (Q, H)
where
    C: Conn,
    Q: Query<C>,
    H: ManageHook<C>,
{
    fn call(&mut self, id: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        if self.0.run(id, conn)? {
            self.1.call(id, state, conn)?;
        }

        Ok(())
    }
}

fn float<C: Conn>(client: WinId, r: Rect, state: &mut State<C>, _: &C) -> Result<()> {
    state.client_set.float(client, r)
}

/// Perform no additional actions when managing a new client.
#[derive(Debug)]
pub struct DefaultTiled;
impl<C: Conn> ManageHook<C> for DefaultTiled {
    fn call(&mut self, _client: WinId, _state: &mut State<C>, _: &C) -> Result<()> {
        Ok(())
    }
}

/// Float clients at a fixed position on the screen.
#[derive(Debug)]
pub struct FloatingFixed(pub Rect);
impl<C: Conn> ManageHook<C> for FloatingFixed {
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        float(client, self.0, state, conn)
    }
}

/// Float clients in the center of the screen.
#[derive(Debug)]
pub struct FloatingCentered {
    w: f64,
    h: f64,
}

impl FloatingCentered {
    /// Create a new [FloatingCentered] with the given width and height ratios.
    ///
    /// # Panics
    /// Panics if `w` or `h` are not in the range `0.0..=1.0`.
    pub fn new(w: f64, h: f64) -> Self {
        if !((0.0..=1.0).contains(&w) && (0.0..=1.0).contains(&h)) {
            panic!("w and h must be between 0.0 and 1.0: got w={w}, h={h}")
        }

        Self { w, h }
    }
}

impl<C: Conn> ManageHook<C> for FloatingCentered {
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        let r_screen = &state.client_set.screens.focus.r;
        let r = r_screen
            .scale_h(self.h)
            .scale_w(self.w)
            .centered_in(r_screen)
            .expect("bounds checks in FloatingCentered::new to be upheld");

        float(client, r, state, conn)
    }
}

/// Float clients at a relative position to the current screen.
#[derive(Debug)]
pub struct FloatingRelative(pub RelativeRect);
impl FloatingRelative {
    /// Create a new [FloatingRelative] with the given x, y, width and height ratios.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self(RelativeRect::new(x, y, w, h))
    }
}

impl<C: Conn> ManageHook<C> for FloatingRelative {
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        let r_screen = &state.client_set.screens.focus.r;
        let r = self.0.applied_to(r_screen);

        float(client, r, state, conn)
    }
}

/// Move the specified client to the named workspace.
#[derive(Debug)]
pub struct SetWorkspace(pub &'static str);
impl<C: Conn> ManageHook<C> for SetWorkspace {
    fn call(&mut self, client: WinId, state: &mut State<C>, _: &C) -> Result<()> {
        state.client_set.move_client_to_tag(&client, self.0);
        Ok(())
    }
}
