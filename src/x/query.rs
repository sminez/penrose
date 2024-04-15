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
pub struct AndQuery<Q1, Q2>(pub Q1, pub Q2);

impl<X: XConn, Q1: Query<X>, Q2: Query<X>> Query<X> for AndQuery<Q1, Q2> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(self.0.run(id, x)? && self.1.run(id, x)?)
    }
}

/// A meta [Query] for combining two queries with a logical OR.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrQuery<Q1, Q2>(pub Q1, pub Q2);

impl<X: XConn, Q1: Query<X>, Q2: Query<X>> Query<X> for OrQuery<Q1, Q2> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(self.0.run(id, x)? || self.1.run(id, x)?)
    }
}

/// A meta [Query] for applying a logical NOT to a query.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NotQuery<Q>(pub Q);

impl<X: XConn, Q: Query<X>> Query<X> for NotQuery<Q> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        Ok(!self.0.run(id, x)?)
    }
}

/// A meta [Query] for combining multiple queries with a logical OR.
pub struct AnyQuery<X>(pub Vec<Box<dyn Query<X>>>);

impl<X: XConn> fmt::Debug for AnyQuery<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyQuery").finish()
    }
}

impl<X: XConn> Query<X> for AnyQuery<X> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        self.0
            .iter()
            .try_fold(false, |acc, query| Ok(acc || query.run(id, x)?))
    }
}

/// A meta [Query] for combining multiple queries with a logical AND.
pub struct AllQuery<X>(pub Vec<Box<dyn Query<X>>>);

impl<X: XConn> fmt::Debug for AllQuery<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AllQuery").finish()
    }
}

impl<X: XConn> Query<X> for AllQuery<X> {
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        self.0
            .iter()
            .try_fold(true, |acc, query| Ok(acc && query.run(id, x)?))
    }
}

trait QueryExt<X>: Query<X>
where
    X: XConn,
{
    fn and(self, other: impl Query<X>) -> impl Query<X>
    where
        Self: Sized,
    {
        AndQuery(self, other)
    }

    fn or(self, other: impl Query<X>) -> impl Query<X>
    where
        Self: Sized,
    {
        OrQuery(self, other)
    }

    fn not(self) -> impl Query<X>
    where
        Self: Sized,
    {
        NotQuery(self)
    }
}

impl<X, Q> QueryExt<X> for Q
where
    X: XConn,
    Q: Query<X>,
{
}

