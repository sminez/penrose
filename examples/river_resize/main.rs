//! penrose :: a window manager that swaps a window into the master area on cue
//!
//! For diagnosing what a layout change actually sends. `M-s` focuses the other
//! window and swaps it into master, which is the case where a small window has to
//! grow, and `tests/headless-resize.sh` reads back what river was told.
use penrose::{
    Result,
    builtin::actions::modify_with,
    core::{
        Config, WindowManager,
        bindings::{KeyEventHandler, parse_keybindings},
    },
    map,
    river::RiverConn,
};
use std::{collections::HashMap, env, fs::File, os::fd::AsFd, thread, time::Duration};
use tracing_subscriber::{self, EnvFilter, prelude::*};

#[path = "../river_bindings/keyboard.rs"]
mod keyboard;

const KEYMAP_ENV: &str = "PENROSE_KEYMAP";

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RiverConn>>> {
    map! {
        map_keys: |k: &str| k.to_string();

        // Focus the small window, without moving it.
        "M-j" => modify_with(|cs| cs.focus_down()),
        // ...then put it in master. This is the config's M-S-h: the window that
        // grows is the one that already had focus, so the pointer warp that
        // follows stays inside it and crosses no boundary.
        "M-s" => modify_with(|cs| cs.swap_focus_and_head()),
    }
}

/// Set to drive the keyboard only, against a window manager that is already
/// running: `PENROSE_KEYS="M-j M-S-h"`. Without it this is its own window
/// manager, which is the self-contained case.
const KEYS_ENV: &str = "PENROSE_KEYS";

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_ansi(false)
        .finish()
        .init();

    let keymap = env::var(KEYMAP_ENV).unwrap_or_else(|_| panic!("{KEYMAP_ENV} is not set"));

    // Keyboard only: type at whichever window manager is running and exit.
    if let Ok(keys) = env::var(KEYS_ENV) {
        if let Err(e) = drive_keys(&keymap, &keys) {
            eprintln!("keyboard: {e}");
        }

        return Ok(());
    }

    thread::spawn(move || {
        if let Err(e) = drive(&keymap) {
            tracing::error!(%e, "keyboard");
        }
    });

    let conn = RiverConn::new()?;
    let key_bindings = parse_keybindings(raw_key_bindings()).into_result()?;
    let wm = WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?;

    wm.run()
}

const KEY_S: u32 = 31;
const KEY_J: u32 = 36;
const MOD4: u32 = 64;

/// Press each space separated chord in turn, a second apart.
///
/// Only the modifiers and keys this needs: `M-` is Mod4 and `S-` is shift.
fn drive_keys(keymap: &str, keys: &str) -> std::result::Result<(), String> {
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(1));

    let mut kb = keyboard::VirtualKeyboard::new()?;
    let f = File::open(keymap).map_err(|e| format!("unable to open {keymap}: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len() as u32;
    kb.keymap(f.as_fd(), size);
    kb.roundtrip()?;

    for chord in keys.split_whitespace() {
        let mut mods = 0;
        let mut name = chord;

        while let Some((prefix, rest)) = name.split_once('-') {
            match prefix {
                "M" => mods |= MOD4,
                "S" => mods |= 1,
                other => return Err(format!("unknown modifier '{other}' in '{chord}'")),
            }
            name = rest;
        }

        let code = match name {
            "j" => KEY_J,
            "h" => 35,
            "s" => KEY_S,
            other => return Err(format!("unknown key '{other}'")),
        };

        eprintln!("pressing {chord}");
        kb.chord(mods, code);

        #[allow(clippy::disallowed_methods)]
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

fn drive(keymap: &str) -> std::result::Result<(), String> {
    // Both windows open and settled first.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(9));

    let mut kb = keyboard::VirtualKeyboard::new()?;
    let f = File::open(keymap).map_err(|e| format!("unable to open {keymap}: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len() as u32;
    kb.keymap(f.as_fd(), size);
    kb.roundtrip()?;

    tracing::info!("FOCUSING THE SMALL WINDOW");
    kb.chord(MOD4, KEY_J);
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(2));

    tracing::info!("SWAPPING NOW");
    kb.chord(MOD4, KEY_S);

    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(4));

    Ok(())
}
