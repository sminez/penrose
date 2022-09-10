//! Window Manager actions
use crate::{
    common::Xid,
    core::{hooks::runner::HookTrigger, manager::state::WmState},
    xconnection::XConn,
    Result,
};
use std::fmt;

pub(crate) trait Action: fmt::Debug + Send + Sync {
    type Output;

    fn handle<X: XConn>(self, conn: &X, state: &mut WmState) -> Result<Self::Output>;
}

#[derive(Debug)]
struct AddClientToWorkspace {
    pub wix: usize,
    pub id: Xid,
}

impl Action for AddClientToWorkspace {
    type Output = ();

    fn handle<X: XConn>(self, conn: &X, state: &mut WmState) -> Result<()> {
        let Self { wix, id } = self;

        state.clients.modify(id, |c| c.set_workspace(wix));
        if let Some(action) = state.workspaces.add_client(wix, id)? {
            conn.set_client_workspace(id, wix)?;
            action.handle(conn, state)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct RunHook {
    trigger: HookTrigger,
}

impl Action for RunHook {
    type Output = ();

    fn handle<X: XConn>(self, conn: &X, state: &mut WmState) -> Result<()> {
        let Self { trigger } = self;

        Ok(())
    }
}
