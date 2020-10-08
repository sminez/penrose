//! Utilities for rendering custom windows
pub mod bar;

pub use bar::*;
pub use inner::{Color, Draw, DrawContext, TextStyle, WindowType, XCBDraw, XCBDrawContext};

mod inner {
    use std::{collections::HashMap, convert::TryFrom, convert::TryInto};

    use crate::{
        core::data_types::{Region, WinId},
        core::helpers::xcb_util,
        Result,
    };

    use anyhow::anyhow;
    use cairo::{Context, XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
    use pango::{EllipsizeMode, FontDescription, Layout};
    use pangocairo::functions::{create_layout, show_layout};

    fn pango_layout(ctx: &Context) -> Result<Layout> {
        create_layout(ctx).ok_or_else(|| anyhow!("unable to create pango layout"))
    }

    fn new_cairo_surface(
        conn: &xcb::Connection,
        screen: &xcb::Screen,
        wt: &WindowType,
        x: i16,
        y: i16,
        w: i32,
        h: i32,
    ) -> Result<(u32, XCBSurface)> {
        let id = xcb_util::create_window(conn, screen, wt.as_ewmh_str(), x, y, w as u16, h as u16)?;

        let depth = xcb_util::get_depth(screen)?;
        let mut visualtype = xcb_util::get_visual_type(&depth)?;

        let surface = unsafe {
            let conn_ptr = conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t;

            XCBSurface::create(
                &XCBConnection::from_raw_none(conn_ptr),
                &XCBDrawable(id),
                &XCBVisualType::from_raw_none(
                    &mut visualtype.base as *mut xcb::ffi::xcb_visualtype_t
                        as *mut cairo_sys::xcb_visualtype_t,
                ),
                w,
                h,
            )
            .map_err(|err| anyhow!("Error creating surface: {}", err))?
        };

        surface.set_size(w, h).unwrap();
        Ok((id, surface))
    }

    #[derive(Clone, Debug, PartialEq)]
    /// A set of styling options for a text string
    pub struct TextStyle<'s> {
        /// Pango font name to use for rendering
        pub font: &'s str,
        /// Point size to render the font at
        pub point_size: i32,
        /// Foreground color in 0xRRGGBB format
        pub fg: Color,
        /// Optional background color in 0xRRGGBB format (default to current background if None)
        pub bg: Option<Color>,
        /// Pixel padding around this string
        pub padding: (f64, f64),
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    /// A simple RGBA based color
    pub struct Color {
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    }
    impl Color {
        /// Create a new Color from a hex encoded u32: 0xRRGGBB or 0xRRGGBBAA
        pub fn new_from_hex(hex: u32) -> Self {
            // TODO: double check if this produces correct results when alpha is omitted.
            let floats: Vec<f64> = hex
                .to_be_bytes()
                .iter()
                .map(|n| *n as f64 / 255.0)
                .collect();

            let (r, g, b, a) = (floats[0], floats[1], floats[2], floats[3]);
            Self { r, g, b, a }
        }

        /// Creates a new Color from its R, G and B channels.
        pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
            Self {
                r: r as f64 / 255.0,
                g: g as f64 / 255.0,
                b: b as f64 / 255.0,
                a: 1.0,
            }
        }

        /// Creates a new Color from its R, G, B and A channels.
        pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
            Self {
                r: r as f64 / 255.0,
                g: g as f64 / 255.0,
                b: b as f64 / 255.0,
                a: a as f64 / 255.0,
            }
        }

        /// The RGB information of this color as 0.0-1.0 range floats representing
        /// proportions of 255 for each of R, G, B
        pub fn rgb(&self) -> (f64, f64, f64) {
            (self.r, self.g, self.b)
        }

