//! penrose :: minimal configuration
//!
//! This file will give you a functional if incredibly minimal window manager that has multiple
//! workspaces and simple client/workspace movement.
use penrose::{
    actions::{exit, modify_with, send_layout_message, spawn},
    bindings::KeyEventHandler,
    core::{Config, WindowManager},
    layout::messages::common::{ExpandMain, IncMain, ShrinkMain},
    map,
    xcb::XcbConn,
    Result,
};
use std::collections::HashMap;

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<XcbConn, ()>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-j" => modify_with(|cs| cs.focus_up()),
        "M-k" => modify_with(|cs| cs.focus_down()),
        "M-S-j" => modify_with(|cs| cs.swap_up()),
        "M-S-k" => modify_with(|cs| cs.swap_down()),
        "M-S-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-S-bracketright" => modify_with(|cs| cs.drag_workspace_forward()),
        "M-S-bracketleft" => modify_with(|cs| cs.drag_workspace_backward()),
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-A-Up" => send_layout_message(|| IncMain(1)),
        "M-A-Down" => send_layout_message(|| IncMain(-1)),
        "M-A-Right" => send_layout_message(|| ExpandMain),
        "M-A-Left" => send_layout_message(|| ShrinkMain),
        "M-semicolon" => spawn("dmenu_run"),
        "M-Return" => spawn("st"),
        "M-A-Escape" => exit(),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |client_set| client_set.focus_tag(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

fn main() -> Result<()> {
    let conn = XcbConn::new()?;
    let key_bindings = conn.parse_keybindings_with_xmodmap(raw_key_bindings())?;
    let wm = WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?;

    wm.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_parse_correctly() {
        let conn = XcbConn::new().unwrap();
        let res = conn.parse_keybindings_with_xmodmap(raw_key_bindings());

        if let Err(e) = res {
            panic!("{e}");
        }
    }
}
