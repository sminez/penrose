//! penrose :: Scratchpads
//!
use penrose::{
    core::{
        actions::{modify_with, send_layout_message, spawn},
        bindings::{parse_keybindings_with_xmodmap, KeyEventHandler},
        layout::{
            messages::common::{ExpandMain, IncMain, ShrinkMain},
            transformers::{Gaps, ReflectHorizontal, ReserveTop},
            LayoutStack, MainAndStack,
        },
        Config, WindowManager,
    },
    extensions::{
        actions::{exit, log_current_state},
        hooks::{
            add_ewmh_hooks, add_named_scratchpads, manage::FloatingCentered, NamedScratchPad,
            SpawnOnStartup, ToggleNamedScratchPad,
        },
    },
    map, stack,
    x::query::ClassName,
    x11rb::X11rbRustConn,
    Color, Result,
};
use penrose_bar::{status_bar, Position, TextStyle};
use std::collections::HashMap;
use tracing_subscriber::{self, prelude::*};

const FONT: &str = "ProFontIIx Nerd Font";
const BLACK: &str = "#282828";
const WHITE: &str = "#ebdbb2";
const GREY: &str = "#3c3836";
const BLUE: &str = "#458588";

fn raw_key_bindings(
    toggle_scratch: ToggleNamedScratchPad,
) -> HashMap<String, Box<dyn KeyEventHandler<X11rbRustConn>>> {
    let mut raw_bindings = map! {
        // map_keys: |k: &str| format!("C-{k}");
        map_keys: |k: &str| k.to_owned();

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
        "M-slash" => Box::new(toggle_scratch),
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
    .map(|layout| ReserveTop::wrap(Gaps::wrap(layout, outer_px, inner_px), 18))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .finish()
        .init();

    let config = add_ewmh_hooks(Config {
        default_layouts: layouts(),
        startup_hook: Some(SpawnOnStartup::boxed(
            "/usr/local/scripts/penrose-startup.sh",
        )),
        ..Config::default()
    });

    // Create a new named scratchpad and toggle handle for use in keybindings.
    let (nsp, toggle_scratch) = NamedScratchPad::new(
        "terminal",
        "st -c StScratchpad",
        ClassName("StScratchpad"),
        FloatingCentered::new(0.8, 0.8),
    );

    let conn = X11rbRustConn::new()?;
    let key_bindings = parse_keybindings_with_xmodmap(raw_key_bindings(toggle_scratch))?;

    // Initialise the required state extension and hooks for handling the named scratchpad
    let wm = add_named_scratchpads(
        WindowManager::new(config, key_bindings, HashMap::new(), conn)?,
        vec![nsp],
    );

    let bar = status_bar(
        18,
        &TextStyle {
            font: FONT.to_string(),
            point_size: 8,
            fg: Color::try_from(WHITE)?,
            bg: Some(Color::try_from(BLACK)?),
            padding: (2.0, 2.0),
        },
        Color::try_from(BLUE)?, // highlight
        Color::try_from(GREY)?, // empty_ws
        Position::Top,
    )
    .unwrap();

    let wm = bar.add_to(wm);

    wm.run()
}
