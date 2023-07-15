//! The core [`Draw`] and [`Context`] structs for rendering UI elements.
use crate::{Error, Result};
use penrose::{
    pure::geometry::Rect,
    x::{WinType, XConn},
    x11rb::RustConn,
    Color, Xid,
};
use std::{
    alloc::{alloc, dealloc, handle_alloc_error, Layout},
    cmp::max,
    collections::HashMap,
    ffi::CString,
};
use tracing::{debug, info};
use x11::{
    xft::{XftColor, XftColorAllocName, XftDrawCreate, XftDrawStringUtf8},
    xlib::{
        CapButt, Display, Drawable, False, JoinMiter, LineSolid, Window, XCopyArea, XCreateGC,
        XCreatePixmap, XDefaultColormap, XDefaultDepth, XDefaultVisual, XDrawRectangle,
        XFillRectangle, XFreeGC, XFreePixmap, XOpenDisplay, XSetForeground, XSetLineAttributes,
        XSync, GC,
    },
};

mod fontset;
use fontset::Fontset;

pub(crate) const SCREEN: i32 = 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// A set of styling options for a text string
pub struct TextStyle {
    /// Foreground color in 0xRRGGBB format
    pub fg: Color,
    /// Optional background color in 0xRRGGBB format (default to current background if None)
    pub bg: Option<Color>,
    /// Pixel padding around this piece of text
    pub padding: (u32, u32),
}

#[derive(Debug)]
struct Surface {
    drawable: Drawable,
    gc: GC,
    r: Rect,
}

/// Your application should create a single [`Draw`] struct to manage the windows and surfaces it
/// needs to render your UI. See the [`Context`] struct for how to draw to the surfaces you have
/// created.
#[derive(Debug)]
pub struct Draw {
    /// The underlying [`XConn`] implementation used to communicate with the X server
    pub conn: RustConn,
    dpy: *mut Display,
    fs: Fontset,
    bg: Color,
    surfaces: HashMap<Xid, Surface>,
    colors: HashMap<Color, XColor>,
}

impl Drop for Draw {
    fn drop(&mut self) {
        unsafe {
            for (_, s) in self.surfaces.drain() {
                XFreePixmap(self.dpy, s.drawable);
                XFreeGC(self.dpy, s.gc);
            }
        }
    }
}

impl Draw {
    /// Construct a new `Draw` instance backed with an [`RustConn`].
    ///
    /// This method will error if it is unable to establish a connection with the X server.
    pub fn new(font: &str, point_size: u8, bg: Color) -> Result<Self> {
        let conn = RustConn::new()?;
        let dpy = unsafe { XOpenDisplay(std::ptr::null()) };
        let mut colors = HashMap::new();
        colors.insert(bg, XColor::try_new(dpy, &bg)?);

        Ok(Self {
            conn,
            dpy,
            fs: Fontset::try_new(dpy, &format!("{font}:size={point_size}"))?,
            surfaces: HashMap::new(),
            bg,
            colors,
        })
    }

    /// Create a new X window with an initialised surface for drawing
    pub fn new_window(&mut self, ty: WinType, r: Rect, managed: bool) -> Result<Xid> {
        info!(?ty, ?r, %managed, "creating new window");
        let id = self.conn.create_window(ty, r, managed)?;

        debug!("initialising graphics context and pixmap");
        let root = *self.conn.root() as Window;
        let (drawable, gc) = unsafe {
            let depth = XDefaultDepth(self.dpy, SCREEN) as u32;
            let drawable = XCreatePixmap(self.dpy, root, r.w, r.h, depth);
            let gc = XCreateGC(self.dpy, root, 0, std::ptr::null_mut());
            XSetLineAttributes(self.dpy, gc, 1, LineSolid, CapButt, JoinMiter);

            (drawable, gc)
        };

        self.surfaces.insert(id, Surface { r, gc, drawable });

        Ok(id)
    }

