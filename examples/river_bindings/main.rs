//! penrose :: a river window manager with a keyboard to press
//!
//! Bindings are the part of the river backend with the most to go wrong and the least to show
//! for it when it does. River registers each binding compositor-side and enables or disables it
//! through the plan, and a key sequence has to enable the keys which would continue it for the
//! duration of a capture and put the previous set back afterwards. Getting that wrong leaves a
//! session in which a shortcut silently does nothing, which looks exactly like a window manager
//! that is working.
//!
//! So this is a window manager rather than a client -- only a window manager has bindings -- with
//! a config whose actions append to a file, and a second connection which gives the seat a
//! keyboard through `virtual-keyboard-unstable-v1` and types on it. A headless seat has no
//! keyboard of its own, so the presses have to come from somewhere.
//!
//! The assertions live in `tests/headless-bindings.sh`, which reads the file this writes.
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
    io::Write,
    os::fd::AsFd,
    thread,
    time::Duration,
};
use tracing_subscriber::{self, EnvFilter, prelude::*};

mod keyboard;

/// Where the config's actions record that they ran.
const LOG_ENV: &str = "PENROSE_BINDING_LOG";

/// A keymap, as produced by `setxkbmap -print | xkbcomp -xkb`. A virtual keyboard has to hand the
/// compositor one before it can send a key.
const KEYMAP_ENV: &str = "PENROSE_KEYMAP";

fn note(msg: &'static str) -> Box<dyn KeyEventHandler<RiverConn>> {
    key_handler(move |_, _| {
        let path = env::var(LOG_ENV).expect("binding log path");
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, "{msg}")?;
        tracing::info!(action = msg, "binding ran");

        Ok(())
    })
}

/// `M-b` is an ordinary binding and is the one that matters: if it still works after a sequence
/// has ended, the bindings the capture disabled were put back.
fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RiverConn>>> {
    map! {
        map_keys: |k: &str| k.to_string();

        "M-b" => note("global-b"),
        "M-m a" => note("seq-a"),
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        // Plain text: these logs are read back by the scripts in tests/.
        .with_ansi(false)
        .finish()
        .init();

    let log = env::var(LOG_ENV).unwrap_or_else(|_| panic!("{LOG_ENV} is not set"));
    File::create(&log)?;
    let keymap = env::var(KEYMAP_ENV).unwrap_or_else(|_| panic!("{KEYMAP_ENV} is not set"));

    // Before run(), which does not return.
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

/// Evdev keycodes, which is what a virtual keyboard sends: an xkb keycode minus 8.
const KEY_M: u32 = 50;
const KEY_A: u32 = 30;
const KEY_B: u32 = 48;
const KEY_Z: u32 = 44;

/// Mod4, which xkb numbers the same way X11 does. That coincidence is the reason a config's
/// `M-` bindings carry over to river unchanged.
const MOD4: u32 = 64;

/// Give the seat a keyboard, then type.
///
/// On a connection of its own, because this is a client where the window manager is not.
fn drive(keymap: &str) -> std::result::Result<(), String> {
    // Long enough for the window manager to have finished its first manage sequence, which is
    // when the bindings are enabled. Nothing observable says when that has happened, so this is
    // a sleep; the assertions fail loudly rather than silently if it is too short.
    // On a thread of our own rather than in a handler.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(4));

    let mut kb = keyboard::VirtualKeyboard::new()?;
    let f = File::open(keymap).map_err(|e| format!("unable to open {keymap}: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len() as u32;
    kb.keymap(f.as_fd(), size);
    kb.roundtrip()?;

    // A sequence: M-m arms the capture, then a completes it.
    kb.chord(MOD4, KEY_M);
    kb.tap(KEY_A);
    // An ordinary binding, which only fires if the capture put the bindings back.
    kb.chord(MOD4, KEY_B);
    // A sequence abandoned by a key that is not in it: river eats it and says ate_unbound_key.
    kb.chord(MOD4, KEY_M);
    kb.tap(KEY_Z);
    // And again, because the abandoned path has to restore the bindings too.
    kb.chord(MOD4, KEY_B);

    kb.roundtrip()?;
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(2));

    Ok(())
}
