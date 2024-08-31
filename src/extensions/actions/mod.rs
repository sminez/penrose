//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    builtin::actions::{key_handler, modify_with},
    core::{bindings::KeyEventHandler, layout::LayoutStack, State},
    util::spawn,
    x::{atom::Atom, property::Prop, XConn, XConnExt},
    Error, Result, Xid,
};
use tracing::{debug, error};

mod dynamic_select;

#[doc(inline)]
pub use dynamic_select::*;

/// The possible valid actions to use when manipulating full screen state
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FullScreenAction {
    /// Force the window out of fullscreen state
    Remove,
    /// Force the window into fullscreen state
    Add,
    /// Toggle the fullscreen state of the window
    Toggle,
}

/// Set the fullscreen state of a particular client
pub fn set_fullscreen_state<X: XConn>(
    id: Xid,
    action: FullScreenAction,
    state: &mut State<X>,
    x: &X,
) -> Result<()> {
    use FullScreenAction::*;

    let net_wm_state = Atom::NetWmState.as_ref();
    let full_screen = x.intern_atom(Atom::NetWmStateFullscreen.as_ref())?;

    let mut wstate = match x.get_prop(id, net_wm_state) {
        Ok(Some(Prop::Cardinal(vals))) => vals,
        _ => vec![],
    };

    let currently_fullscreen = wstate.contains(&full_screen);
    debug!(%currently_fullscreen, ?action, %id, "setting fullscreen state");

    if action == Add || (action == Toggle && !currently_fullscreen) {
        let r = state
            .client_set
            .screen_for_client(&id)
            .ok_or_else(|| Error::UnknownClient(id))?
            .r;
        state.client_set.float(id, r)?;
        wstate.push(*full_screen);
    } else if action == Remove || (action == Toggle && currently_fullscreen) {
        state.client_set.sink(&id);
        wstate.retain(|&val| val != *full_screen);
    }

    x.set_prop(id, net_wm_state, Prop::Cardinal(wstate))?;
    x.refresh(state)
}

/// Toggle the fullscreen state of the currently focused window.
///
/// **NOTE**: You will need to make use of [add_ewmh_hooks][0] for this action to
///           work correctly.
///
///   [0]: crate::extensions::hooks::add_ewmh_hooks
pub fn toggle_fullscreen<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let id = match state.client_set.current_client() {
            Some(&id) => id,
            None => return Ok(()),
        };

        set_fullscreen_state(id, FullScreenAction::Toggle, state, x)
    })
}

/// Jump to, or create a [Workspace][0].
///
/// Call 'get_name' to obtain a Workspace name and check to see if there is currently a Workspace
/// with that name being managed by the WindowManager. If there is no existing workspace with the
/// given name, create it with the supplied available layouts. If a matching Workspace _does_
/// already exist then simply switch focus to it. This action is most useful when combined with the
/// DefaultWorkspace hook that allows for auto populating named Workspaces when first focusing them.
///
/// > If you just want to dynamically select an existing workspace then you can use
/// > [switch_to_workspace] to select from known workspace names.
///
///   [0]: crate::pure::Workspace
pub fn create_or_switch_to_workspace<X>(
    get_name: fn() -> Option<String>,
    layouts: LayoutStack,
) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    modify_with(move |cs| {
        if let Some(name) = get_name() {
            // if this errors it's because the tag is already present in the stackset
            // so we can just focus it.
            _ = cs.add_workspace(&name, layouts.clone());

            cs.focus_tag(&name);
        }
    })
}

/// Jump to a [Workspace][0] by name.
///
/// Call 'select_name' to select a Workspace name and switch focus to it if it exists.
///
///   [0]: crate::pure::Workspace
pub fn switch_to_workspace<X>(
    select_name: fn(&[String]) -> Option<String>,
) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    modify_with(move |cs| {
        let tags = cs.ordered_tags();
        if let Some(name) = select_name(&tags) {
            cs.focus_tag(&name);
        }
    })
}

/// Focus a client with the given class as `WM_CLASS` or spawn the program with the given command
/// if no such client exists.
///
/// This is useful for key bindings that are based on the program you want to work with rather than
/// having to remember where things are running.
pub fn focus_or_spawn<X>(class: &'static str, command: &'static str) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    key_handler(move |s: &mut State<X>, x: &X| {
        let mut client = None;

        for &id in s.client_set.clients() {
            if let Some(Prop::UTF8String(classes)) = x.get_prop(id, Atom::WmClass.as_ref())? {
                if classes.iter().any(|s| s == class) {
                    client = Some(id);
                    break;
                }
            }
        }

        x.modify_and_refresh(s, |cs| {
            if let Some(id) = client {
                cs.focus_client(&id)
            } else if let Err(e) = spawn(command) {
                error!(%e, %command, "unable to spawn program")
            }
        })
    })
}