    /// Register a new font by name in the font cache so it can be used in a drawing [`Context`].
    pub fn set_font(&mut self, font: &str) -> Result<()> {
        self.fs = Fontset::try_new(self.dpy, font)?;

        Ok(())
    }

    /// Retrieve the drawing [`Context`] for the given window [`Xid`].
    ///
    /// This method will error if the requested id does not already have an initialised surface.
    /// See the [`new_window`] method for details.
    pub fn context_for(&mut self, id: Xid) -> Result<Context<'_>> {
        let s = self
            .surfaces
            .get(&id)
            .ok_or(Error::UnintialisedSurface { id })?;

        Ok(Context {
            id: *id as u64,
            dx: 0,
            dy: 0,
            dpy: self.dpy,
            s,
            bg: self.bg,
            fs: &mut self.fs,
            colors: &mut self.colors,
        })
    }

    /// Flush any pending requests to the X server and map the specifed window to the screen.
    pub fn flush(&self, id: Xid) -> Result<()> {
        if let Some(s) = self.surfaces.get(&id) {
            let Rect { x, y, w, h } = s.r;
            let (x, y) = (x as i32, y as i32);

            unsafe {
                XCopyArea(self.dpy, s.drawable, *id as u64, s.gc, x, y, w, h, x, y);
                XSync(self.dpy, False);
            }
        };

        self.conn.map(id)?;
        self.conn.flush();

        Ok(())
    }
}

/// A minimal drawing context for rendering text based UI elements
#[derive(Debug)]
pub struct Context<'a> {
    id: u64,
    dx: i32,
    dy: i32,
    dpy: *mut Display,
    s: &'a Surface,
    bg: Color,
    fs: &'a mut Fontset,
    colors: &'a mut HashMap<Color, XColor>,
}

impl<'a> Context<'a> {
    /// Clear the underlying surface, restoring it to the background color.
    pub fn clear(&mut self) -> Result<()> {
        self.fill_rect(Rect::new(0, 0, self.s.r.w, self.s.r.h), self.bg)
    }

