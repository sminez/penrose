use crate::{core::SCREEN, Error, Result};
use fontconfig_sys::{
    constants::{FC_CHARSET, FC_SCALABLE},
    FcCharSetAddChar, FcCharSetCreate, FcCharSetDestroy, FcConfig, FcConfigSubstitute,
    FcDefaultSubstitute, FcMatchPattern, FcPatternAddBool, FcPatternAddCharSet, FcPatternDestroy,
    FcPatternDuplicate,
};
use std::{
    alloc::{alloc, handle_alloc_error, Layout},
    collections::HashMap,
    ffi::CString,
};
use x11::{
    xft::{
        FcPattern, FcResult, XftCharExists, XftFont, XftFontClose, XftFontMatch, XftFontOpenName,
        XftFontOpenPattern, XftNameParse, XftTextExtentsUtf8,
    },
    xlib::Display,
    xrender::XGlyphInfo,
};

#[derive(Debug)]
pub(crate) struct Fontset {
    dpy: *mut Display,
    primary: Font,
    fallback: Vec<Font>,
    char_cache: HashMap<char, FontMatch>,
}

impl Fontset {
    pub(crate) fn try_new(dpy: *mut Display, fnt: &str) -> Result<Self> {
        Ok(Self {
            dpy,
            primary: Font::try_new_from_name(dpy, fnt)?,
            fallback: Default::default(),
            char_cache: Default::default(),
        })
    }

    // Find boundaries where we need to change the font we are using for rendering utf8
    // characters from the given input.
    pub(crate) fn per_font_chunks<'a>(&mut self, txt: &'a str) -> Vec<(&'a str, FontMatch)> {
        let mut char_indices = txt.char_indices();
        let mut chunks = Vec::new();
        let mut last_split = 0;
        let mut chunk: &str;
        let mut rest = txt;

        let mut cur_fm = match char_indices.next() {
            Some((_, c)) => self.fnt_for_char(c),
            None => return chunks, // empty string: no chunks
        };

        for (i, c) in char_indices {
            let fm = self.fnt_for_char(c);
            if fm != cur_fm {
                (chunk, rest) = rest.split_at(i - last_split);
                chunks.push((chunk, cur_fm));
                cur_fm = fm;
                last_split = i;
            }
        }

        if !rest.is_empty() {
            chunks.push((rest, cur_fm));
        }

        chunks
    }

    pub(crate) fn fnt(&self, fm: FontMatch) -> &Font {
        match fm {
            FontMatch::Primary => &self.primary,
            FontMatch::Fallback(n) => &self.fallback[n],
        }
    }

    fn fnt_for_char(&mut self, c: char) -> FontMatch {
        if let Some(fm) = self.char_cache.get(&c) {
            return *fm;
        }

        if self.primary.contains_char(self.dpy, c) {
            self.char_cache.insert(c, FontMatch::Primary);
            return FontMatch::Primary;
        }

        for (i, fnt) in self.fallback.iter().enumerate() {
            if fnt.contains_char(self.dpy, c) {
                self.char_cache.insert(c, FontMatch::Fallback(i));
                return FontMatch::Fallback(i);
            }
        }

        let fallback = match self.primary.fallback_for_char(self.dpy, c) {
            Ok(fnt) => {
                self.fallback.push(fnt);
                FontMatch::Fallback(self.fallback.len() - 1)
            }

            Err(e) => {
                // TODO: add tracing to this crate
                println!("ERROR: {e}");
                FontMatch::Primary
            }
        };

        self.char_cache.insert(c, fallback);

        fallback
    }
}

