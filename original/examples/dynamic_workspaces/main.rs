use penrose::{
    common::{bindings::KeyEventHandler, helpers::index_selectors},
    contrib::{
        actions::create_or_switch_to_workspace,
        extensions::{dmenu::*, Scratchpad},
        hooks::{DefaultWorkspace, LayoutSymbolAsRootName, RemoveEmptyWorkspaces},
        layouts::paper,
    },
    core::{
        config::Config,
        hooks::{Hook, Hooks},
        layout::{bottom_stack, side_stack, Layout, LayoutConf},
    },
    gen_keybindings, logging_error_handler, run_external, run_internal,
    xcb::{new_xcb_backed_window_manager, XcbConnection},
    xconnection::XConn,
    Backward, Forward, Less, More, Result,
};
use std::collections::HashMap;

fn my_layouts() -> Vec<Layout> {
    let n_main = 1;
    let ratio = 0.6;
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
        allow_wrapping: false,
    };

    vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
    ]
}

fn dynamic_workspaces<X: XConn>() -> KeyEventHandler<X> {
    create_or_switch_to_workspace(
        || {
            let options = vec!["1term", "2term", "3term", "web", "files"];
            let menu = DMenu::new("WS-SELECT: ", options, DMenuConfig::default());
            if let Ok(MenuMatch::Line(_, choice)) = menu.run(0) {
                Some(choice)
            } else {
                None
            }
        },
        my_layouts(),
    )
}

fn main() -> Result<()> {
    let mut config_builder = Config::default().builder();
    let config = config_builder
        .workspaces(vec!["main"])
        .layouts(my_layouts())
        .build()
        .unwrap();

    let sp = Scratchpad::new("st", 0.8, 0.8);

    let hooks: Hooks<_> = vec![
        LayoutSymbolAsRootName::new() as Box<dyn Hook<XcbConnection>>,
        RemoveEmptyWorkspaces::new(config.workspaces().clone()),
        DefaultWorkspace::new("1term", "[side]", vec!["st"]),
        DefaultWorkspace::new("2term", "[botm]", vec!["st", "st"]),
        DefaultWorkspace::new("3term", "[side]", vec!["st", "st", "st"]),
        DefaultWorkspace::new("web", "[papr]", vec!["firefox"]),
        DefaultWorkspace::new("files", "[botm]", vec!["thunar"]),
        sp.get_hook(),
    ];

    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("dmenu_run");
        "M-Return" => run_external!("st");

        // client management
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-S-q" => run_internal!(kill_client);
        "M-slash" => sp.toggle();

        // workspace management
        "M-w" => dynamic_workspaces();
        "M-Tab" => run_internal!(toggle_workspace);
        "M-bracketright" => run_internal!(cycle_screen, Forward);
        "M-bracketleft" => run_internal!(cycle_screen, Backward);
        "M-S-bracketright" => run_internal!(drag_workspace, Forward);
        "M-S-bracketleft" => run_internal!(drag_workspace, Backward);

        // Layout management
        "M-grave" => run_internal!(cycle_layout, Forward);
        "M-S-grave" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => run_internal!(update_max_main, More);
        "M-A-Down" => run_internal!(update_max_main, Less);
        "M-A-Right" => run_internal!(update_main_ratio, More);
        "M-A-Left" => run_internal!(update_main_ratio, Less);

        "M-A-s" => run_internal!(detect_screens);
        "M-A-Escape" => run_internal!(exit);

        // setting up bindings for 6 possible workspaces
        map: { "1", "2", "3", "4", "5", "6" } to index_selectors(6) => {
            "M-{}" => focus_workspace (REF);
            "M-S-{}" => client_to_workspace (REF);
        };
    };

    let mut wm = new_xcb_backed_window_manager(config, hooks, logging_error_handler())?;
    wm.grab_keys_and_run(key_bindings, HashMap::new())?;

    Ok(())
}
