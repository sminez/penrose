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
//! to the pure WindowManager internal [`State`] and the [`XConn`] in order to run
//! any set up code you need which requires the the bindings to already have been
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
//! `true`, to skip the handling (and prevent the normal behviour for such an event) you
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
//! >           They are already called by the XConn immediately before refreshing so all
//! >           triggering a refresh directly will do is run the refresh twice: once with
//! >           the inital state of the client before your hook was applied and once after.
//!
//! ### Layout Hooks
//!
//! Finally we have [`LayoutHook`]s which operate a little differently, in that they have
//! two methods to implement. Layout hooks are run _around_ whatever [Layout][1] is active
//! for the focused workspace, allowing you to modify the screen dimensions available for the
//! layout algorithm before it runs and editing the list of window positions it generates
//! before they are applied. This lets you do things like prevent windows being positions on
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
//! In this case however, your hook will be run each time the XConn refreshes the X state in
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
    core::{layout::LayoutTransformer, State},
    pure::geometry::Rect,
    x::{XConn, XEvent},
    Result, Xid,
};
use std::fmt;

/// Handle an [XEvent], return `true` if default event handling should be run afterwards.
///
/// This hook is called before incoming XEvents are processed by the default event handling
/// logic.
pub trait EventHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool>;

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn EventHook<X>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with another [EventHook]. The second hook will be skipped if this one
    /// returns `false`.
    fn then<H>(self, next: H) -> ComposedEventHook<X>
    where
        H: EventHook<X> + 'static,
        Self: Sized + 'static,
    {
        ComposedEventHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Compose this hook with a boxed [EventHook]. The second hook will be skipped if this one
    /// returns `false`.
    fn then_boxed(self, next: Box<dyn EventHook<X>>) -> Box<dyn EventHook<X>>
    where
        Self: Sized + 'static,
        X: 'static,
    {
        Box::new(ComposedEventHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<X: XConn> fmt::Debug for Box<dyn EventHook<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventHook").finish()
    }
}

/// The result of composing two event hooks using `then`
#[derive(Debug)]
pub struct ComposedEventHook<X>
where
    X: XConn,
{
    first: Box<dyn EventHook<X>>,
    second: Box<dyn EventHook<X>>,
}

impl<X> EventHook<X> for ComposedEventHook<X>
where
    X: XConn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
        if self.first.call(event, state, x)? {
            self.second.call(event, state, x)
        } else {
            Ok(false)
        }
    }
}

impl<F, X> EventHook<X> for F
where
    F: FnMut(&XEvent, &mut State<X>, &X) -> Result<bool>,
    X: XConn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
        (self)(event, state, x)
    }
}

/// Action to run when a new client becomes managed.
///
/// Manage hooks should _not_ trigger refreshes of state directly: they are called
/// immediately before a refresh is run by main window manager logic.
pub trait ManageHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, client: Xid, state: &mut State<X>, x: &X) -> Result<()>;

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn ManageHook<X>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with another [ManageHook].
    fn then<H>(self, next: H) -> ComposedManageHook<X>
    where
        H: ManageHook<X> + 'static,
        Self: Sized + 'static,
    {
        ComposedManageHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Compose this hook with a boxed [ManageHook].
    fn then_boxed(self, next: Box<dyn ManageHook<X>>) -> Box<dyn ManageHook<X>>
    where
        Self: Sized + 'static,
        X: 'static,
    {
        Box::new(ComposedManageHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<X: XConn> fmt::Debug for Box<dyn ManageHook<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManageHook").finish()
    }
}

/// The result of composing two manage hooks using `then`
#[derive(Debug)]
pub struct ComposedManageHook<X>
where
    X: XConn,
{
    first: Box<dyn ManageHook<X>>,
    second: Box<dyn ManageHook<X>>,
}

impl<X> ManageHook<X> for ComposedManageHook<X>
where
    X: XConn,
{
    fn call(&mut self, client: Xid, state: &mut State<X>, x: &X) -> Result<()> {
        self.first.call(client, state, x)?;
        self.second.call(client, state, x)
    }
}

impl<F, X> ManageHook<X> for F
where
    F: FnMut(Xid, &mut State<X>, &X) -> Result<()>,
    X: XConn,
{
    fn call(&mut self, client: Xid, state: &mut State<X>, x: &X) -> Result<()> {
        (self)(client, state, x)
    }
}

/// An arbitrary action that can be run and modify [State]
pub trait StateHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()>;

    /// Compose this hook with another [StateHook].
    fn then<H>(self, next: H) -> ComposedStateHook<X>
    where
        H: StateHook<X> + 'static,
        Self: Sized + 'static,
    {
        ComposedStateHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn StateHook<X>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with a boxed [StateHook].
    fn then_boxed(self, next: Box<dyn StateHook<X>>) -> Box<dyn StateHook<X>>
    where
        Self: Sized + 'static,
        X: 'static,
    {
        Box::new(ComposedStateHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<X: XConn> fmt::Debug for Box<dyn StateHook<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateHook").finish()
    }
}

/// The result of composing two state hooks using `then`
#[derive(Debug)]
pub struct ComposedStateHook<X>
where
    X: XConn,
{
    first: Box<dyn StateHook<X>>,
    second: Box<dyn StateHook<X>>,
}

impl<X> StateHook<X> for ComposedStateHook<X>
where
    X: XConn,
{
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        self.first.call(state, x)?;
        self.second.call(state, x)
    }
}

impl<F, X> StateHook<X> for F
where
    F: FnMut(&mut State<X>, &X) -> Result<()>,
    X: XConn,
{
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        (self)(state, x)
    }
}

