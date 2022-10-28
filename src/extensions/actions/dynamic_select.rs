//! Dynamic selection based actions using Dmenu.
use crate::{
    core::{actions::key_handler, bindings::KeyEventHandler, State},
    custom_error,
    extensions::util::dmenu::{DMenu, DMenuConfig, MenuMatch},
    x::{XConn, XConnExt},
    Xid,
};
use std::collections::HashMap;

/// Use [DMenu] to dynamically select and focus a client window.
pub fn dmenu_focus_client<X: XConn>(config: DMenuConfig) -> Box<dyn KeyEventHandler<X>> {
    key_handler(move |state: &mut State<X>, x: &X| {
        let choices: HashMap<String, Xid> = state
            .client_set
            .iter_workspaces()
            .filter(|w| !state.client_set.invisible_tags.iter().any(|t| t == w.tag()))
            .flat_map(|w| {
                w.clients().map(|&c| {
                    let title = x.window_title(c).unwrap_or_else(|_| (*c).to_string());

                    (format!("{}: {}", w.tag(), title), c)
                })
            })
            .collect();

        let menu = DMenu::new("Window:", choices.keys().collect(), config);
        let screen = state.client_set.current_screen().index();

        if let MenuMatch::Line(_, s) = menu.run(screen)? {
            let id = choices
                .get(&s)
                .ok_or_else(|| custom_error!("unexpected dmenu output: {}", s))?;

            x.modify_and_refresh(state, |cs| cs.focus_client(id))?;
        }

        Ok(())
    })
}

/// Use [DMenu] to dynamically select and focus a client window.
pub fn dmenu_focus_tag<X: XConn>(config: DMenuConfig) -> Box<dyn KeyEventHandler<X>> {
    key_handler(move |state: &mut State<X>, x: &X| {
        let choices = state.client_set.ordered_tags();
        let menu = DMenu::new("Workspace:", choices, config);
        let screen = state.client_set.current_screen().index();

        if let MenuMatch::Line(_, tag) = menu.run(screen)? {
            x.modify_and_refresh(state, |cs| cs.focus_tag(&tag))?;
        }

        Ok(())
    })
}
