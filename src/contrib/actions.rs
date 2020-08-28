//! Additional helper functions and actions for use with penrose.
use crate::{
    core::{Client, Layout, WindowManager, Workspace},
    data_types::{FireAndForget, Selector},
    helpers::spawn,
};

/**
 * Call 'get_name' to obtain a Workspace name and check to see if there is currently a Workspace
 * with that name being managed by the WindowManager. If there is no existing workspace with the
 * given name, create it with the supplied available layouts. If a matching Workspace _does_
 * already exist then simply switch focus to it. This action is most useful when combined with the
 * DefaultWorkspace hook that allows for auto populating named Workspaces when first focusing them.
 */
pub fn create_or_switch_to_workspace(
    get_name: fn() -> String,
    layouts: Vec<Layout>,
) -> FireAndForget {
    Box::new(move |wm: &mut WindowManager| {
        let name = &get_name();
        let cond = |ws: &Workspace| ws.name() == name;
        let sel = Selector::Condition(&cond);
        if wm.workspace(&sel).is_none() {
            wm.push_workspace(Workspace::new(name, layouts.clone()))
        }
        wm.focus_workspace(&sel);
    })
}

/**
 * Focus a Client with the given class as WM_CLASS or spawn the program with
 * the given command if no such Client exists. This is useful for key bindings
 * that are based on the program you want to work with rather than having to
 * remember where things are running.
 */
pub fn focus_or_spawn(class: String, command: String) -> FireAndForget {
    Box::new(move |wm: &mut WindowManager| {
        let cond = |c: &Client| c.class() == &class;
        if let Some(client) = wm.client(&Selector::Condition(&cond)) {
            let workspace = client.workspace();
            wm.focus_workspace(&Selector::Index(workspace));
        } else {
            spawn(&command);
        }
    })
}
