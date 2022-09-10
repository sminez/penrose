use crate::{
    core::{
        config::Config,
        manager::{clients::Clients, screens::Screens, workspaces::Workspaces, WindowManager},
    },
    xconnection::XConn,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WmState {
    pub(super) config: Config,
    pub(super) clients: Clients,
    pub(super) screens: Screens,
    pub(super) workspaces: Workspaces,
}

impl<X> Deref for WindowManager<X>
where
    X: XConn,
{
    type Target = WmState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<X> DerefMut for WindowManager<X>
where
    X: XConn,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
