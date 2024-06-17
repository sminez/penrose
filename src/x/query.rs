//! Queries against client windows
use crate::{
    x::{atom::Atom, property::Prop, XConn},
    Result, Xid,
};
use std::fmt;

/// A query to be run against client windows for identifying specific windows
/// or programs.
pub trait Query<X: XConn> {
    /// Run this query for a given window ID.
    fn run(&self, id: Xid, x: &X) -> Result<bool>;

    /// Combine this query with another query using a logical AND.
    fn and<Other: Query<X>>(self, other: Other) -> AndQuery<X, Self, Other>
    where
        Self: Sized,
    {
        AndQuery {
            first: self,
            second: other,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Combine this query with another query using a logical OR.
    fn or<Other: Query<X>>(self, other: Other) -> OrQuery<X, Self, Other>
    where
        Self: Sized,
    {
        OrQuery {
            first: self,
            second: other,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Apply a logical NOT to this query.
    fn not(self) -> NotQuery<X, Self>
    where
        Self: Sized,
    {
        NotQuery {
            inner: self,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<X: XConn> fmt::Debug for Box<dyn Query<X>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Query").finish()
    }
}

pub(super) fn str_prop<X>(prop: impl AsRef<str>, id: Xid, x: &X) -> Result<Option<Vec<String>>>
where
    X: XConn,
{
    match x.get_prop(id, prop.as_ref())? {
        Some(Prop::UTF8String(strs)) if !strs.is_empty() => Ok(Some(strs)),
        _ => Ok(None),
    }
}

/// A [Query] for fetching a window's title following ICCCM / EWMH standards.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Title(pub &'static str);

impl<X> Query<X> for Title
where
    X: XConn,
{
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        let strs = str_prop(Atom::WmName, id, x)
            .ok()
            .or_else(|| str_prop(Atom::NetWmName, id, x).ok())
            .flatten();

        match strs {
            Some(strs) if !strs.is_empty() => Ok(strs[0] == self.0),
            _ => Ok(false),
        }
    }
}

/// A [Query] for fetching a window's application name (the first string returned
/// under the WM_CLASS property).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AppName(pub &'static str);

impl<X> Query<X> for AppName
where
    X: XConn,
{
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        match str_prop(Atom::WmClass, id, x)? {
            Some(strs) if !strs.is_empty() => Ok(strs[0] == self.0),
            _ => Ok(false),
        }
    }
}

/// A [Query] for fetching a window's class name (the second string returned
/// under the WM_CLASS property).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClassName(pub &'static str);

impl<X> Query<X> for ClassName
where
    X: XConn,
{
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        match str_prop(Atom::WmClass, id, x)? {
            Some(strs) if strs.len() > 1 => Ok(strs[1] == self.0),
            _ => Ok(false),
        }
    }
}

/// A [Query] for fetching a string property from a client window.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StringProperty(pub &'static str, pub &'static str);

impl<X> Query<X> for StringProperty
where
    X: XConn,
{
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        match str_prop(self.0, id, x)? {
            Some(strs) if !strs.is_empty() => Ok(strs[0] == self.1),
            _ => Ok(false),
        }
    }
}

/// A meta [Query] for combining two queries with a logical AND.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AndQuery<X: XConn, Q1, Q2> {
    first: Q1,
    second: Q2,
    _phantom: std::marker::PhantomData<X>,
}

impl<X: XConn, Q1: Query<X>, Q2: Query<X>> Query<X> for AndQuery<X, Q1, Q2> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(self.first.run(id, x)? && self.second.run(id, x)?)
    }
}

/// A meta [Query] for combining two queries with a logical OR.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrQuery<X: XConn, Q1, Q2> {
    first: Q1,
    second: Q2,
    _phantom: std::marker::PhantomData<X>,
}

impl<X: XConn, Q1: Query<X>, Q2: Query<X>> Query<X> for OrQuery<X, Q1, Q2> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(self.first.run(id, x)? || self.second.run(id, x)?)
    }
}

/// A meta [Query] for applying a logical NOT to a query.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NotQuery<X: XConn, Q> {
    inner: Q,
    _phantom: std::marker::PhantomData<X>,
}

impl<X: XConn, Q: Query<X>> Query<X> for NotQuery<X, Q> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(!self.inner.run(id, x)?)
    }
}
