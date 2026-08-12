//! penrose :: restarting on river without losing the session
//!
//! X11 restarts by exiting and letting a supervisor relaunch, relying on the X server as the
//! state store: which workspace a window was on is read back off the window itself. River has no
//! property store, so that route loses tags and focus outright.
//!
//! River's hot swap is the answer instead. `M-q` writes a state file keyed on river's window
//! identifier -- which is stable across a restart because it belongs to the window rather than to
//! our connection -- asks river to stop sending us events, and execs itself when river says it is
//! finished. Every client stays alive across the swap.
//!
//! This is the shape a real config wants; `tests/headless-restart.sh` runs it and asserts that
//! the windows and their tags came back.
use penrose::{
    Result, WinId,
    builtin::actions::{key_handler, modify_with},
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
    fs::{self, File},
    io::Write,
    os::{fd::AsFd, unix::process::CommandExt},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};
use std::{thread, time::Duration};
use tracing_subscriber::{self, EnvFilter, prelude::*};

// The same virtual keyboard the bindings spec uses: a headless seat has none, and a restart is
// triggered by a binding like everything else.
#[path = "../river_bindings/keyboard.rs"]
mod keyboard;

/// A keymap, as produced by `setxkbmap -print | xkbcomp -xkb`.
const KEYMAP_ENV: &str = "PENROSE_KEYMAP";

/// Evdev keycodes, and Mod4 as xkb numbers it.
const KEY_Q: u32 = 16;
const KEY_2: u32 = 3;
const MOD4: u32 = 64;
const SHIFT: u32 = 1;

/// Move the focused window to tag 2, look at it, then restart.
fn drive(keymap: String) -> std::result::Result<(), String> {
    // On a thread of our own rather than in a handler.
    #[allow(clippy::disallowed_methods)]
    thread::sleep(Duration::from_secs(5));

    let mut kb = keyboard::VirtualKeyboard::new()?;
    let f = File::open(&keymap).map_err(|e| format!("unable to open {keymap}: {e}"))?;
    let size = f.metadata().map_err(|e| e.to_string())?.len() as u32;
    kb.keymap(f.as_fd(), size);
    kb.roundtrip()?;

    kb.chord(MOD4 | SHIFT, KEY_2);
    kb.chord(MOD4, KEY_2);
    kb.chord(MOD4, KEY_Q);

    kb.roundtrip()?;

    Ok(())
}

/// Where the tag of each window is written, keyed by river's window identifier.
const STATE_ENV: &str = "PENROSE_RESTART_STATE";

/// How many times this has restarted, so the test can tell the generations apart.
const GENERATION_ENV: &str = "PENROSE_RESTART_GENERATION";

/// Set by the restart binding, read after `run` returns. A restart cannot exec from inside the
/// handler: river is told to stop and answers with `finished`, and the run loop has to see that
/// and unwind before the process is replaced.
static RESTARTING: AtomicBool = AtomicBool::new(false);

fn restart() -> Box<dyn KeyEventHandler<RiverConn>> {
    key_handler(|state, conn: &mut RiverConn| {
        let path = env::var(STATE_ENV).expect("state file path");
        let mut f = File::create(path)?;

        let tagged: Vec<(WinId, String)> = state
            .client_set
            .clients()
            .filter_map(|&id| Some((id, state.client_set.tag_for_client(&id)?.to_string())))
            .collect();

        for (id, tag) in tagged {
            if let Some(identifier) = conn.window_identifier(id) {
                writeln!(f, "{identifier}\t{tag}")?;
            }
        }

        RESTARTING.store(true, Ordering::SeqCst);
        conn.stop();

        Ok(())
    })
}

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RiverConn>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-q" => restart(),
    };

    for tag in &["1", "2", "3"] {
        raw_bindings.extend([
            (format!("M-{tag}"), modify_with(move |cs| cs.focus_tag(tag))),
            (
                format!("M-S-{tag}"),
                modify_with(move |cs| cs.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

/// Read back what the previous generation wrote, if there was one.
fn restore_tags() -> HashMap<String, String> {
    let Ok(path) = env::var(STATE_ENV) else {
        return HashMap::new();
    };

    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(id, tag)| (id.to_string(), tag.to_string()))
        .collect()
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        // Plain text: these logs are read back by the scripts in tests/.
        .with_ansi(false)
        .finish()
        .init();

    let generation: u32 = env::var(GENERATION_ENV)
        .ok()
        .and_then(|g| g.parse().ok())
        .unwrap_or(0);

    let tags = restore_tags();
    tracing::info!(generation, restoring = tags.len(), "starting");

    // Only the first generation types: the second exists to be looked at.
    if generation == 0
        && let Ok(keymap) = env::var(KEYMAP_ENV)
    {
        thread::spawn(move || {
            if let Err(e) = drive(keymap) {
                tracing::error!(%e, "keyboard");
            }
        });
    }

    let conn = RiverConn::new()?.restore_tags(tags);
    let fatal = conn.fatal_watch();
    let key_bindings = parse_keybindings(raw_key_bindings()).into_result()?;
    let wm = WindowManager::new(Config::default(), key_bindings, HashMap::new(), conn)?;

    wm.run()?;

    if let Some(reason) = fatal.reason() {
        eprintln!("river connection lost: {reason}");
        std::process::exit(1);
    }

    if RESTARTING.load(Ordering::SeqCst) {
        let exe = env::current_exe().expect("our own path");
        tracing::info!(generation = generation + 1, "restarting");

        // Never returns unless the exec itself fails.
        let e = Command::new(exe)
            .env(GENERATION_ENV, (generation + 1).to_string())
            .exec();
        tracing::error!(%e, "unable to exec ourselves");
    }

    Ok(())
}
