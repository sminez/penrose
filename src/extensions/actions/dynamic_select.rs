//! Dynamic selection based actions using Dmenu.
use crate::{
    builtin::actions::key_handler,
    core::{bindings::KeyEventHandler, State},
    custom_error,
    extensions::util::dmenu::{DMenu, DMenuConfig, MenuMatch},
    x::{XConn, XConnExt},
    Xid,
};
use std::collections::HashMap;

/// Use [DMenu] to dynamically select and focus a client window.
///
/// # Arguments
/// * `DMenuConfig` users custom DMenuConfig, the dmenu instance that is launched will
///    obey colorscheme, postion, etc..
///
/// * `custom_prompt` so the user isn't locked into a default prompt.
///    Default: Window
pub fn dmenu_focus_client<X: XConn>(
    config: DMenuConfig,
    custom_prompt: Option<String>,
) -> Box<dyn KeyEventHandler<X>> {
    key_handler(move |state: &mut State<X>, x: &X| {
        let choices: HashMap<String, Xid> = state
            .client_set
            .workspaces()
            .filter(|w| !state.client_set.invisible_tags.iter().any(|t| t == w.tag()))
            .flat_map(|w| {
                w.clients().map(|&id| {
                    let title = x.window_title(id).unwrap_or_else(|_| (*id).to_string());

                    (format!("{}: {}", w.tag(), title), id)
                })
            })
            .collect();

        let screen = state.client_set.current_screen().index();
        let dmenu: DMenu = if custom_prompt.is_some() {
            DMenu::new(custom_prompt.to_owned(), &config, screen)
        } else {
            DMenu::new(Some("Window:".to_owned()), &config, screen)
        };

        if let MenuMatch::Line(_, s) = dmenu.build_menu(choices.keys().collect())? {
            let id = choices
                .get(&s)
                .ok_or_else(|| custom_error!("unexpected dmenu output: {}", s))?;

            x.modify_and_refresh(state, |cs| cs.focus_client(id))?;
        }

        Ok(())
    })
}

/// Use [DMenu] to dynamically select and focus a client window.
///
/// # Arguments
/// * `DMenuConfig` users custom DMenuConfig, the dmenu instance that is launched will
///    obey colorscheme, postion, etc..
///
/// * `custom_prompt` so the user isn't locked into a default prompt.
///    Default: Workspace
pub fn dmenu_focus_tag<X: XConn>(
    config: DMenuConfig,
    custom_prompt: Option<String>,
) -> Box<dyn KeyEventHandler<X>> {
    key_handler(move |state: &mut State<X>, x: &X| {
        let choices = state.client_set.ordered_tags();
        let screen = state.client_set.current_screen().index();

        let dmenu: DMenu = if custom_prompt.is_some() {
            DMenu::new(custom_prompt.to_owned(), &config, screen)
        } else {
            DMenu::new(Some("Window:".to_owned()), &config, screen)
        };

        if let MenuMatch::Line(_, tag) = dmenu.build_menu(choices)? {
            x.modify_and_refresh(state, |cs| cs.focus_tag(&tag))?;
        }

        Ok(())
    })
}

/// Launch [DMenu] for its most basic purposes, launching other programs.
///
/// # Arguments
/// * `DMenuConfig` users custom DMenuConfig, the dmenu instance that is launched will
///    obey colorscheme, postion, etc..
///
/// * `custom_prompt` so the user isn't locked into a default prompt.
///    Default: >>>
pub fn launch_dmenu<X: XConn>(
    config: DMenuConfig,
    custom_prompt: Option<String>,
) -> Box<dyn KeyEventHandler<X>> {
    key_handler(move |state, _| {
        let screen = state.client_set.current_screen().index();

        let dmenu: DMenu = if custom_prompt.is_some() {
            DMenu::new(custom_prompt.to_owned(), &config, screen)
        } else {
            DMenu::new(Some(">>> ".to_owned()), &config, screen)
        };
        dmenu.run()
    })
}
