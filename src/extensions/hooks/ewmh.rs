//! EWMH compliance for Penrose
//!
//! The primary use of this extension is to provide support for external
//! status bars and panels.
//!
//! See details of the spec here:
//!   https://specifications.freedesktop.org/wm-spec/wm-spec-latest.html
use crate::{
    core::{ClientSet, Config, State},
    x::{atom::Atom, event::ClientMessage, property::Prop, XConn, XConnExt, XEvent},
    Result, Xid,
};

/// The set of Atoms this extension adds support for.
///
/// _NET_SUPPORTED is set to this as part of [EwhmStartupHook]
pub const EWMH_SUPPORTED_ATOMS: &[Atom] = &[
    Atom::NetWmStateHidden,
    Atom::NetWmStateFullscreen,
    Atom::NetWmStateDemandsAttention,
    Atom::NetNumberOfDesktops,
    Atom::NetClientList,
    Atom::NetClientListStacking,
    Atom::NetCurrentDesktop,
    Atom::NetDesktopNames,
    Atom::NetActiveWindow,
    Atom::NetWmDesktop,
    Atom::NetWmStrut,
    Atom::NetWmState,
    Atom::NetWmName,
    // TODO: read up on how this works and implement
    // Atom::NetDesktopViewport,
];

/// The WM_NAME that will be set for the X server
pub const WM_NAME: &str = "penrose";

/// Add the required hooks to manage EWMH compliance to an existing [Config].
///
/// See the module level docs for details of what functionality is provided by
/// this extension.
pub fn add_ewmh_hooks<X>(mut config: Config<X>) -> Config<X>
where
    X: XConn + 'static,
{
    config.compose_or_set_startup_hook(startup_hook);
    config.compose_or_set_refresh_hook(refresh_hook);
    config.compose_or_set_event_hook(event_hook);

    config
}

/// Advertise EWMH support to the X server
pub fn startup_hook<X: XConn>(_state: &mut State<X>, x: &X) -> Result<()> {
    let root = x.root();

    x.set_prop(
        root,
        Atom::WmName.as_ref(),
        Prop::UTF8String(vec![WM_NAME.to_owned()]),
    )?;

    x.set_prop(
        root,
        Atom::NetSupported.as_ref(),
        Prop::Atom(
            EWMH_SUPPORTED_ATOMS
                .iter()
                .map(|a| a.as_ref().to_owned())
                .collect(),
        ),
    )
}

/// Intercept messages from external applications and handle them.
///
/// Currently supports the following:
///   - _NET_CURRENT_DESKTOP :: switching between workspaces
///   - _NET_WM_DESKTOP      :: moving clients between workspaces
///   - _NET_ACTIVE_WINDOW   :: focus a new client and handle workspace switching
///   - _NET_CLOSE_WINDOW    :: closing a client window
pub fn event_hook<X: XConn>(event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
    let ClientMessage {
        id, dtype, data, ..
    } = match event {
        XEvent::ClientMessage(m) => m,
        _ => return Ok(true),
    };

    match dtype.as_ref() {
        // Focus the requested desktop
        "_NET_CURRENT_DESKTOP" => {
            let tag = state.client_set.tag_for_workspace_id(data.as_usize()[0]);
            if let Some(tag) = tag {
                x.modify_and_refresh(state, |cs| cs.focus_tag(&tag))?;
            }
        }

        // Move the client receiving the message to its desired workspace
        "_NET_WM_DESKTOP" => {
            let tag = state.client_set.tag_for_workspace_id(data.as_usize()[0]);
            if let Some(tag) = tag {
                x.modify_and_refresh(state, |cs| cs.move_client_to_tag(id, &tag))?;
            }
        }

        // If the request came from a pager, the first data element should be 2.
        // For pager requests, set the active client (see docs linked at the top of
        // this file for more details on the semantics of this message)
        // TODO: XMonad allows for the user specifying what action should be taken
        //       here (with the default being to focus like this). Might need to
        //       support that in future?
        "_NET_ACTIVE_WINDOW" => {
            if data.as_u32()[0] == 2 {
                x.set_active_client(*id, state)?;
            }
        }

        // Attempt to remove the requested client
        "_NET_CLOSE_WINDOW" => x.modify_and_refresh(state, |cs| {
            cs.remove_client(id);
        })?,

        // Leave other client messages for the default event handling
        _ => (),
    }

    Ok(true)
}

/// Notify external clients of the current status of workspaces and clients
pub fn refresh_hook<X: XConn>(state: &mut State<X>, x: &X) -> Result<()> {
    set_known_desktops(&state.client_set, x)?;
    set_known_clients(&state.client_set, x)?;
    set_current_desktop(&state.client_set, x)?;
    set_client_desktops(&state.client_set, x)?;
    set_active_client(&state.client_set, x)?;

    // TODO: set desktop viewport

    Ok(())
}

fn set_known_desktops<X>(cs: &ClientSet, x: &X) -> Result<()>
where
    X: XConn,
{
    let workspaces_names = cs.ordered_tags();

    x.set_prop(
        x.root(),
        Atom::NetNumberOfDesktops.as_ref(),
        Prop::Cardinal(vec![workspaces_names.len() as u32]),
    )?;

    x.set_prop(
        x.root(),
        Atom::NetDesktopNames.as_ref(),
        Prop::UTF8String(workspaces_names),
    )
}

fn set_known_clients<X>(cs: &ClientSet, x: &X) -> Result<()>
where
    X: XConn,
{
    // FIXME: this currently isn't in stacking order
    let ordered_clients: Vec<Xid> = cs.clients().copied().collect();

    x.set_prop(
        x.root(),
        Atom::NetClientList.as_ref(),
        Prop::Window(ordered_clients.clone()),
    )?;

    x.set_prop(
        x.root(),
        Atom::NetClientListStacking.as_ref(),
        Prop::Window(ordered_clients),
    )
}

fn set_current_desktop<X>(cs: &ClientSet, x: &X) -> Result<()>
where
    X: XConn,
{
    let current_desktop = cs.current_workspace().id as u32;

    x.set_prop(
        x.root(),
        Atom::NetCurrentDesktop.as_ref(),
        Prop::Cardinal(vec![current_desktop]),
    )
}

fn set_client_desktops<X>(cs: &ClientSet, x: &X) -> Result<()>
where
    X: XConn,
{
    let client_desktops = cs.workspaces().flat_map(|w| {
        w.stack
            .iter()
            .flat_map(|s| s.iter().map(|&c| (w.id as u32, c)))
    });

    for (desktop, client) in client_desktops {
        x.set_prop(
            client,
            Atom::NetWmDesktop.as_ref(),
            Prop::Cardinal(vec![desktop]),
        )?;
    }

    Ok(())
}

fn set_active_client<X>(cs: &ClientSet, x: &X) -> Result<()>
where
    X: XConn,
{
    if let Some(&id) = cs.current_client() {
        x.set_prop(
            x.root(),
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![id]),
        )?;
    }

    Ok(())
}
