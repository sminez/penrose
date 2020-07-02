/*
 * penrose :: A tiling window manager in the style of dwm
 */
use std::collections::HashMap;
use std::{env, process};
use xcb;

/*
 * macros
 *
 * NOTE: need to define these at the top of the file for them to be usable by
 *       functions later.
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

macro_rules! warn(
    ($msg:expr) => ({ eprintln!("warn :: {}", $msg); });
    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!("warn :: {}", format!($fmt, $($arg)*));
     });
);

macro_rules! run_external(
    ($cmd:tt) => ({
        let parts: Vec<&str> = $cmd.split_whitespace().collect();
        if parts.len() > 1 {
            Box::new(move || {
                match process::Command::new(parts[0]).args(&parts[1..]).status() {
                    Ok(_) => (),
                    Err(e) => warn!("error running external program: {}", e),
                };
            }) as FireAndForget
        } else {
            Box::new(move || {
                match process::Command::new(parts[0]).status() {
                    Ok(_) => (),
                    Err(e) => warn!("error running external program: {}", e),
                };
            }) as FireAndForget
        }
     });
);

macro_rules! map(
    {} => { ::std::collections::HashMap::new(); };

    { $($key:expr => $value:expr),+, } => {
        {
            let mut _map = ::std::collections::HashMap::new();
            $(_map.insert($key, $value);)+
            _map
        }
     };
);

/*
 * configuration constants
 * most simple changes to penrose behaviour can be made through editing the following constants.
 */
const FONTS: &[&str] = &["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];
const TAGS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
const STARTUP_SCRIPT: &str = "~/bin/scripts/start-dwm.sh";
const COLOR_SCHEME: ColorScheme = ColorScheme {
    bg: "#282828",
    fg_1: "#3c3836",
    fg_2: "#a89984",
    fg_3: "#f2e5bc",
    hl: "#458588",
};
const MOD_KEY: u32 = xcb::MOD_MASK_4;
const BORDER_PX: usize = 2;
const GAP_PX: usize = 6;
const SYSTRAY_SPACING: usize = 2;
const SHOW_SYSTRAY: bool = true;
const SHOW_BAR: bool = true;
const TOP_BAR: bool = true;
const MAIN_RATIO: f32 = 0.60;
const N_MAIN: usize = 1;
const RESPECT_RESIZE_HINTS: bool = true;

fn keybindings() -> HashMap<&'static str, FireAndForget> {
    map! {
        "M-;" => run_external!("rofi-apps"),
    }
}

/*
 * Run the xmodmap command to dump the system keymap table in a form
 * that we can load in and convert back to key codes. This lets the user
 * define key bindings in the way that they would expect while also
 * ensuring that it is east to debug any odd issues with bindings by
 * referring the user to the xmodmap output.
 */
fn keycodes_from_xmodmap() -> HashMap<String, u8> {
    match process::Command::new("xmodmap").arg("-pke").output() {
        Err(e) => die!("unable to fetch keycodes via xmodmap: {}", e),
        Ok(o) => match String::from_utf8(o.stdout) {
            Err(e) => die!("invalid utf8 from xmodmap: {}", e),
            Ok(s) => s
                .lines()
                .flat_map(|l| {
                    let mut words = l.split_whitespace(); // keycode <code> = <names ...>
                    let key_code: u8 = words.nth(1).unwrap().parse().unwrap();
                    words.skip(1).map(move |name| (name.into(), key_code))
                })
                .collect::<HashMap<String, u8>>(),
        },
    }
}

/*
 * structs
 */
#[derive(Clone, Copy)]
struct Region {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct ColorScheme {
    bg: &'static str,
    fg_1: &'static str,
    fg_2: &'static str,
    fg_3: &'static str,
    hl: &'static str,
}

type FireAndForget = Box<dyn Fn() -> ()>;

struct WindowManager {
    conn: xcb::Connection,
    screen_num: i32,
    screen_dims: Vec<Region>,
    screen_tags: Vec<usize>,
    key_bindings: HashMap<&'static str, FireAndForget>,
}

impl WindowManager {
    fn new() -> WindowManager {
        let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
        // let keycodes = keycodes_from_xmodmap();

        let mut wm = WindowManager {
            conn,
            screen_num,
            screen_dims: vec![],
            screen_tags: vec![],
            key_bindings: keybindings(),
        };

        wm.update_screen_dimensions();
        wm.screen_tags = wm.screen_dims.iter().enumerate().map(|(i, _)| i).collect();

        wm
    }

    fn update_screen_dimensions(&mut self) {
        let screen = match self.conn.get_setup().roots().nth(0) {
            None => die!("unable to get handle for screen"),
            Some(s) => s,
        };

        let win_id = self.conn.generate_id();
        let root = screen.root();

        // TODO: add a comment on what the args for this are
        xcb::create_window(&self.conn, 0, win_id, root, 0, 0, 1, 1, 0, 0, 0, &[]);
        let resources = xcb::randr::get_screen_resources(&self.conn, win_id);

        // TODO: add a comment on what this is doing
        self.screen_dims = match resources.get_reply() {
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

    fn run(&mut self) {
        let f = run_external!("rofi-apps");
        f();
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

    let mut wm = WindowManager::new();
    wm.run();
}
