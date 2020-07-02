/*
 * XXX: here be dragons
 * unsafe code for penrose (primarily x11 stuff) lives here.
 */
use crate::client::Client;
use crate::util::Region;
use libc::c_int;
use std::mem;
use x11::xlib;
use x11::xlib::{CWBorderWidth, CWHeight, CWWidth, CWX, CWY};

// X11 constants and flags
const XCONFIGURE_WINDOW_FLAGS: u32 = (CWX | CWY | CWWidth | CWHeight | CWBorderWidth) as u32;

// window management and events

pub fn unsafe_configure_window(d: *mut xlib::Display, c: &mut Client, w: usize) {
    unsafe {
        xlib::XConfigureWindow(
            d,
            c.x_window,
            XCONFIGURE_WINDOW_FLAGS,
            &mut region_as_window_changes(&c.region, w),
        );
        c.configure();
        xlib::XSync(d, 0);
    }
}

fn region_as_window_changes(r: &Region, w: usize) -> xlib::XWindowChanges {
    xlib::XWindowChanges {
        x: r.x as c_int,
        y: r.y as c_int,
        width: r.w as c_int,
        height: r.h as c_int,
        border_width: w as c_int,
        sibling: 0,
        stack_mode: 0,
    }
}

// wrapper to yield x11 events for a given display
pub struct XEventReader {
    e: xlib::XEvent,
}

impl XEventReader {
    pub fn new() -> XEventReader {
        XEventReader {
            e: unsafe { mem::zeroed() },
        }
    }

    pub fn next(&mut self, d: *mut xlib::Display) -> xlib::XEvent {
        unsafe { xlib::XNextEvent(d, &mut self.e) };
        self.e.clone()
    }
}

// Rust level error handler for the xlib error routine.
unsafe extern "C" fn x_error_handler(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> c_int {
    return 0;
}
