//! Queries against client windows
use crate::{
    x::{atom::Atom, property::Prop, XConn},
    Result, Xid,
};

/// A query to be run against client windows for identifying specific windows
/// or programs.
pub trait Query<X: XConn> {
    fn run(&self, id: Xid, x: &X) -> Result<bool>;
}

fn str_prop<X>(prop: impl AsRef<str>, id: Xid, x: &X) -> Result<Option<Vec<String>>>
where
    X: XConn,
{
    match x.get_prop(id, prop.as_ref())? {
        Some(Prop::UTF8String(strs)) if !strs.is_empty() => Ok(Some(strs)),
        _ => Ok(None),
    }
}

/// A [Query] for fetching a window's title following ICCCM / EWMH standards.
pub struct Title(pub String);

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
pub struct AppName(pub String);

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
pub struct ClassName(pub String);

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
pub struct StringProperty(pub String, pub String);

impl<X> Query<X> for StringProperty
where
    X: XConn,
{
    fn run(&self, id: Xid, x: &X) -> Result<bool> {
        match str_prop(&self.0, id, x)? {
            Some(strs) if !strs.is_empty() => Ok(strs[0] == self.1),
            _ => Ok(false),
        }
    }
}
