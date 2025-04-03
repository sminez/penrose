//! Traits for writing and composing hooks.
//!
//! ## Hook points
//!
//! Penrose offers several different hook points where you are able to provide custom
//! logic to execute as part of the main WindowManager event loop. Unlike logic you
//! add as KeyEventHandlers, hooks will be run automatically by Penrose as and when
//! the conditions for their execution arises. Each hook point requires a specific
//! trait to be implemented and in the simplest case, functions with the correct
//! type signature can be used directly (though you will likely want to implement
//! traits directly if you are looking for more control over how your hook logic is
//! run.
//!
//!
//! ### Startup Hooks
//!
//! Startup hooks are implemented using the [`StateHook`] trait, allowing you access
//! to the pure WindowManager internal [`State`] and the [`Conn`] in order to run
//! any set up code you need which requires the bindings to already have been
//! grabbed but before any existing clients are parsed and managed by the WindowManager.
//!
//! > **NOTE**: Startup hooks are run to completion before entering the main event loop.
//!
//! ### Event Hooks
//!
//! The [`EventHook`] trait allows you to pre-process incoming [`XEvent`]s as they
//! arrive from the X server, _before_ they are seen by the default event handling logic.
//! This allows you to intercept or modify incoming events as you need and act
//! accordingly. Maybe you want to keep track of changes to a specific property on clients
//! or maybe you want to know if a specific client is being destroyed.
//!
//! This hook returns a `bool` indicating whether or not the default event handling logic
//! needs to run after your hook has finished: to run the default handling you should return
//! `true`, to skip the handling (and prevent the normal behaviour for such an event) you
//! can return `false`.
//!
//! > **NOTE**: Be careful about disabling default event handling! If you drop events
//! >           that are required for the normal behaviour of the WindowManager then you
//! >           will need to make sure that you track and maintain any required state
//! >           that may now be missing.
//!
//! ### Manage Hooks
//!
//! [`ManageHook`]s let you run some additional logic to optionally modify the pure
//! window manager state _after_ a newly managed client has been processed and stored, but
//! before that change is applied to the X server. This allows you to modify how the new
//! client is set up when it first appears, such as moving it to a specific workspace or
//! marking it as floating in a specific position. There are some reference hooks in the
//! [extensions module][0] that can serve as a starting point for looking at the sorts of
//! things that are possible.
//!
//! > **NOTE**: ManageHooks should _not_ directly trigger a refresh of the X state!
//! >           They are already called by the Conn immediately before refreshing so all
//! >           triggering a refresh directly will do is run the refresh twice: once with
//! >           the initial state of the client before your hook was applied and once after.
//!
//! ### Layout Hooks
//!
//! Next we have [`LayoutHook`]s which operate a little differently, in that they have
//! two methods to implement. Layout hooks are run _around_ whatever [Layout][1] is active
//! for the focused workspace, allowing you to modify the screen dimensions available for the
//! layout algorithm before it runs and editing the list of window positions it generates
//! before they are applied. This lets you do things like prevent windows being positioned on
//! certain parts of the screen, or injecting/removing additional window positions.
//!
//! This is somewhat similar to the [`LayoutTransformer`] trait which is a wrapper around a
//! specific Layout, but it doesn't allow for introspection of the underlying Layout or
//! responding to Messages. On the plus side, layout hooks are registered and run centrally
//! rather than needing to be applied to each Layout you want to add that behaviour to.
//!
//! ### Refresh Hooks
//!
//! Refresh hooks are implemented using the same [`StateHook`] trait used for Startup hooks.
//! In this case however, your hook will be run each time the Conn refreshes the X state in
//! response to changes being made to the internal state of the WindowManager.
//! This is one of the more general purpose hooks available for you to make use of and can be
//! used to run code any time something changes in the internal state of your window manager.
//!
//! ## Setting and composing hooks
//!
//! Each kind of hook has a corresponding `compose_or_set_*_hook` method on the [Config][2]
//! struct. If multiple hooks of the same type are registered they are composed together as
//! a stack, with the most recently added hook running first (keep this in mind if the hooks
//! you are registering have any potential interactions in how they operate).
//!
//!   [0]: crate::extensions::hooks::manage
//!   [1]: crate::core::layout::Layout
//!   [2]: crate::core::Config

use crate::{
    core::{
        conn::{Conn, WinId},
        layout::LayoutTransformer,
        State,
    },
    pure::geometry::Rect,
    x::XEvent,
    Result,
};
use std::fmt;

