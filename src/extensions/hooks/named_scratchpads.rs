//! Support for managing multiple floating scratchpad programs that can be
//! toggled on or off on the active workspace.

use crate::{
    hooks::ManageHook,
    x::{Query, XConn},
};

pub struct ScratchPad<X>
where
    X: XConn,
{
    pub name: &'static str,
    pub command: &'static str,
    pub query: Box<dyn Query<X>>,
    pub hook: Box<dyn ManageHook<X>>,
}