        /// The RGBA information of this color as 0.0-1.0 range floats representing
        /// proportions of 255 for each of R, G, B, A
        pub fn rgba(&self) -> (f64, f64, f64, f64) {
            (self.r, self.g, self.b, self.a)
        }
    }

    impl From<u32> for Color {
        fn from(hex: u32) -> Self {
            Self::new_from_hex(hex)
        }
    }

    impl From<(f64, f64, f64)> for Color {
        fn from(rgb: (f64, f64, f64)) -> Self {
            let (r, g, b) = rgb;
            Self { r, g, b, a: 1.0 }
        }
    }

    impl From<(f64, f64, f64, f64)> for Color {
        fn from(rgba: (f64, f64, f64, f64)) -> Self {
            let (r, g, b, a) = rgba;
            Self { r, g, b, a }
        }
    }

    impl TryFrom<String> for Color {
        type Error = anyhow::Error;

        fn try_from(s: String) -> Result<Color> {
            (&s[..]).try_into()
        }
    }

    impl TryFrom<&str> for Color {
        type Error = anyhow::Error;

        fn try_from(s: &str) -> Result<Color> {
            let hex = u32::from_str_radix(s.strip_prefix('#').unwrap_or_else(|| &s), 16)?;

            if s.len() == 7 {
                Ok(Self::new_from_hex((hex << 8) + 0xFF))
            } else if s.len() == 9 {
                Ok(Self::new_from_hex(hex))
            } else {
                Err(anyhow!(
                    "failed to parse {} into a Color, invalid length",
                    &s
                ))
            }
        }
    }

    /// An EWMH Window type
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum WindowType {
        /// A dock / status bar
        Dock,
        /// A menu
        Menu,
        /// A normal window
        Normal,
    }
    impl WindowType {
        pub(crate) fn as_ewmh_str(&self) -> &str {
            match self {
                WindowType::Dock => "_NET_WM_WINDOW_TYPE_DOCK",
                WindowType::Menu => "_NET_WM_WINDOW_TYPE_MENU",
                WindowType::Normal => "_NET_WM_WINDOW_TYPE_NORMAL",
            }
        }
    }

    /// A simple drawing abstraction
    pub trait Draw {
        /// The type of drawing context used for drawing
        type Ctx: DrawContext;

        /// Create a new client window with a canvas for drawing
        fn new_window(
            &mut self,
            t: &WindowType,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
        ) -> Result<WinId>;
        /// Get the size of the target screen in pixels
        fn screen_sizes(&self) -> Result<Vec<Region>>;
        /// Register a font by name for later use
        fn register_font(&mut self, font_name: &str);
        /// Get a new DrawContext for the target window
        fn context_for(&self, id: WinId) -> Result<Self::Ctx>;
        /// Flush pending actions
        fn flush(&self, id: WinId);
        /// Map the target window to the screen
        fn map_window(&self, id: WinId);
        /// Unmap the target window from the screen
        fn unmap_window(&self, id: WinId);
        /// Destroy the target window
        fn destroy_window(&self, id: WinId);
    }

    /// Used for simple drawing to the screen
    pub trait DrawContext {
        /// Set the active font, must have been registered on the partent Draw
        fn font(&mut self, font_name: &str, point_size: i32) -> Result<()>;
        /// Set the color used for subsequent drawing operations
        fn color(&mut self, color: &Color);
        /// Clears the context
        fn clear(&mut self);
        /// Translate this context by (dx, dy) from its current position
        fn translate(&self, dx: f64, dy: f64);
        /// Set the x offset for this context absolutely
        fn set_x_offset(&self, x: f64);
        /// Set the y offset for this context absolutely
        fn set_y_offset(&self, y: f64);
        /// Draw a filled rectangle using the current color
        fn rectangle(&self, x: f64, y: f64, w: f64, h: f64);
        /// Render 's' using the current font with the supplied padding. returns the extent taken
        /// up by the rendered text
        fn text(&self, s: &str, h_offset: f64, padding: (f64, f64)) -> Result<(f64, f64)>;
        /// Determine the pixel width of a given piece of text using the current font
        fn text_extent(&self, s: &str) -> Result<(f64, f64)>;
        /// Flush pending actions
        fn flush(&self);
    }

    /// An XCB based Draw
    pub struct XCBDraw {
        conn: xcb::Connection,
        fonts: HashMap<String, FontDescription>,
        surfaces: HashMap<WinId, cairo::XCBSurface>,
    }
    impl XCBDraw {
        /// Create a new empty XCBDraw. Fails if unable to connect to the X server
        pub fn new() -> Result<Self> {
            let (conn, _) = xcb::Connection::connect(None)?;

            Ok(Self {
                conn,
                fonts: HashMap::new(),
                surfaces: HashMap::new(),
            })
        }

        fn screen(&self, ix: usize) -> Result<xcb::Screen> {
            self.conn
                .get_setup()
                .roots()
                .nth(ix)
                .ok_or_else(|| anyhow!("Screen index out of bounds"))
        }
    }
    impl Draw for XCBDraw {
        type Ctx = XCBDrawContext;

        fn new_window(
            &mut self,
            win_type: &WindowType,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
        ) -> Result<WinId> {
            let screen = self.screen(0)?;
            let (id, surface) = new_cairo_surface(
                &self.conn, &screen, win_type, x as i16, y as i16, w as i32, h as i32,
            )?;
            self.surfaces.insert(id, surface);

            Ok(id)
        }

        fn screen_sizes(&self) -> Result<Vec<Region>> {
            xcb_util::screen_sizes(&self.conn)
        }

        fn register_font(&mut self, font_name: &str) {
            self.fonts
                .insert(font_name.into(), FontDescription::from_string(font_name));
        }

        fn context_for(&self, id: WinId) -> Result<Self::Ctx> {
            let ctx = Context::new(
                self.surfaces
                    .get(&id)
                    .ok_or_else(|| anyhow!("uninitilaised window surface: {}", id))?,
            );

            Ok(XCBDrawContext {
                ctx,
                font: None,
                fonts: self.fonts.clone(),
            })
        }

        fn flush(&self, id: WinId) {
            if let Some(s) = self.surfaces.get(&id) {
                s.flush()
            };
            self.map_window(id);
            self.conn.flush();
        }

        fn map_window(&self, id: WinId) {
            xcb::map_window(&self.conn, id);
        }

        fn unmap_window(&self, id: WinId) {
            xcb::unmap_window(&self.conn, id);
        }

        fn destroy_window(&self, id: WinId) {
            xcb::destroy_window(&self.conn, id);
        }
    }

    /// An XCB based drawing context using pango and cairo
    pub struct XCBDrawContext {
        ctx: Context,
        font: Option<FontDescription>,
        fonts: HashMap<String, FontDescription>,
    }
    impl DrawContext for XCBDrawContext {
        fn font(&mut self, font_name: &str, point_size: i32) -> Result<()> {
            let mut font = self
                .fonts
                .get_mut(font_name)
                .ok_or_else(|| anyhow!("unknown font: {}", font_name))?
                .clone();
            font.set_size(point_size * pango::SCALE);
            self.font = Some(font);

            Ok(())
        }

        fn color(&mut self, color: &Color) {
            let (r, g, b, a) = color.rgba();
            self.ctx.set_source_rgba(r, g, b, a);
        }

        fn clear(&mut self) {
            self.ctx.save();
            self.ctx.set_operator(cairo::Operator::Clear);
            self.ctx.paint();
            self.ctx.restore();
        }

        fn translate(&self, dx: f64, dy: f64) {
            self.ctx.translate(dx, dy)
        }

        fn set_x_offset(&self, x: f64) {
            let (_, y_offset) = self.ctx.get_matrix().transform_point(0.0, 0.0);
            self.ctx.set_matrix(cairo::Matrix::identity());
            self.ctx.translate(x, y_offset);
        }

        fn set_y_offset(&self, y: f64) {
            let (x_offset, _) = self.ctx.get_matrix().transform_point(0.0, 0.0);
            self.ctx.set_matrix(cairo::Matrix::identity());
            self.ctx.translate(x_offset, y);
        }

        fn rectangle(&self, x: f64, y: f64, w: f64, h: f64) {
            self.ctx.rectangle(x, y, w, h);
            self.ctx.fill();
        }

        fn text(&self, txt: &str, h_offset: f64, padding: (f64, f64)) -> Result<(f64, f64)> {
            let layout = pango_layout(&self.ctx)?;
            if let Some(ref font) = self.font {
                layout.set_font_description(Some(font));
            }

            layout.set_text(txt);
            layout.set_ellipsize(EllipsizeMode::End);

            let (w, h) = layout.get_pixel_size();
            let (l, r) = padding;
            self.ctx.translate(l, h_offset);
            show_layout(&self.ctx, &layout);
            self.ctx.translate(-l, -h_offset);

            let width = w as f64 + l + r;
            let height = h as f64;

            Ok((width, height))
        }

        fn text_extent(&self, s: &str) -> Result<(f64, f64)> {
            let layout = pango_layout(&self.ctx)?;
            if let Some(ref font) = self.font {
                layout.set_font_description(Some(font));
            }
            layout.set_text(&s);
            let (w, h) = layout.get_pixel_size();

            Ok((w as f64, h as f64))
        }

        fn flush(&self) {
            self.ctx.get_target().flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_color_from_hex_rgba() {
        assert_eq!(Color::from(0x00000000), Color::from((0.0, 0.0, 0.0, 0.0)));
        assert_eq!(Color::from(0xFF00FFFF), Color::from((1.0, 0.0, 1.0, 1.0)));
        assert_eq!(Color::from(0xFFFFFFFF), Color::from((1.0, 1.0, 1.0, 1.0)));
        assert_eq!(Color::from(0xFFFF00FF), Color::from((1.0, 1.0, 0.0, 1.0)));
        assert_eq!(Color::from(0xFFFF0000), Color::from((1.0, 1.0, 0.0, 0.0)));
        assert_eq!(Color::from(0xFF000000), Color::from((1.0, 0.0, 0.0, 0.0)));
        assert_eq!(Color::from(0x000000FF), Color::from((0.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_color_from_str_rgb() {
        assert_eq!(
            Color::try_from("#000000").unwrap(),
            Color::from((0.0, 0.0, 0.0, 1.0))
        );
        assert_eq!(
            Color::try_from("#FF00FF").unwrap(),
            Color::from((1.0, 0.0, 1.0, 1.0))
        );
    }

    #[test]
    fn test_color_from_str_rgba() {
        assert_eq!(
            Color::try_from("#000000FF").unwrap(),
            Color::from((0.0, 0.0, 0.0, 1.0))
        );
        assert_eq!(
            Color::try_from("#FF00FF00").unwrap(),
            Color::from((1.0, 0.0, 1.0, 0.0))
        );
    }
}