/// Handle an [XEvent], return `true` if default event handling should be run afterwards.
///
/// This hook is called before incoming XEvents are processed by the default event handling
/// logic.
pub trait EventHook<C>
where
    C: Conn,
{
    /// Run this hook
    fn call(&mut self, event: &XEvent, state: &mut State<C>, conn: &C) -> Result<bool>;

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn EventHook<C>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with another [EventHook]. The second hook will be skipped if this one
    /// returns `false`.
    fn then<H>(self, next: H) -> ComposedEventHook<C>
    where
        H: EventHook<C> + 'static,
        Self: Sized + 'static,
    {
        ComposedEventHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Compose this hook with a boxed [EventHook]. The second hook will be skipped if this one
    /// returns `false`.
    fn then_boxed(self, next: Box<dyn EventHook<C>>) -> Box<dyn EventHook<C>>
    where
        Self: Sized + 'static,
        C: 'static,
    {
        Box::new(ComposedEventHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<C> EventHook<C> for Vec<Box<dyn EventHook<C>>>
where
    C: Conn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<C>, conn: &C) -> Result<bool> {
        let mut call_next = true;
        for hook in self.iter_mut() {
            call_next = hook.call(event, state, conn)?;
            if !call_next {
                return Ok(false);
            }
        }

        Ok(call_next)
    }
}

impl<C: Conn> fmt::Debug for Box<dyn EventHook<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventHook").finish()
    }
}

/// The result of composing two event hooks using `then`
#[derive(Debug)]
pub struct ComposedEventHook<C>
where
    C: Conn,
{
    first: Box<dyn EventHook<C>>,
    second: Box<dyn EventHook<C>>,
}

impl<C> EventHook<C> for ComposedEventHook<C>
where
    C: Conn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<C>, conn: &C) -> Result<bool> {
        if self.first.call(event, state, conn)? {
            self.second.call(event, state, conn)
        } else {
            Ok(false)
        }
    }
}

impl<F, C> EventHook<C> for F
where
    F: FnMut(&XEvent, &mut State<C>, &C) -> Result<bool>,
    C: Conn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<C>, conn: &C) -> Result<bool> {
        (self)(event, state, conn)
    }
}