/// Logic to run before and after laying out clients
pub trait LayoutHook<X>
where
    X: XConn,
{
    #[allow(unused_variables)]
    /// Optionally modify the screen dimensions being given to a [Layout][crate::core::layout::Layout]
    fn transform_initial(&mut self, r: Rect, state: &State<X>, x: &X) -> Rect {
        r
    }

    #[allow(unused_variables)]
    /// Optionally modify the client positions returned by a [Layout][crate::core::layout::Layout]
    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(Xid, Rect)>,
        state: &State<X>,
        x: &X,
    ) -> Vec<(Xid, Rect)> {
        positions
    }

    /// Compose this hook with another [LayoutHook].
    fn then<H>(self, next: H) -> ComposedLayoutHook<X>
    where
        H: LayoutHook<X> + 'static,
        Self: Sized + 'static,
    {
        ComposedLayoutHook {
            first: Box::new(self),
            second: Box::new(next),
        }
    }

    /// Convert to a trait object
    fn boxed(self) -> Box<dyn LayoutHook<X>>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Compose this hook with a boxed [LayoutHook].
    fn then_boxed(self, next: Box<dyn LayoutHook<X>>) -> Box<dyn LayoutHook<X>>
    where
        Self: Sized + 'static,
        X: 'static,
    {
        Box::new(ComposedLayoutHook {
            first: Box::new(self),
            second: next,
        })
    }
}

impl<X: XConn> fmt::Debug for Box<dyn LayoutHook<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutHook").finish()
    }
}

/// The result of composing two state hooks using `then`
#[derive(Debug)]
pub struct ComposedLayoutHook<X>
where
    X: XConn,
{
    first: Box<dyn LayoutHook<X>>,
    second: Box<dyn LayoutHook<X>>,
}

impl<X> LayoutHook<X> for ComposedLayoutHook<X>
where
    X: XConn,
{
    fn transform_initial(&mut self, r: Rect, state: &State<X>, x: &X) -> Rect {
        self.second
            .transform_initial(self.first.transform_initial(r, state, x), state, x)
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(Xid, Rect)>,
        state: &State<X>,
        x: &X,
    ) -> Vec<(Xid, Rect)> {
        self.second.transform_positions(
            r,
            self.first.transform_positions(r, positions, state, x),
            state,
            x,
        )
    }
}

impl<F, G, X> LayoutHook<X> for (F, G)
where
    F: FnMut(Rect, &State<X>, &X) -> Rect,
    G: FnMut(Rect, Vec<(Xid, Rect)>, &State<X>, &X) -> Vec<(Xid, Rect)>,
    X: XConn,
{
    fn transform_initial(&mut self, r: Rect, state: &State<X>, x: &X) -> Rect {
        (self.0)(r, state, x)
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(Xid, Rect)>,
        state: &State<X>,
        x: &X,
    ) -> Vec<(Xid, Rect)> {
        (self.1)(r, positions, state, x)
    }
}

impl<T, X> LayoutHook<X> for T
where
    T: LayoutTransformer,
    X: XConn,
{
    fn transform_initial(&mut self, r: Rect, _: &State<X>, _: &X) -> Rect {
        LayoutTransformer::transform_initial(self, r)
    }

    fn transform_positions(
        &mut self,
        r: Rect,
        positions: Vec<(Xid, Rect)>,
        _: &State<X>,
        _: &X,
    ) -> Vec<(Xid, Rect)> {
        LayoutTransformer::transform_positions(self, r, positions)
    }
}
