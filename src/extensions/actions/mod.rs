//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    actions::{key_handler, modify_with},
    bindings::KeyEventHandler,
    core::State,
    layout::LayoutStack,
    util::spawn,
    x::{atom::Atom, property::Prop, XConn, XConnExt},
};
use tracing::{error, info};

/// Exit penrose
///
/// Immediately exit the window manager with exit code 0.
pub fn exit<X>() -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    key_handler(|_, _| std::process::exit(0))
}

/// Info log the current window manager [State].
pub fn log_current_state<X>() -> Box<dyn KeyEventHandler<X>>
where
    X: XConn + std::fmt::Debug,
{
    key_handler(|s: &mut State<X>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}

/// Jump to, or create, a [Workspace]
///
/// Call 'get_name' to obtain a Workspace name and check to see if there is currently a Workspace
/// with that name being managed by the WindowManager. If there is no existing workspace with the
/// given name, create it with the supplied available layouts. If a matching Workspace _does_
/// already exist then simply switch focus to it. This action is most useful when combined with the
/// DefaultWorkspace hook that allows for auto populating named Workspaces when first focusing them.
pub fn create_or_switch_to_workspace<X>(
    get_name: fn() -> Option<String>,
    layouts: LayoutStack,
) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn + std::fmt::Debug,
{
    modify_with(move |cs| {
        if let Some(name) = get_name() {
            if !cs.contains_tag(&name) {
                cs.add_workspace(&name, layouts.clone());
            }

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
    X: XConn + std::fmt::Debug,
{
    key_handler(move |s: &mut State<X>, x: &X| {
        let mut client = None;

        for &id in s.client_set.iter_clients() {
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
