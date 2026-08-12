//! penrose :: minimal river configuration
//!
//! The same shape as the `minimal` example, on the river Wayland compositor instead of X11. The
//! bindings are written identically -- river reuses X11's keysym numbers and modifier masks -- so
//! the only difference is which conn is constructed.
//!
//! Run it from river's init script:
//!
//! ```sh
//! cargo build --no-default-features --features river --example river_minimal
//! river -c /path/to/an/init/script/that/runs/it
//! ```
//!
//! `tests/headless-river.sh` does exactly that against a headless compositor.
use penrose::{
    Result,
    builtin::{
        actions::{exit, modify_with, send_layout_message, spawn},
        layout::messages::{ExpandMain, IncMain, ShrinkMain},
    },
    core::{
        Config, WindowManager,
        bindings::{KeyEventHandler, parse_keybindings},
    },
    map,
    river::RiverConn,
};
use std::collections::HashMap;
use tracing_subscriber::{self, EnvFilter, prelude::*};

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RiverConn>>> {
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
        // A prompt has to be a separate program under Wayland whether we like it or not: a
        // window manager is told that a binding fired, never what was pressed, so it cannot
        // read a line of text. fuzzel is the usual answer.
        "M-semicolon" => spawn("fuzzel"),
        "M-Return" => spawn("alacritty"),
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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        // Plain text: these logs are read back by the scripts in tests/.
        .with_ansi(false)
        .finish()
        .init();

    let conn = RiverConn::new()?;
    let key_bindings = parse_keybindings(raw_key_bindings()).into_result()?;
    // River has no pointer counterpart to X11's motion events outside its own interactive
    // operation, so there are no mouse bindings here yet.
    let wm = WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?;

    wm.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_parse_correctly() {
        let res = parse_keybindings(raw_key_bindings()).into_result();

        if let Err(e) = res {
            panic!("{e}");
        }
    }
}
