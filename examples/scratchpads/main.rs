//! penrose :: adding named scratchpads to your config
//!
//! This file adds named scratchpad support to the `minimal` example.
use penrose::{
    builtin::{
        actions::{exit, modify_with, send_layout_message, spawn},
        layout::messages::{ExpandMain, IncMain, ShrinkMain},
    },
    core::{
        bindings::{parse_keybindings_with_xmodmap, KeyEventHandler},
        Config, WindowManager,
    },
    extensions::hooks::{
        add_named_scratchpads, manage::FloatingCentered, NamedScratchPad, ToggleNamedScratchPad,
    },
    map,
    x::query::ClassName,
    x11rb::RustConn,
    Result,
};
use std::collections::HashMap;
use tracing_subscriber::{self, prelude::*};

fn raw_key_bindings(
    toggle_1: ToggleNamedScratchPad,
    toggle_2: ToggleNamedScratchPad,
) -> HashMap<String, Box<dyn KeyEventHandler<RustConn>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-S-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-S-Up" => send_layout_message(|| IncMain(1)),
        "M-S-Down" => send_layout_message(|| IncMain(-1)),
        "M-S-Right" => send_layout_message(|| ExpandMain),
        "M-S-Left" => send_layout_message(|| ShrinkMain),
        "M-semicolon" => spawn("dmenu_run"),
        "M-Return" => spawn("st"),
        "M-A-Escape" => exit(),

        // Adding the bindings for the named scratchpad handlers in addition
        // to the bindings from `minimal/main.rs`
        "M-slash" => Box::new(toggle_1),
        "M-p" => Box::new(toggle_2),
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
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .finish()
        .init();

    // Constructing NamedScratchPads gives you back the scratchpad itself (which
    // needs to be added to the window manager state below) and a handle which is
    // usable as a key binding.
    // The `Query` used to identify your scratchpads needs to be something that
    // can be used to determine which NamedScratchPad you are talking about. In
    // this case there are two different programs being used with distinct class
    // names but you are free to use any `Query` you wish.
    let (nsp_1, toggle_1) = NamedScratchPad::new(
        "terminal",
        "st -c StScratchpad",
        ClassName("StScratchpad"),
        FloatingCentered::new(0.8, 0.8),
        true,
    );
    let (nsp_2, toggle_2) = NamedScratchPad::new(
        "qt-console",
        "jupyter-qtconsole",
        ClassName("jupyter-qtconsole"),
        FloatingCentered::new(0.8, 0.8),
        true,
    );

    let conn = RustConn::new()?;
    let key_bindings = parse_keybindings_with_xmodmap(raw_key_bindings(toggle_1, toggle_2))?;

    // `add_named_scratchpads` is used to store the required state extensions in your WindowManager
    // so that the toggle hooks bound to keys above are able to track and manage your scratchpad
    // windows.
    // The order in which the NamedScratchPads are specified in the Vec passed here determines the
    // order in which each scratchpad's `Query` will be run to determine owndership of newly
    // spawned client windows.
    let wm = add_named_scratchpads(
        WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?,
        vec![nsp_1, nsp_2],
    );

    wm.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_parse_correctly_with_xmodmap() {
        let res = parse_keybindings_with_xmodmap(raw_key_bindings());

        if let Err(e) = res {
            panic!("{e}");
        }
    }
}
