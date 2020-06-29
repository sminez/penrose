use penrose::manager;
use penrose::util::*;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && &args[1] == "-v" {
        println!("{}", option_env!("CARGO_PKG_VERSION").unwrap());
        process::exit(0);
    } else if args.len() > 1 {
        println!("usage: penrose [-v]");
        process::exit(1);
    }

    // Check that locale is set & correct

    // Run setup
    let mut wm = manager::WindowManager::new();

    // Scan

    run_autostart();

    // main loop
    wm.run();

    // Cleanup on exit
    // Close display

    process::exit(0);
}