    /// Offset future drawing operations by an additional (dx, dy)
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.dx += dx;
        self.dy += dy;
    }

    /// Set future drawing operations to apply from the origin.
    pub fn reset_offset(&mut self) {
        self.dx = 0;
        self.dy = 0;
    }

    /// Set an absolute x offset for future drawing operations.
    pub fn set_x_offset(&mut self, x: i32) {
        self.dx = x;
    }

    /// Set an absolute y offset for future drawing operations.
    pub fn set_y_offset(&mut self, y: i32) {
        self.dy = y;
    }

    fn get_or_try_init_xcolor(&mut self, c: Color) -> Result<*mut XftColor> {
        Ok(self
            .colors
            .entry(c)
            .or_insert(XColor::try_new(self.dpy, &c)?)
            .0)
    }

    /// Render a rectangular border using the supplied color.
    pub fn draw_rect(&mut self, Rect { x, y, w, h }: Rect, color: Color) -> Result<()> {
        let xcol = self.get_or_try_init_xcolor(color)?;
        let (x, y) = (self.dx + x as i32, self.dy + y as i32);

        unsafe {
            XSetForeground(self.dpy, self.s.gc, (*xcol).pixel);
            XDrawRectangle(self.dpy, self.s.drawable, self.s.gc, x, y, w, h);
        }

        Ok(())
    }

    /// Render a filled rectangle using the supplied color.
    pub fn fill_rect(&mut self, Rect { x, y, w, h }: Rect, color: Color) -> Result<()> {
        let xcol = self.get_or_try_init_xcolor(color)?;
        let (x, y) = (self.dx + x as i32, self.dy + y as i32);

        unsafe {
            XSetForeground(self.dpy, self.s.gc, (*xcol).pixel);
            XFillRectangle(self.dpy, self.s.drawable, self.s.gc, x, y, w, h);
        }

        Ok(())
    }

    // TODO: Need to bounds checks
    // https://keithp.com/~keithp/talks/xtc2001/xft.pdf
    // https://keithp.com/~keithp/render/Xft.tutorial
    //
    /// Render the provided text at the current context offset using the supplied color.
    pub fn draw_text(
        &mut self,
        txt: &str,
        h_offset: u32,
        padding: (u32, u32),
        c: Color,
    ) -> Result<(u32, u32)> {
        let d = unsafe {
            XftDrawCreate(
                self.dpy,
                self.s.drawable,
                XDefaultVisual(self.dpy, SCREEN),
                XDefaultColormap(self.dpy, SCREEN),
            )
        };

        let (lpad, rpad) = (padding.0 as i32, padding.1);
        let (mut x, y) = (lpad + self.dx, self.dy);
        let (mut total_w, mut total_h) = (x as u32, 0);
        let xcol = self.get_or_try_init_xcolor(c)?;

        for (chunk, fm) in self.fs.per_font_chunks(txt).into_iter() {
            let fnt = self.fs.fnt(fm);
            let (chunk_w, chunk_h) = fnt.get_exts(self.dpy, chunk)?;

            // SAFETY: fnt pointer is non-null
            let chunk_y = unsafe { y + h_offset as i32 + (*fnt.xfont).ascent };
            let c_str = CString::new(chunk)?;

            unsafe {
                XftDrawStringUtf8(
                    d,
                    xcol,
                    fnt.xfont,
                    x,
                    chunk_y,
                    c_str.as_ptr() as *mut _,
                    c_str.as_bytes().len() as i32,
                );
            }

            x += chunk_w as i32;
            total_w += chunk_w;
            total_h = max(total_h, chunk_h);
        }

        Ok((total_w + rpad, total_h))
    }

    /// Determine the width and height taken up by a given string in pixels.
    pub fn text_extent(&mut self, txt: &str) -> Result<(u32, u32)> {
        let (mut w, mut h) = (0, 0);
        for (chunk, fm) in self.fs.per_font_chunks(txt) {
            let (cw, ch) = self.fs.fnt(fm).get_exts(self.dpy, chunk)?;
            w += cw;
            h = max(h, ch);
        }

        Ok((w, h))
    }

    /// Flush pending requests to the X server.
    pub fn flush(&self) {
        let Surface {
            r: Rect { w, h, .. },
            gc,
            drawable,
        } = *self.s;

        unsafe {
            XCopyArea(self.dpy, drawable, self.id, gc, 0, 0, w, h, 0, 0);
            XSync(self.dpy, False);
        }
    }
}

#[derive(Debug)]
struct XColor(*mut XftColor);

impl Drop for XColor {
    fn drop(&mut self) {
        let layout = Layout::new::<XftColor>();
        unsafe { dealloc(self.0 as *mut u8, layout) }
    }
}

impl XColor {
    fn try_new(dpy: *mut Display, c: &Color) -> Result<Self> {
        let inner = unsafe { try_xftcolor_from_name(dpy, &c.as_rgb_hex_string())? };

        Ok(Self(inner))
    }
}

unsafe fn try_xftcolor_from_name(dpy: *mut Display, color: &str) -> Result<*mut XftColor> {
    // https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#tymethod.alloc
    let layout = Layout::new::<XftColor>();
    let ptr = alloc(layout);
    if ptr.is_null() {
        handle_alloc_error(layout);
    }

    let c_name = CString::new(color)?;
    let res = XftColorAllocName(
        dpy,
        XDefaultVisual(dpy, SCREEN),
        XDefaultColormap(dpy, SCREEN),
        c_name.as_ptr(),
        ptr as *mut XftColor,
    );

    if res == 0 {
        Err(Error::UnableToAllocateColor)
    } else {
        Ok(ptr as *mut XftColor)
    }
}
