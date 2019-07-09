extern crate x11;

use std::ffi::CString;
use std::process;
use x11::{xft, xlib, xrender};

/// A color scheme is simply a foreground, background and border color
pub struct ColorScheme {
    pub fg: xft::XftColor,
    pub bg: xft::XftColor,
    pub border: xft::XftColor,
}

impl ColorScheme {
    /// Create a new color scheme from color names. This can fail and exit the process due to
    /// errors in wrapped unsafe code.
    pub fn new(display: &mut xlib::Display, screen: i32, fg_name: &str, bg_name: &str, border_name: &str) -> ColorScheme {
        let fg = color_from_name(display, screen, fg_name);
        let bg = color_from_name(display, screen, bg_name);
        let border = color_from_name(display, screen, border_name);

        ColorScheme { fg, bg, border }
    }
}

// Create a new XftColor instance for a given color name
fn color_from_name(display: &mut xlib::Display, screen: i32, name: &str) -> xft::XftColor {
    let mut xftColor = xft::XftColor {
        pixel: 0,
        color: xrender::XRenderColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
    };

    let retCode = unsafe {
        xft::XftColorAllocName(
            display,
            xlib::XDefaultVisual(display, screen),
            xlib::XDefaultColormap(display, screen),
            CString::new(name).unwrap().as_ptr(),
            &mut xftColor,
        )
    };

    if retCode == 0 {
        eprintln!("ERROR - unable to alocate color: {:?}\n", name);
        process::exit(1);
    }

    xftColor
}
