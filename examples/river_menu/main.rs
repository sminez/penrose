//! penrose :: a handler that blocks on a menu, which is the case the two threads exist for
//!
//! A menu under Wayland is a layer surface, and river gives a layer surface exclusive keyboard
//! focus *at the end of the manage sequence in which it says so*. So a handler that spawns one
//! and waits for the user's choice is not merely slow: if answering that sequence were the same
//! thread's job, the menu would never get the keyboard, the user could never choose, and the
//! handler would never return. A deadlocked session, from one blocking read.
//!
//! Penrose has one such helper of its own (`DMenu::build_menu`) and any real config has several:
//! a prompt, an action menu, a rebuild that waits on a compiler. So the connection gets a thread
//! of its own which answers sequences from the last published plan, and handlers may block.
//!
//! This is that case, end to end and for real: `M-b` runs `fuzzel --dmenu` and blocks on its
//! output. `tests/headless-menu.sh` types a choice into it and asserts the handler got it.
use penrose::{
    Result,
    builtin::actions::key_handler,
    core::{
        Config, WindowManager,
        bindings::{KeyEventHandler, parse_keybindings},
    },
    map,
    river::RiverConn,
};
use std::{
    collections::HashMap,
    env,
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::fd::AsFd,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tracing_subscriber::{self, EnvFilter, prelude::*};

#[path = "../river_bindings/keyboard.rs"]
mod keyboard;

const LOG_ENV: &str = "PENROSE_MENU_LOG";
const KEYMAP_ENV: &str = "PENROSE_KEYMAP";

/// The options offered, chosen so that one keystroke picks an unambiguous one.
const OPTIONS: &str = "alpha\nbravo\ncharlie\n";

fn note(msg: &str) {
    let path = env::var(LOG_ENV).expect("menu log path");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("menu log");
    writeln!(f, "{msg}").expect("menu log");
    tracing::info!(msg, "noted");
}

/// The shape every real config has: options in, selection out, blocking in between.
fn menu() -> Box<dyn KeyEventHandler<RiverConn>> {
    key_handler(|_, _| {
        note("menu-opening");

        let mut child = Command::new("fuzzel")
            .args(["--dmenu", "--log-level=warning"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        child
            .stdin
            .take()
            .expect("fuzzel stdin")
            .write_all(OPTIONS.as_bytes())?;

        // The blocking read. Under a single threaded window manager this never returns, because
        // the manage sequence that would give fuzzel the keyboard is waiting on this thread.
        let mut chosen = String::new();
        child
            .stdout
            .take()
            .expect("fuzzel stdout")
            .read_to_string(&mut chosen)?;

        note(&format!("chose: {}", chosen.trim()));

        Ok(())
    })
}

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RiverConn>>> {
    map! {
        map_keys: |k: &str| k.to_string();

        "M-b" => menu(),
        "M-n" => key_handler(|_, _| { note("after"); Ok(()) }),
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_ansi(false)
        .finish()
        .init();

    let log = env::var(LOG_ENV).unwrap_or_else(|_| panic!("{LOG_ENV} is not set"));
    File::create(&log)?;
    let keymap = env::var(KEYMAP_ENV).unwrap_or_else(|_| panic!("{KEYMAP_ENV} is not set"));

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

/// Evdev keycodes, and Mod4 as xkb numbers it.
const KEY_B: u32 = 48;
const KEY_N: u32 = 49;
const KEY_R: u32 = 19;
const KEY_A: u32 = 30;
const KEY_V: u32 = 47;
const KEY_ENTER: u32 = 28;
const MOD4: u32 = 64;

fn drive(keymap: &str) -> std::result::Result<(), String> {
    // On a thread of our own rather than in a handler.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(4));

    let mut kb = keyboard::VirtualKeyboard::new()?;
    let f = File::open(keymap).map_err(|e| format!("unable to open {keymap}: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len() as u32;
    kb.keymap(f.as_fd(), size);
    kb.roundtrip()?;

    // Open the menu. The handler blocks here until something is chosen.
    kb.chord(MOD4, KEY_B);

    // Give fuzzel time to map and take the keyboard, then type at it. These keys go to fuzzel
    // rather than to a binding, which is the point: it has exclusive focus.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(2));
    kb.tap(KEY_B);
    kb.tap(KEY_R);
    kb.tap(KEY_A);
    kb.tap(KEY_V);
    kb.tap(KEY_ENTER);

    // And an ordinary binding afterwards, to show the window manager is still there.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(2));
    kb.chord(MOD4, KEY_N);

    kb.roundtrip()?;

    Ok(())
}
