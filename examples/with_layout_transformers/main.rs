//! penrose :: layout transformers
//!
//! Layouts can be wrapped with transformers that modify their behaviour.
use penrose::{
    actions::{modify_with, send_layout_message, spawn},
    bindings::{parse_keybindings_with_xmodmap, KeyEventHandler},
    core::{Config, WindowManager},
    extensions::actions::{exit, log_current_state},
    layout::{
        messages::common::{ExpandMain, IncMain, ShrinkMain},
        transformers::{Gaps, ReflectHorizontal},
        LayoutStack, MainAndStack,
    },
    map, stack,
    x11rb::X11rbRustConn,
    Result,
};
use std::collections::HashMap;
use tracing_subscriber::{self, prelude::*};

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<X11rbRustConn>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| format!("C-{k}");

        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-S-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-S-bracketright" => modify_with(|cs| cs.drag_workspace_forward()),
        "M-S-bracketleft" => modify_with(|cs| cs.drag_workspace_backward()),
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-Up" => send_layout_message(|| IncMain(1)),
        "M-Down" => send_layout_message(|| IncMain(-1)),
        "M-Right" => send_layout_message(|| ExpandMain),
        "M-Left" => send_layout_message(|| ShrinkMain),
        "M-semicolon" => spawn("dmenu_run"),
        "M-S-s" => log_current_state(),
        "M-Return" => spawn("st"),
        "M-Escape" => exit(),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-C-{tag}"),
                modify_with(move |client_set| client_set.focus_tag(tag)),
            ),
            (
                format!("M-C-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

fn layouts() -> LayoutStack {
    let max_main = 1;
    let ratio = 0.6;
    let ratio_step = 0.1;
    let outer_px = 5;
    let inner_px = 5;

    stack!(
        MainAndStack::side(max_main, ratio, ratio_step),
        ReflectHorizontal::wrap(MainAndStack::side(max_main, ratio, ratio_step)),
        MainAndStack::bottom(max_main, ratio, ratio_step)
    )
    .map(|layout| Gaps::wrap(layout, outer_px, inner_px))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .finish()
        .init();

    let config = Config {
        default_layouts: layouts(),
        ..Config::default()
    };

    let conn = X11rbRustConn::new()?;
    let key_bindings = parse_keybindings_with_xmodmap(raw_key_bindings())?;
    let wm = WindowManager::new(config, key_bindings, HashMap::new(), conn)?;

    wm.run()
}
