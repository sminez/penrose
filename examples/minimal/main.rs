/**
 * penrose :: minimal configuration
 * This file will give you a functional if incredibly minimal window manager that has multiple
 * workspaces and simple client/workspace movement. For a more fleshed out example see the
 * 'simple_config_with_hooks' example.
 */
use penrose::{
    bindings::KeyBindings,
    core::{Config, WindowManager},
    layout::messages::common::{ExpandMain, IncMain, ShrinkMain},
    layout_message, map, modify, spawn,
    xcb::XcbConn,
    Result,
};
use std::collections::HashMap;

fn main() -> Result<()> {
    // FIXME: needs to be keycodes not &str
    // let mut key_bindings: KeyBindings<_, _> = map! {
    //     "M-j" => modify!(|cs| cs.focus_up()),
    //     "M-k" => modify!(|cs| cs.focus_down()),
    //     "M-S-j" => modify!(|cs| cs.swap_up()),
    //     "M-S-k" => modify!(|cs| cs.swap_down()),
    //     "M-S-q" => modify!(|cs| { cs.remove_focused(); }),
    //     // "M-Tab" => run_internal!(toggle_workspace);
    //     // "M-bracketright" => run_internal!(cycle_screen, Forward);
    //     // "M-bracketleft" => run_internal!(cycle_screen, Backward);
    //     // "M-S-bracketright" => run_internal!(drag_workspace, Forward);
    //     // "M-S-bracketleft" => run_internal!(drag_workspace, Backward);
    //     // "M-grave" => run_internal!(cycle_layout, Forward);
    //     // "M-S-grave" => run_internal!(cycle_layout, Backward);
    //     "M-A-Up" => layout_message!(IncMain(1)),
    //     "M-A-Down" => layout_message!(IncMain(-1)),
    //     "M-A-Right" => layout_message!(ExpandMain),
    //     "M-A-Left" => layout_message!(ShrinkMain),
    //     "M-semicolon" => spawn!("dmenu_run"),
    //     "M-Return" => spawn!("st"),
    //     "M-A-Escape" => Box::new(|_, _| std::process::exit(0)),
    // };

    // let key_bindings = gen_keybindings! {
    //     map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
    //         "M-{}" => focus_workspace (REF);
    //         "M-S-{}" => client_to_workspace (REF);
    //     };
    // };

    let wm = WindowManager::new(
        Config::default(),
        // key_bindings,
        HashMap::new(),
        HashMap::new(),
        XcbConn::new()?,
    )?;

    wm.run()
}
