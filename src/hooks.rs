//! Traits for writing and composing hooks
use crate::{
    core::{ClientSet, State},
    x::{XConn, XEvent},
    Xid,
};

/// Handle an [XEvent], return `true` if default event handling should be run afterwards.
pub trait EventHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> bool;

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
}

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
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> bool {
        if self.first.call(event, state, x) {
            self.second.call(event, state, x)
        } else {
            false
        }
    }
}

impl<F, X> EventHook<X> for F
where
    F: FnMut(&XEvent, &mut State<X>, &X) -> bool,
    X: XConn,
{
    fn call(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> bool {
        (self)(event, state, x)
    }
}

/// Action to run when a new client becomes managed
pub trait ManageHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X);

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
}

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
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X) {
        self.first.call(client, cs, x);
        self.second.call(client, cs, x);
    }
}

impl<F, X> ManageHook<X> for F
where
    F: FnMut(Xid, &mut ClientSet, &X),
    X: XConn,
{
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X) {
        (self)(client, cs, x)
    }
}

/// An arbitrary action that can be run and modify [State]
pub trait StateHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, state: &mut State<X>, x: &X);

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
}

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
    fn call(&mut self, state: &mut State<X>, x: &X) {
        self.first.call(state, x);
        self.second.call(state, x);
    }
}

impl<F, X> StateHook<X> for F
where
    F: FnMut(&mut State<X>, &X),
    X: XConn,
{
    fn call(&mut self, state: &mut State<X>, x: &X) {
        (self)(state, x)
    }
}
