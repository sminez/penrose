/**
 * penrose :: minimal configuration
 * This file will give you a functional if incredibly minimal window manager that has multiple
 * workspaces and simple client/workspace movement. For a more fleshed out example see the
 * 'simple_config_with_hooks' example.
 */
#[macro_use]
extern crate penrose;

use penrose::{Backward, Config, Forward, Less, More, WindowManager, XcbConnection};

fn main() {
    let config = Config::default();
    let key_bindings = gen_keybindings! {
        "M-j" => run_internal!(cycle_client, Forward),
        "M-k" => run_internal!(cycle_client, Backward),
        "M-S-j" => run_internal!(drag_client, Forward),
        "M-S-k" => run_internal!(drag_client, Backward),
        "M-S-q" => run_internal!(kill_client),
        "M-Tab" => run_internal!(toggle_workspace),
        "M-bracketright" => run_internal!(cycle_screen, Forward),
        "M-bracketleft" => run_internal!(cycle_screen, Backward),
        "M-S-bracketright" => run_internal!(drag_workspace, Forward),
        "M-S-bracketleft" => run_internal!(drag_workspace, Backward),
        "M-grave" => run_internal!(cycle_layout, Forward),
        "M-S-grave" => run_internal!(cycle_layout, Backward),
        "M-A-Up" => run_internal!(update_max_main, More),
        "M-A-Down" => run_internal!(update_max_main, Less),
        "M-A-Right" => run_internal!(update_main_ratio, More),
        "M-A-Left" => run_internal!(update_main_ratio, Less),
        "M-A-Escape" => run_internal!(exit),
        "M-semicolon" => run_external!("dmenu_run"),
        "M-Return" => run_external!("st");

        forall_workspaces: config.workspaces => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let conn = XcbConnection::new();
    let mut wm = WindowManager::init(config, &conn);
    wm.grab_keys_and_run(key_bindings);
}