/// Action to run when a new client becomes managed.
///
/// Manage hooks should _not_ trigger refreshes of state directly: they are called
/// immediately before a refresh is run by main window manager logic.
pub trait ManageHook<C>
where
    C: Conn,
{
    /// Run this hook
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()>;

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn ManageHook<C>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with another [ManageHook].
    fn then<H>(self, next: H) -> ComposedManageHook<C>
    where
        H: ManageHook<C> + 'static,
        Self: Sized + 'static,
    {
        ComposedManageHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Compose this hook with a boxed [ManageHook].
    fn then_boxed(self, next: Box<dyn ManageHook<C>>) -> Box<dyn ManageHook<C>>
    where
        Self: Sized + 'static,
        C: 'static,
    {
        Box::new(ComposedManageHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<C> ManageHook<C> for Vec<Box<dyn ManageHook<C>>>
where
    C: Conn,
{
    fn call(&mut self, id: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        for hook in self.iter_mut() {
            hook.call(id, state, conn)?;
        }

        Ok(())
    }
}

impl<C: Conn> fmt::Debug for Box<dyn ManageHook<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManageHook").finish()
    }
}

/// The result of composing two manage hooks using `then`
#[derive(Debug)]
pub struct ComposedManageHook<C>
where
    C: Conn,
{
    first: Box<dyn ManageHook<C>>,
    second: Box<dyn ManageHook<C>>,
}

impl<C> ManageHook<C> for ComposedManageHook<C>
where
    C: Conn,
{
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        self.first.call(client, state, conn)?;
        self.second.call(client, state, conn)
    }
}

impl<F, C> ManageHook<C> for F
where
    F: FnMut(WinId, &mut State<C>, &C) -> Result<()>,
    C: Conn,
{
    fn call(&mut self, client: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
        (self)(client, state, conn)
    }
}

/// An arbitrary action that can be run and modify [State]
pub trait StateHook<C>
where
    C: Conn,
{
    /// Run this hook
    fn call(&mut self, state: &mut State<C>, conn: &C) -> Result<()>;

    /// Compose this hook with another [StateHook].
    fn then<H>(self, next: H) -> ComposedStateHook<C>
    where
        H: StateHook<C> + 'static,
        Self: Sized + 'static,
    {
        ComposedStateHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn StateHook<C>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with a boxed [StateHook].
    fn then_boxed(self, next: Box<dyn StateHook<C>>) -> Box<dyn StateHook<C>>
    where
        Self: Sized + 'static,
        C: 'static,
    {
        Box::new(ComposedStateHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<C> StateHook<C> for Vec<Box<dyn StateHook<C>>>
where
    C: Conn,
{
    fn call(&mut self, state: &mut State<C>, conn: &C) -> Result<()> {
        for hook in self.iter_mut() {
            hook.call(state, conn)?;
        }

        Ok(())
    }
}

impl<C: Conn> fmt::Debug for Box<dyn StateHook<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateHook").finish()
    }
}

/// The result of composing two state hooks using `then`
#[derive(Debug)]
pub struct ComposedStateHook<C>
where
    C: Conn,
{
    first: Box<dyn StateHook<C>>,
    second: Box<dyn StateHook<C>>,
}

impl<C> StateHook<C> for ComposedStateHook<C>
where
    C: Conn,
{
    fn call(&mut self, state: &mut State<C>, conn: &C) -> Result<()> {
        self.first.call(state, conn)?;
        self.second.call(state, conn)
    }
}

impl<F, C> StateHook<C> for F
where
    F: FnMut(&mut State<C>, &C) -> Result<()>,
    C: Conn,
{
    fn call(&mut self, state: &mut State<C>, conn: &C) -> Result<()> {
        (self)(state, conn)
    }
}

/// Logic to run before and after laying out clients
pub trait LayoutHook<C>
where
    C: Conn,
{
    #[allow(unused_variables)]
    /// Optionally modify the screen dimensions being given to a
    /// [Layout][crate::core::layout::Layout] on a particular screen index.
    ///
    /// By default this just calls through to [LayoutHook::transform_initial].
    fn transform_initial_for_screen(
        &mut self,
        screen_index: usize,
        r: Rect,
        state: &State<C>,
        conn: &C,
    ) -> Rect {
        self.transform_initial(r, state, conn)
    }

    #[allow(unused_variables)]
    /// Optionally modify the screen dimensions being given to a [Layout][crate::core::layout::Layout]
    fn transform_initial(&mut self, r: Rect, state: &State<C>, conn: &C) -> Rect {
        r
    }

    #[allow(unused_variables)]
    /// Optionally modify the client positions returned by a [Layout][crate::core::layout::Layout]
    /// on a particular screen index.
    ///
    /// By default this just calls through to [LayoutHook::transform_positions].
    fn transform_positions_for_screen(
        &mut self,
        screen_index: usize,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        state: &State<C>,
        conn: &C,
    ) -> Vec<(WinId, Rect)> {
        self.transform_positions(r, positions, state, conn)
    }

    #[allow(unused_variables)]
    /// Optionally modify the client positions returned by a [Layout][crate::core::layout::Layout]
    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        state: &State<C>,
        conn: &C,
    ) -> Vec<(WinId, Rect)> {
        positions
    }

    /// Compose this hook with another [LayoutHook].
    fn then<H>(self, next: H) -> ComposedLayoutHook<C>
    where
        H: LayoutHook<C> + 'static,
        Self: Sized + 'static,
    {
        ComposedLayoutHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn LayoutHook<C>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with a boxed [LayoutHook].
    fn then_boxed(self, next: Box<dyn LayoutHook<C>>) -> Box<dyn LayoutHook<C>>
    where
        Self: Sized + 'static,
        C: 'static,
    {
        Box::new(ComposedLayoutHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<C: Conn> fmt::Debug for Box<dyn LayoutHook<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutHook").finish()
    }
}

/// The result of composing two state hooks using `then`
#[derive(Debug)]
pub struct ComposedLayoutHook<C>
where
    C: Conn,
{
    first: Box<dyn LayoutHook<C>>,
    second: Box<dyn LayoutHook<C>>,
}

impl<C> LayoutHook<C> for ComposedLayoutHook<C>
where
    C: Conn,
{
    fn transform_initial_for_screen(
        &mut self,
        screen_index: usize,
        r: Rect,
        state: &State<C>,
        conn: &C,
    ) -> Rect {
        self.second.transform_initial_for_screen(
            screen_index,
            self.first
                .transform_initial_for_screen(screen_index, r, state, conn),
            state,
            conn,
        )
    }

    fn transform_initial(&mut self, r: Rect, state: &State<C>, conn: &C) -> Rect {
        self.second
            .transform_initial(self.first.transform_initial(r, state, conn), state, conn)
    }

    fn transform_positions_for_screen(
        &mut self,
        screen_index: usize,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        state: &State<C>,
        conn: &C,
    ) -> Vec<(WinId, Rect)> {
        self.second.transform_positions_for_screen(
            screen_index,
            r,
            self.first
                .transform_positions_for_screen(screen_index, r, positions, state, conn),
            state,
            conn,
        )
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        state: &State<C>,
        conn: &C,
    ) -> Vec<(WinId, Rect)> {
        self.second.transform_positions(
            r,
            self.first.transform_positions(r, positions, state, conn),
            state,
            conn,
        )
    }
}

impl<F, G, C> LayoutHook<C> for (F, G)
where
    F: FnMut(Rect, &State<C>, &C) -> Rect,
    G: FnMut(Rect, Vec<(WinId, Rect)>, &State<C>, &C) -> Vec<(WinId, Rect)>,
    C: Conn,
{
    fn transform_initial(&mut self, r: Rect, state: &State<C>, conn: &C) -> Rect {
        (self.0)(r, state, conn)
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        state: &State<C>,
        conn: &C,
    ) -> Vec<(WinId, Rect)> {
        (self.1)(r, positions, state, conn)
    }
}

impl<T, C> LayoutHook<C> for T
where
    T: LayoutTransformer,
    C: Conn,
{
    fn transform_initial(&mut self, r: Rect, _: &State<C>, _: &C) -> Rect {
        LayoutTransformer::transform_initial(self, r)
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(WinId, Rect)>,
        _: &State<C>,
        _: &C,
    ) -> Vec<(WinId, Rect)> {
        LayoutTransformer::transform_positions(self, r, positions)
    }
}