impl Drop for Fontset {
    fn drop(&mut self) {
        // SAFETY: the Display we have a pointer to is freed by the parent draw
        unsafe {
            XftFontClose(self.dpy, self.primary.xfont);
            for f in self.fallback.drain(0..) {
                XftFontClose(self.dpy, f.xfont);
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FontMatch {
    Primary,
    Fallback(usize),
}

// Fonts contain a resource that requires a Display to free on Drop so they
// are owned by their parent Draw and cleaned up when the Draw is dropped
//
// https://man.archlinux.org/man/extra/libxft/XftFontMatch.3.en
// https://refspecs.linuxfoundation.org/fontconfig-2.6.0/index.html
#[derive(Debug)]
pub(crate) struct Font {
    pub(crate) h: u32,
    pub(crate) xfont: *mut XftFont,
    pattern: *mut FcPattern,
}

impl Font {
    fn try_new_from_name(dpy: *mut Display, name: &str) -> Result<Self> {
        let c_name = CString::new(name)?;

        // SAFETY:
        // - Null pointers are checked and explicitly converted to Rust Errors
        // - Raw pointer dereferences are only carried out after checking for null pointers
        let (xfont, pattern, h) = unsafe {
            let xfont = XftFontOpenName(dpy, SCREEN, c_name.as_ptr());
            if xfont.is_null() {
                return Err(Error::UnableToOpenFont(name.to_string()));
            }

            let pattern = XftNameParse(c_name.as_ptr());
            if pattern.is_null() {
                XftFontClose(dpy, xfont);
                return Err(Error::UnableToParseFontPattern(name.to_string()));
            }

            let h = (*xfont).ascent + (*xfont).descent;

            (xfont, pattern, h as u32)
        };

        Ok(Font { xfont, pattern, h })
    }

    fn try_new_from_pattern(dpy: *mut Display, pattern: *mut FcPattern) -> Result<Self> {
        // SAFETY:
        // - Null pointers are checked and explicitly converted to Rust Errors
        // - Raw pointer dereferences are only carried out after checking for null pointers
        let (xfont, h) = unsafe {
            let xfont = XftFontOpenPattern(dpy, pattern);
            if xfont.is_null() {
                return Err(Error::UnableToOpenFontPattern);
            }

            let h = (*xfont).ascent + (*xfont).descent;

            (xfont, h as u32)
        };

        Ok(Font { xfont, pattern, h })
    }

    fn contains_char(&self, dpy: *mut Display, c: char) -> bool {
        // SAFETY: self.xfont is known to be non-null
        unsafe { XftCharExists(dpy, self.xfont, c as u32) == 1 }
    }

    pub(crate) fn get_exts(&self, dpy: *mut Display, txt: &str) -> Result<(u32, u32)> {
        // SAFETY:
        // - allocation failures are explicitly handled
        // - invalid C strings are converted to Rust Errors
        // - self.xfont is known to be non-null
        unsafe {
            // https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#tymethod.alloc
            let layout = Layout::new::<XGlyphInfo>();
            let ptr = alloc(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            let ext = ptr as *mut XGlyphInfo;

            let c_str = CString::new(txt)?;
            XftTextExtentsUtf8(
                dpy,
                self.xfont,
                c_str.as_ptr() as *mut u8,
                c_str.as_bytes().len() as i32,
                ext,
            );

            Ok(((*ext).xOff as u32, self.h))
        }
    }

    /// Find a font that can handle a given character using fontconfig and this font's pattern
    fn fallback_for_char(&self, dpy: *mut Display, c: char) -> Result<Self> {
        let pat = self.fc_font_match(dpy, c)?;

        Font::try_new_from_pattern(dpy, pat)
    }

    fn fc_font_match(&self, dpy: *mut Display, c: char) -> Result<*mut FcPattern> {
        // SAFETY:
        // - allocation failures are explicitly handled
        // - Null pointers are checked and explicitly converted to Rust Errors
        // - valid constant values from the fontconfig_sys crate are used for C string parameters
        // - null pointer parameter for FcConfigSubstutute config param (first argument) is valid
        //   as documented here: https://man.archlinux.org/man/extra/fontconfig/FcConfigSubstitute.3.en
        unsafe {
            let charset = FcCharSetCreate();
            FcCharSetAddChar(charset, c as u32);

            let pat = FcPatternDuplicate(self.pattern as *const _);
            FcPatternAddCharSet(pat, FC_CHARSET.as_ptr(), charset);
            FcPatternAddBool(pat, FC_SCALABLE.as_ptr(), 1); // FcTrue=1

            FcConfigSubstitute(std::ptr::null::<FcConfig>() as *mut _, pat, FcMatchPattern);
            FcDefaultSubstitute(pat);

            // https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#tymethod.alloc
            let layout = Layout::new::<FcResult>();
            let ptr = alloc(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            let res = ptr as *mut FcResult;

            // Passing the pointer from fontconfig_sys to x11 here
            let font_match = XftFontMatch(dpy, SCREEN, pat as *const _, res);

            FcCharSetDestroy(charset);
            FcPatternDestroy(pat);

            if font_match.is_null() {
                Err(Error::NoFallbackFontForChar(c))
            } else {
                Ok(font_match as *mut _)
            }
        }
    }
}
