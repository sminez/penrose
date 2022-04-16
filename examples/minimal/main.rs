/**
 * penrose :: minimal configuration
 * This file will give you a functional if incredibly minimal window manager that has multiple
 * workspaces and simple client/workspace movement. For a more fleshed out example see the
 * 'simple_config_with_hooks' example.
 */
use penrose::{
    common::{bindings::MouseEvent, helpers::index_selectors},
    core::{config::Config, manager::WindowManager},
    gen_keybindings, gen_mousebindings, logging_error_handler, run_external, run_internal,
    xcb::new_xcb_backed_window_manager,
    Backward, Forward, Less, More, Result,
};

fn main() -> Result<()> {
    let config = Config::default();
    let hooks = vec![];

    let key_bindings = gen_keybindings! {
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-S-q" => run_internal!(kill_client);
        "M-Tab" => run_internal!(toggle_workspace);
        "M-bracketright" => run_internal!(cycle_screen, Forward);
        "M-bracketleft" => run_internal!(cycle_screen, Backward);
        "M-S-bracketright" => run_internal!(drag_workspace, Forward);
        "M-S-bracketleft" => run_internal!(drag_workspace, Backward);
        "M-grave" => run_internal!(cycle_layout, Forward);
        "M-S-grave" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => run_internal!(update_max_main, More);
        "M-A-Down" => run_internal!(update_max_main, Less);
        "M-A-Right" => run_internal!(update_main_ratio, More);
        "M-A-Left" => run_internal!(update_main_ratio, Less);
        "M-A-Escape" => run_internal!(exit);
        "M-semicolon" => run_external!("dmenu_run");
        "M-Return" => run_external!("st");

        map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
            "M-{}" => focus_workspace (REF);
            "M-S-{}" => client_to_workspace (REF);
        };
    };

    let mouse_bindings = gen_mousebindings! {
        Press Right + [Meta] => |wm: &mut WindowManager<_>, _: &MouseEvent| wm.cycle_workspace(Forward),
        Press Left + [Meta] => |wm: &mut WindowManager<_>, _: &MouseEvent| wm.cycle_workspace(Backward)
    };

    let mut wm = new_xcb_backed_window_manager(config, hooks, logging_error_handler())?;
    wm.grab_keys_and_run(key_bindings, mouse_bindings)?;

    Ok(())
}
