//! The core [`Draw`] and [`Context`] structs for rendering UI elements.
use crate::{Error, Result};
use cairo::{Matrix, Operator, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use pango::{EllipsizeMode, FontDescription, SCALE};
use pangocairo::functions::{create_layout, show_layout};
use penrose::{
    pure::geometry::Rect,
    x::{WinType, XConn},
    x11rb::XcbConn,
    Color, Xid,
};
use std::collections::HashMap;
use tracing::{debug, info};
use x11rb::{connection::Connection, protocol::xproto::Screen};

// A rust version of XCB's `xcb_visualtype_t` struct for FFI.
// Taken from https://github.com/psychon/x11rb/blob/c3894c092101a16cedf4c45e487652946a3c4284/cairo-example/src/main.rs
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct XcbVisualtypeT {
    pub visual_id: u32,
    pub class: u8,
    pub bits_per_rgb_value: u8,
    pub colormap_entries: u16,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub pad0: [u8; 4],
}

#[derive(Clone, Debug, PartialEq)]
/// A set of styling options for a text string
pub struct TextStyle {
    /// Font name to use for rendering
    pub font: String,
    /// Point size to render the font at
    pub point_size: i32,
    /// Foreground color in 0xRRGGBB format
    pub fg: Color,
    /// Optional background color in 0xRRGGBB format (default to current background if None)
    pub bg: Option<Color>,
    /// Pixel padding around this piece of text
    pub padding: (f64, f64),
}

/// Your application should create a single [`Draw`] struct to manage the windows and surfaces it
/// needs to render your UI. See the [`Context`] struct for how to draw to the surfaces you have
/// created.
#[derive(Debug)]
pub struct Draw {
    /// The underlying [`XConn`] implementation used to communicate with the X server
    pub conn: XcbConn,
    fonts: HashMap<String, FontDescription>,
    surfaces: HashMap<Xid, XCBSurface>,
}

impl Draw {
    /// Construct a new `Draw` instance backed with an [`XcbConn`].
    ///
    /// This method will error if it is unable to establish a connection with the X server.
    pub fn new() -> Result<Self> {
        Ok(Self {
            conn: XcbConn::new()?,
            fonts: HashMap::new(),
            surfaces: HashMap::new(),
        })
    }

    /// Create a new X window and initialise a cairo surface for drawing.
    pub fn new_window(&mut self, ty: WinType, r: Rect, managed: bool) -> Result<Xid> {
        info!(?ty, ?r, %managed, "creating new window");
        let id = self.conn.create_window(ty, r, managed)?;

        debug!("getting screen details");
        let screen = &self.conn.connection().setup().roots[0];

        debug!("creating surface");
        let surface = self.surface(*id, screen, r.w as i32, r.h as i32)?;
        self.surfaces.insert(id, surface);

        Ok(id)
    }

    fn surface(&self, id: u32, screen: &Screen, w: i32, h: i32) -> Result<XCBSurface> {
        let mut visual = self.find_xcb_visualtype(screen.root_visual);

        let surface = unsafe {
            debug!(%id, "calling cairo::XCBSurface::create");
            cairo::XCBSurface::create(
                &XCBConnection::from_raw_none(self.conn.connection().get_raw_xcb_connection() as _),
                &XCBDrawable(id),
                &XCBVisualType::from_raw_none(&mut visual as *mut _ as _),
                w,
                h,
            )?
        };

        debug!(%id, "setting surface size");
        surface.set_size(w, h)?;

        Ok(surface)
    }

    fn find_xcb_visualtype(&self, visual_id: u32) -> XcbVisualtypeT {
        for root in &self.conn.connection().setup().roots {
            for depth in &root.allowed_depths {
                for visual in &depth.visuals {
                    if visual.visual_id == visual_id {
                        return XcbVisualtypeT {
                            visual_id: visual.visual_id,
                            class: visual.class.into(),
                            bits_per_rgb_value: visual.bits_per_rgb_value,
                            colormap_entries: visual.colormap_entries,
                            red_mask: visual.red_mask,
                            green_mask: visual.green_mask,
                            blue_mask: visual.blue_mask,
                            pad0: [0; 4],
                        };
                    }
                }
            }
        }

        panic!("unable to find XCB visual type")
    }

    /// Register a new font by name in the font cache so it can be used in a drawing [`Context`].
    pub fn register_font(&mut self, font_name: &str) {
        let description = FontDescription::from_string(font_name);
        self.fonts.insert(font_name.into(), description);
    }

    /// Retrieve the drawing [`Context`] for the given window [`Xid`].
    ///
    /// This method will error if the requested id does not already have an initialised surface.
    /// See the [`new_window`] method for details.
    pub fn context_for(&self, id: Xid) -> Result<Context> {
        let ctx = cairo::Context::new(
            self.surfaces
                .get(&id)
                .ok_or(Error::UnintialisedSurface { id })?,
        )?;

        Ok(Context {
            ctx,
            font: None,
            fonts: self.fonts.clone(),
        })
    }

    /// Construct a disposable context of the specified dimensions without requiring a
    /// window [`Xid`].
    pub fn temp_context(&self, w: i32, h: i32) -> Result<Context> {
        let screen = &self.conn.connection().setup().roots[0];
        let surface = self.surface(*self.conn.root(), screen, w, h)?;
        let surface = surface.create_similar(cairo::Content::Color, w, h)?;
        let ctx = cairo::Context::new(&surface)?;

        Ok(Context {
            ctx,
            font: None,
            fonts: self.fonts.clone(),
        })
    }

    /// Flush any pending requests to the X server and map the specifed window to the screen.
    pub fn flush(&self, id: Xid) -> Result<()> {
        if let Some(s) = self.surfaces.get(&id) {
            s.flush()
        };

        self.conn.map(id)?;
        self.conn.flush();

        Ok(())
    }
}

/// A minimal drawing context for rendering text based UI elements to a cairo surface.
#[derive(Clone, Debug)]
pub struct Context {
    ctx: cairo::Context,
    font: Option<FontDescription>,
    fonts: HashMap<String, FontDescription>,
}

impl Context {
    /// Set the current font by name.
    ///
    /// This method will error if the font has not previously been registered in the parent
    /// [`Draw`] struct.
    pub fn font(&mut self, font_name: &str, point_size: i32) -> Result<()> {
        let mut font = self
            .fonts
            .get_mut(font_name)
            .ok_or_else(|| Error::UnknownFont {
                font: font_name.into(),
            })?
            .clone();

        font.set_size(point_size * SCALE);
        self.font = Some(font);

        Ok(())
    }

    /// Set the active color for following draw operations.
    pub fn color(&mut self, color: &Color) {
        let (r, g, b, a) = color.rgba();
        self.ctx.set_source_rgba(r, g, b, a);
    }

    /// Clear the underlying [`cairo::Context`].
    pub fn clear(&mut self) -> Result<()> {
        self.ctx.save()?;
        self.ctx.set_operator(Operator::Clear);
        self.ctx.paint()?;
        self.ctx.restore()?;

        Ok(())
    }

    /// Translate the underlying [`cairo::Context`] by a specified offset
    pub fn translate(&self, dx: f64, dy: f64) {
        self.ctx.translate(dx, dy)
    }

    /// Set the x offset of the underlying [`cairo::Context`] without modifying the
    /// current y position.
    pub fn set_x_offset(&self, x: f64) {
        let (_, y_offset) = self.ctx.matrix().transform_point(0.0, 0.0);
        self.ctx.set_matrix(Matrix::identity());
        self.ctx.translate(x, y_offset);
    }

    /// Set the y offset of the underlying [`cairo::Context`] without modifying the
    /// current x position.
    pub fn set_y_offset(&self, y: f64) {
        let (x_offset, _) = self.ctx.matrix().transform_point(0.0, 0.0);
        self.ctx.set_matrix(Matrix::identity());
        self.ctx.translate(x_offset, y);
    }

    /// Fill the specified area with the currently active color.
    pub fn rectangle(&self, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        self.ctx.rectangle(x, y, w, h);
        self.ctx.fill()?;

        Ok(())
    }

    /// Draw the specified text using the currently active font and color.
    ///
    /// Returns the width and height of the area taken up by the text.
    pub fn text(&self, txt: &str, h_offset: f64, padding: (f64, f64)) -> Result<(f64, f64)> {
        let layout = create_layout(&self.ctx).ok_or(Error::UnableToCreateLayout)?;
        if let Some(ref font) = self.font {
            layout.set_font_description(Some(font));
        }

        layout.set_text(txt);
        layout.set_ellipsize(EllipsizeMode::End);

        let (w, h) = layout.pixel_size();
        let (l, r) = padding;
        self.ctx.translate(l, h_offset);
        show_layout(&self.ctx, &layout);
        self.ctx.translate(-l, -h_offset);

        let width = w as f64 + l + r;
        let height = h as f64;

        Ok((width, height))
    }

    /// Determine the width and height required to render a specific piece of text
    /// using the current font without rendering it to the underlying [`cairo::Context`].
    pub fn text_extent(&self, s: &str) -> Result<(f64, f64)> {
        let layout = create_layout(&self.ctx).ok_or(Error::UnableToCreateLayout)?;
        if let Some(ref font) = self.font {
            layout.set_font_description(Some(font));
        }
        layout.set_text(s);
        let (w, h) = layout.pixel_size();

        Ok((w as f64, h as f64))
    }

    /// Flush pending requests to the X server.
    pub fn flush(&self) {
        self.ctx.target().flush();
    }
}
