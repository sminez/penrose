//! Core data structures and user facing functionality for the window manager
use std::ops::Deref;

/// An X11 ID for a given resource
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Xid(u64);

impl Deref for Xid {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u64> for Xid {
    fn from(id: u64) -> Self {
        Self(id)
    }
}
