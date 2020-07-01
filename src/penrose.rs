// use penrose::manager;
// use penrose::util::*;
use std::thread;
use std::time::Duration;
use std::{env, process};
use xcb;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && &args[1] == "-v" {
        println!("{}", option_env!("CARGO_PKG_VERSION").unwrap());
        process::exit(0);
    } else if args.len() > 1 {
        println!("usage: penrose [-v]");
        process::exit(1);
    }

    eprintln!("for this to work you need to be in a TTY");
    eprintln!("put 'exec $WM' in your ~/.xinitrc and then run the following:");
    eprintln!("WM=/path/to/penrose startx");
    xcb_root_demo();

    // Check that locale is set & correct

    // let mut wm = manager::WindowManager::new();
    // run_autostart();
    // wm.run();

    // Cleanup on exit
    // Close display

    process::exit(0);
}

fn xcb_root_demo() {
    let (conn, screen_num) = match xcb::Connection::connect(None) {
        Ok((conn, screen_num)) => {
            eprintln!("Established X connection on '{}'", screen_num);
            (conn, screen_num)
        }
        Err(e) => {
            eprintln!("Failed to establish X connection: '{}'", e);
            process::exit(42);
        }
    };

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let gc = conn.generate_id();

    xcb::create_gc(
        &conn,
        gc,
        screen.root(),
        &[
            (xcb::GC_FUNCTION, xcb::GX_XOR),
            (xcb::GC_FOREGROUND, screen.white_pixel()),
            (xcb::GC_BACKGROUND, screen.black_pixel()),
            (xcb::GC_LINE_WIDTH, 1),
            (xcb::GC_LINE_STYLE, xcb::LINE_STYLE_ON_OFF_DASH),
            (xcb::GC_GRAPHICS_EXPOSURES, 0),
        ],
    );

    let recs: &[xcb::Rectangle] = &[xcb::Rectangle::new(200, 200, 400, 400)];

    xcb::poly_rectangle(&conn, screen.root(), gc, &recs);
    xcb::map_window(&conn, screen.root());
    conn.flush();

    thread::sleep(Duration::from_secs(5));
}
