//! penrose :: minimal configuration
//!
//! This file will give you a functional if incredibly minimal window manager that has multiple
//! workspaces and simple client/workspace movement.
use penrose::{
    bindings::handlers::{exit, modify_with, send_layout_message, spawn},
    core::{Config, WindowManager},
    layout::messages::common::{ExpandMain, IncMain, ShrinkMain},
    map,
    xcb::XcbConn,
    Result,
};
use std::collections::HashMap;

fn main() -> Result<()> {
    let mut key_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-j" => modify_with(|cs| cs.focus_up()),
        "M-k" => modify_with(|cs| cs.focus_down()),
        "M-S-j" => modify_with(|cs| cs.swap_up()),
        "M-S-k" => modify_with(|cs| cs.swap_down()),
        "M-S-q" => modify_with(|cs| { cs.remove_focused(); }),
        // "M-Tab" => run_internal!(toggle_workspace);
        // "M-bracketright" => run_internal!(cycle_screen, Forward);
        // "M-bracketleft" => run_internal!(cycle_screen, Backward);
        // "M-S-bracketright" => run_internal!(drag_workspace, Forward);
        // "M-S-bracketleft" => run_internal!(drag_workspace, Backward);
        // "M-grave" => run_internal!(cycle_layout, Forward);
        // "M-S-grave" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => send_layout_message(|| IncMain(1)),
        "M-A-Down" => send_layout_message(|| IncMain(-1)),
        "M-A-Right" => send_layout_message(|| ExpandMain),
        "M-A-Left" => send_layout_message(|| ShrinkMain),
        "M-semicolon" => spawn("dmenu_run"),
        "M-Return" => spawn("st"),
        "M-A-Escape" => exit(),
    };

    let workspace_tags = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];

    for tag in workspace_tags.iter() {
        key_bindings.extend([
            (format!("M-{tag}"), modify_with(move |cs| cs.focus_tag(tag))),
            (
                format!("M-S-{tag}"),
                modify_with(move |cs| cs.move_focused_to_tag(tag)),
            ),
        ]);
    }

    let conn = XcbConn::new()?;
    let key_bindings = conn.parse_keybindings_with_xmodmap(key_bindings)?;

    let wm = WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?;

    wm.run()
}
