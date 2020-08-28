//! Utility functions for use in other parts of penrose
use crate::{
    data_types::{CodeMap, KeyCode},
    Result,
};

use std::{
    io::Read,
    process::{Command, Stdio},
};

use anyhow::anyhow;
use xcb;

/**
 * Run an external command
 *
 * This redirects the process stdout and stderr to /dev/null.
 * Logs a warning if there were any errors in kicking off the process.
 */
pub fn spawn<S: Into<String>>(cmd: S) {
    let s = cmd.into();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .args(&parts[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new(parts[0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    };

    if let Err(e) = result {
        warn!("error spawning external program: {}", e);
    }
}

/**
 * Run an external command and return its output.
 *
 * NOTE: std::process::Command::output will not work within penrose due to the
 * way that signal handling is set up. Use this function if you need to access the
 * output of a process that you spawn.
 */
pub fn spawn_for_output<S: Into<String>>(cmd: S) -> Result<String> {
    let s = cmd.into();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .stdout(std::process::Stdio::piped())
            .args(&parts[1..])
            .spawn()
    } else {
        Command::new(parts[0])
            .stdout(std::process::Stdio::piped())
            .spawn()
    };

    let child = result?;
    let mut buff = String::new();
    Ok(child
        .stdout
        .ok_or_else(|| anyhow!("unable to get stdout for child process: {}", s))?
        .read_to_string(&mut buff)
        .map(|_| buff)?)
}

/**
 * Run the xmodmap command to dump the system keymap table.
 *
 * This is done in a form that we can load in and convert back to key
 * codes. This lets the user define key bindings in the way that they
 * would expect while also ensuring that it is east to debug any odd
 * issues with bindings by referring the user to the xmodmap output.
 */
pub fn keycodes_from_xmodmap() -> CodeMap {
    match Command::new("xmodmap").arg("-pke").output() {
        Err(e) => panic!("unable to fetch keycodes via xmodmap: {}", e),
        Ok(o) => match String::from_utf8(o.stdout) {
            Err(e) => panic!("invalid utf8 from xmodmap: {}", e),
            Ok(s) => s
                .lines()
                .flat_map(|l| {
                    let mut words = l.split_whitespace(); // keycode <code> = <names ...>
                    let key_code: u8 = words.nth(1).unwrap().parse().unwrap();
                    words.skip(1).map(move |name| (name.into(), key_code))
                })
                .collect::<CodeMap>(),
        },
    }
}

/**
 * Convert user friendly key bindings into X keycodes.
 *
 * Allows the user to define their keybindings using the gen_keybindings macro
 * which calls through to this. Bindings are of the form '<MOD>-<key name>'
 * with multipple modifiers being allowed, and keynames being taken from the
 * output of 'xmodmap -pke'.
 *
 * Allowed modifiers are:
 *   M - Super
 *   A - Alt
 *   C - Ctrl
 *   S - Shift
 *
 * The user friendly patterns are parsed into a modifier mask and X key code
 * pair that is then grabbed by penrose to trigger the bound action.
 */
pub fn parse_key_binding(pattern: impl Into<String>, known_codes: &CodeMap) -> Option<KeyCode> {
    let s = pattern.into();
    let mut parts: Vec<&str> = s.split("-").collect();
    match known_codes.get(parts.remove(parts.len() - 1)) {
        Some(code) => {
            let mask = parts
                .iter()
                .map(|s| match s {
                    &"A" => xcb::MOD_MASK_1,
                    &"M" => xcb::MOD_MASK_4,
                    &"S" => xcb::MOD_MASK_SHIFT,
                    &"C" => xcb::MOD_MASK_CONTROL,
                    &_ => panic!("invalid key binding prefix: {}", s),
                })
                .fold(0, |acc, v| acc | v);

            debug!("binding '{}' as [{}, {}]", s, mask, code);
            Some(KeyCode {
                mask: mask as u16,
                code: *code,
            })
        }
        None => None,
    }
}

// Helper functions for XCB based operations
pub(crate) mod xcb_util {
    use crate::{data_types::Region, Result};
    use anyhow::anyhow;
    use xcb;

    pub fn intern_atom(conn: &xcb::Connection, name: &str) -> Result<u32> {
        xcb::intern_atom(conn, false, name)
            .get_reply()
            .map(|r| r.atom())
            .map_err(|err| anyhow!("unable to intern xcb atom '{}': {}", name, err))
    }

    pub fn create_window(
        conn: &xcb::Connection,
        screen: &xcb::Screen,
        window_type: &str,
        x: i16,
        y: i16,
        w: u16,
        h: u16,
    ) -> Result<u32> {
        let id = conn.generate_id();

        xcb::create_window(
            &conn,
            xcb::COPY_FROM_PARENT as u8,
            id,
            screen.root(),
            x,
            y,
            w,
            h,
            0,
            xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
            0,
            &[
                (xcb::CW_BACK_PIXEL, screen.black_pixel()),
                (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
            ],
        );

        xcb::change_property(
            &conn,                                      // xcb connection to X11
            xcb::PROP_MODE_REPLACE as u8,               // discard current prop and replace
            id,                                         // window to change prop on
            intern_atom(&conn, "_NET_WM_WINDOW_TYPE")?, // prop to change
            intern_atom(&conn, "UTF8_STRING")?,         // type of prop
            8,                                          // data format (8/16/32-bit)
            window_type.as_bytes(),                     // data
        );

        xcb::map_window(&conn, id);
        conn.flush();

        Ok(id)
    }

    pub fn get_visual_type(
        conn: &xcb::Connection,
        screen: &xcb::Screen,
    ) -> Result<xcb::Visualtype> {
        conn.get_setup()
            .roots()
            .flat_map(|r| r.allowed_depths())
            .flat_map(|d| d.visuals())
            .find(|v| v.visual_id() == screen.root_visual())
            .ok_or_else(|| anyhow!("unable to get screen visual type"))
    }

    pub fn screen_sizes(conn: &xcb::Connection) -> Result<Vec<Region>> {
        // If we can't unwrap here then there is no screen(!)
        let root = conn.get_setup().roots().nth(0).unwrap().root();
        let check_win = conn.generate_id();
        let class = xcb::xproto::WINDOW_CLASS_INPUT_ONLY as u16;
        xcb::create_window(conn, 0, check_win, root, 0, 0, 1, 1, 0, class, 0, &[]);
        conn.flush();

        let res = xcb::randr::get_screen_resources(conn, check_win)
            .get_reply()
            .map_err(|e| anyhow!("unable to read randr screen resources: {}", e))?
            .crtcs()
            .iter()
            .flat_map(|c| xcb::randr::get_crtc_info(conn, *c, 0).get_reply())
            .flat_map(|r| {
                if r.width() > 0 {
                    Some(Region::new(
                        r.x() as u32,
                        r.y() as u32,
                        r.width() as u32,
                        r.height() as u32,
                    ))
                } else {
                    None
                }
            })
            .collect();

        xcb::destroy_window(&conn, check_win);
        Ok(res)
    }
}
