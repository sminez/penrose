//! Support for managing multiple floating scratchpad programs that can be
//! toggled on or off on the active workspace.
use crate::{
    Result, WinId,
    core::{
        State, WindowManager,
        bindings::KeyEventHandler,
        conn::{Conn, ConnExt, Query},
        hooks::ManageHook,
    },
    util::spawn,
};
use std::{borrow::Cow, collections::HashMap, fmt};
use tracing::{debug, error, warn};

/// The tag used for a placeholder Workspace that holds scratchpad windows when
/// they are currently hidden.
pub const NSP_TAG: &str = "NSP";

/// A toggle-able client program that can be shown and hidden via a keybinding.
pub struct NamedScratchPad<C>
where
    C: Conn,
{
    name: Cow<'static, str>,
    prog: Cow<'static, str>,
    client: Option<WinId>,
    query: Box<dyn Query<C>>,
    hook: Box<dyn ManageHook<C>>,
}

impl<C: Conn> fmt::Debug for NamedScratchPad<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedScratchpad")
            .field("name", &self.name)
            .field("prog", &self.prog)
            .field("client", &self.client)
            .finish()
    }
}

impl<C> NamedScratchPad<C>
where
    C: Conn,
{
    /// Create a new named scratchpad.
    pub fn new<Q, H>(
        name: impl Into<Cow<'static, str>>,
        prog: impl Into<Cow<'static, str>>,
        query: Q,
        manage_hook: H,
        run_hook_on_toggle: bool,
    ) -> (Self, ToggleNamedScratchPad)
    where
        Q: Query<C> + 'static,
        H: ManageHook<C> + 'static,
    {
        let name = name.into();
        let nsp = Self {
            name: name.clone(),
            prog: prog.into(),
            client: None,
            query: Box::new(query),
            hook: Box::new(manage_hook),
        };

        (
            nsp,
            ToggleNamedScratchPad {
                name,
                run_hook_on_toggle,
            },
        )
    }
}

// Private wrapper type to ensure that only this module can access this state extension
struct NamedScratchPadState<C: Conn>(HashMap<Cow<'static, str>, NamedScratchPad<C>>);

/// Add the required hooks to manage EWMH compliance to an existing [crate::core::Config].
///
/// See the module level docs for details of what functionality is provided by
/// this extension.
pub fn add_named_scratchpads<C>(wm: &mut WindowManager<C>, scratchpads: Vec<NamedScratchPad<C>>)
where
    C: Conn + 'static,
{
    let state: HashMap<_, _> = scratchpads
        .into_iter()
        .map(|nsp| (nsp.name.clone(), nsp))
        .collect();

    wm.state.add_extension(NamedScratchPadState(state));
    wm.state
        .client_set
        .add_invisible_workspace(NSP_TAG)
        .expect("named scratchpad tag to be unique");
    wm.state.config.compose_or_set_manage_hook(manage_hook);
    wm.state.config.compose_or_set_refresh_hook(refresh_hook);
}

/// Store clients matching NamedScratchPad queries and run the associated [ManageHook].
pub fn manage_hook<C: Conn + 'static>(id: WinId, state: &mut State<C>, conn: &mut C) -> Result<()> {
    let s = state.extension::<NamedScratchPadState<C>>()?;

    for sp in s.borrow_mut().0.values_mut() {
        if sp.client.is_none() && sp.query.run(id, conn)? {
            debug!(scratchpad=sp.name.as_ref(), %id, "matched query for named scratchpad");
            sp.client = Some(id);
            return sp.hook.call(id, state, conn);
        }
    }

    Ok(())
}

/// Remove destroyed clients from internal scratchpad state
pub fn refresh_hook<C: Conn + 'static>(state: &mut State<C>, _: &mut C) -> Result<()> {
    let s = state.extension::<NamedScratchPadState<C>>()?;
    for sp in s.borrow_mut().0.values_mut() {
        match sp.client {
            Some(id) if !state.client_set.contains(&id) => {
                debug!(%sp.name, %id, "scratchpad client destroyed");
                sp.client = None;
                break;
            }
            _ => (),
        }
    }

    Ok(())
}

/// Toggle the visibility of a NamedScratchPad.
///
/// This will spawn the requested client program if it isn't currently running or
/// move it to the focused workspace. If the scratchpad is currently visible it
/// will be hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleNamedScratchPad {
    name: Cow<'static, str>,
    run_hook_on_toggle: bool,
}

impl<C: Conn + 'static> KeyEventHandler<C> for ToggleNamedScratchPad {
    #[tracing::instrument(level = "debug", skip(state, conn))]
    fn call(&mut self, state: &mut State<C>, conn: &mut C) -> Result<()> {
        let _s = state.extension::<NamedScratchPadState<C>>()?;
        let mut s = _s.borrow_mut();
        let name = self.name.as_ref();

        let (id, hook) = match s.0.get_mut(&self.name) {
            // Active client somewhere in the StackSet
            Some(NamedScratchPad {
                client: Some(id),
                hook,
                ..
            }) if state.client_set.contains(id) => {
                debug!(%id, %name, "NamedScratchPad client exists in state");
                (*id, hook)
            }

            // No active client or client is no longer in state
            Some(nsp) => {
                debug!(%nsp.prog, %name, ?nsp.client, "spawning NamedScratchPad program");
                nsp.client = None;
                return spawn(nsp.prog.as_ref());
            }

            // The user created a ToggleNamedScratchPad but didn't register the scratchpad
            None => {
                warn!(%name, "toggle called for unknown scratchpad: did you remember to call add_named_scratchpads?");
                return Ok(());
            }
        };

        debug!(
            %id,
            %name,
            current_tag = state.client_set.current_tag(),
            current_screen = state.client_set.current_screen().index(),
            "Toggling nsp client"
        );

        if state.client_set.current_workspace().contains(&id) {
            // Toggle off: hiding the client on our invisible workspace
            debug!(%id, "current workspace contains target client: moving to NSP tag");
            state.client_set.move_client_to_tag(&id, NSP_TAG);
        } else {
            // Toggle on / bring to current workspace
            debug!(%id, "current workspace does not contain target client: moving to tag");
            state.client_set.move_client_to_current_tag(&id);

            if self.run_hook_on_toggle
                && let Err(e) = hook.call(id, state, conn)
            {
                error!(%e, %name, %id, "unable to run NSP manage hook during toggle");
            }
        }

        drop(s);

        debug!(%id, %name, "running refresh following NamedScratchPad toggle");
        conn.refresh(state)
    }
}
