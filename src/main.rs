/**
 * penrose :: A tiling window manager in the style of dwm
 *
 * Simple configuration can be done by modifying the contents of config.rs,
 * for anything not covered there you should be able to edit the source
 * code with minimal difficulty.
 * To follow the start-up logic and main loop, start with manager.rs
 *
 * The C-level xcb API documentation can be found in the following places:
 *   online manpages: https://www.mankier.com/package/libxcb-devel
 *   official:        https://xcb.freedesktop.org/XcbApi/
 */
use penrose::manager::WindowManager;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && &args[1] == "-v" {
        println!("penrose-{}", option_env!("CARGO_PKG_VERSION").unwrap());
        process::exit(0);
    } else if args.len() > 1 {
        println!("usage: penrose [-v]");
        process::exit(1);
    }

    let mut wm = WindowManager::init();
    wm.run();
}
