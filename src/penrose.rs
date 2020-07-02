/*
 * Penrose :: A tiling window manager
 */
use std::{env, process};
use xcb;

/*
 * config
 */
const TAGS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
const MOD_KEY: u32 = xcb::MOD_MASK_4;

/*
 * helper functions / macros
 */
macro_rules! die(
    ($msg:expr) => ({
        eprintln!("fatal :: {}", $msg);
        process::exit(42);
     });

    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!("fatal :: {}", format!($fmt, $($arg)*));
        process::exit(42);
     });
);

fn debug(msg: &str) {
    eprintln!("debug :: {}", msg);
}

fn log(msg: &str) {
    eprintln!("info  :: {}", msg);
}

fn error(msg: &str) {
    eprintln!("error :: {}", msg);
}

/*
 * structs
 */
struct Region {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct WindowManager {
    conn: xcb::Connection,
    screen_num: i32,
    screen_dimensions: Vec<Region>,
}

impl WindowManager {
    fn new() -> WindowManager {
        let (conn, screen_num) = xcb::Connection::connect(None).unwrap();

        let mut wm = WindowManager {
            conn,
            screen_num,
            screen_dimensions: vec![],
        };

        wm.update_screen_dimensions();
        wm
    }

    fn update_screen_dimensions(&mut self) {
        let screen = match self.conn.get_setup().roots().nth(0) {
            Some(screen) => screen,
            None => die!("unable to get handle for screen"),
        };

        let win_id = self.conn.generate_id();
        let root = screen.root();

        // TODO: add a comment on what the args for this are
        xcb::create_window(&self.conn, 0, win_id, root, 0, 0, 1, 1, 0, 0, 0, &[]);
        let resources = xcb::randr::get_screen_resources(&self.conn, win_id);

        // TODO: add a comment on what this is doing
        self.screen_dimensions = match resources.get_reply() {
            Err(e) => die!("error reading X screen resources: {}", e),
            Ok(reply) => reply
                .crtcs()
                .iter()
                .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
                .map(|r| Region {
                    x: r.x() as usize,
                    y: r.y() as usize,
                    w: r.width() as usize,
                    h: r.height() as usize,
                })
                .filter(|r| r.w > 0)
                .collect(),
        };
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && &args[1] == "-v" {
        println!("penrose-{}", option_env!("CARGO_PKG_VERSION").unwrap());
        process::exit(0);
    } else if args.len() > 1 {
        println!("usage: penrose [-v]");
        process::exit(1);
    }

    let wm = WindowManager::new();
}
