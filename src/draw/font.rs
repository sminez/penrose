extern crate x11;

use std::ffi::CString;
use std::process;

use x11::{xft, xlib, xrender};

/// Font extent (width and height)
pub struct Extent {
    pub w: u32,
    pub h: u32,
}

/// An xft font and associated metadata
pub struct Font {
    pub ascent: i32,
    pub descent: i32,
    pub h: u32,
    pub xfont: *mut xft::XftFont,
    pub pattern: *mut xft::FcPattern,
}

impl PartialEq for Font {
    fn eq(&self, other: &Font) -> bool {
        self.xfont == other.xfont
    }
}

impl Font {
    /// Create a new Font from an Xft font name
    pub fn new_from_name(display: &mut xlib::Display, screen: i32, name: &str) -> Option<Font> {
        let cstr_name = CString::new(name).unwrap();
        let xfont = unsafe { xft::XftFontOpenName(display, screen, cstr_name.as_ptr()) };

        if xfont.is_null() {
            eprintln!("ERROR - unable to load font: {:?}\n", name);
            None
        } else {
            let pattern = unsafe { xft::XftNameParse(cstr_name.as_ptr()) };
            if pattern.is_null() {
                eprintln!("ERROR - unable to load font: {:?}\n", name);
                process::exit(1);
            }
            unsafe {
                Some(Font {
                    ascent: (*xfont).ascent,
                    descent: (*xfont).descent,
                    h: ((*xfont).ascent + (*xfont).descent) as u32,
                    xfont: xfont,
                    pattern: pattern,
                })
            }
        }
    }

    /// Create a new Font from an existing FcPattern
    pub fn new_from_pattern(display: &mut xlib::Display, pattern: &mut xft::FcPattern) -> Option<Font> {
        let xfont = unsafe { xft::XftFontOpenPattern(display, pattern) };

        return if !xfont.is_null() {
            eprintln!("ERROR - unable to load font pattern\n");
            process::exit(1);
        } else {
            unsafe {
                Some(Font {
                    ascent: (*xfont).ascent,
                    descent: (*xfont).descent,
                    h: ((*xfont).ascent + (*xfont).descent) as u32,
                    xfont: xfont,
                    pattern: pattern,
                })
            }
        };
    }

    /// Use xft to free this font
    pub fn unsafe_font_close(&self, display: &mut xlib::Display) {
        unsafe { xft::XftFontClose(display, self.xfont) };
    }

    /// Update (mutate) the given Extent based on this font
    pub fn set_extent(&self, display: &mut xlib::Display, text: Vec<u8>, extent: &mut Extent) {
        let mut dummy_info = xrender::XGlyphInfo {
            height: 0,
            width: 0,
            x: 0,
            y: 0,
            xOff: 0,
            yOff: 0,
        };
        unsafe { xft::XftTextExtentsUtf8(display, self.xfont, text.as_ptr(), text.len() as i32, &mut dummy_info) }

        extent.h = self.h;
        extent.w = dummy_info.xOff as u32;
    }
}
