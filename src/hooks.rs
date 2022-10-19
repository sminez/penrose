//! Traits for writing and composing hooks
use crate::{
    core::{ClientSet, State},
    x::{XConn, XEvent},
    Result, Xid,
};

/// Handle an [XEvent], return `true` if default event handling should be run afterwards.
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

/// Action to run when a new client becomes managed
pub trait ManageHook<X>
where
    X: XConn,
{
    /// Run this hook
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X) -> Result<()>;

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
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X) -> Result<()> {
        self.first.call(client, cs, x)?;
        self.second.call(client, cs, x)
    }
}

impl<F, X> ManageHook<X> for F
where
    F: FnMut(Xid, &mut ClientSet, &X) -> Result<()>,
    X: XConn,
{
    fn call(&mut self, client: Xid, cs: &mut ClientSet, x: &X) -> Result<()> {
        (self)(client, cs, x)
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
