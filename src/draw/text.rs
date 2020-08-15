//! Text elements for rendering in windows
use anyhow::anyhow;
use cairo::{Context, Surface};
use pango::{EllipsizeMode, FontDescription, Layout};
use pangocairo::functions::{create_layout, show_layout};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Color {
    r: f64,
    g: f64,
    b: f64,
}
impl Color {
    pub fn new_from_hex(hex: u32) -> Self {
        Self {
            r: ((hex & 0xFF0000) >> 16) as f64 / 255.0,
            g: ((hex & 0x00FF00) >> 8) as f64 / 255.0,
            b: (hex & 0x0000FF) as f64 / 255.0,
        }
    }

    pub fn rgb(&self) -> (f64, f64, f64) {
        (self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// A set of styling options for a text string
pub struct TextStyle {
    /// Pango font name to use for rendering
    pub font: &'static str,
    /// Point size to render the font at
    pub point_size: i32,
    /// Foreground color in 0xRRGGBB format
    pub fg: u32,
    /// Optional background color in 0xRRGGBB format (default to current background if None)
    pub bg: Option<u32>,
    /// Pixel padding around this string
    pub padding: (f64, f64, f64, f64),
}

#[derive(Clone, Debug)]
/// A section of Text
pub struct Text {
    s: String,
    font: FontDescription,
    fg: Color,
    bg: Option<Color>,
    padding: (f64, f64, f64, f64),
    pub(crate) width: f64,
    pub(crate) height: f64,
}
impl Text {
    /// Create a new Text section using the supplied style
    pub fn new<S: Into<String>>(s: S, style: &TextStyle) -> Self {
        let mut font = FontDescription::from_string(style.font);
        font.set_size(style.point_size * pango::SCALE);
        Self {
            s: s.into(),
            font,
            fg: Color::new_from_hex(style.fg),
            bg: style.bg.map(|bg| Color::new_from_hex(bg)),
            padding: style.padding,
            width: 0.0,
            height: 0.0,
        }
    }

    pub(crate) fn update_extent(&mut self, surface: &Surface) -> anyhow::Result<()> {
        let ctx = Context::new(&surface);
        let layout = pango_layout(&ctx)?;
        layout.set_text(&self.s);
        layout.set_font_description(Some(&self.font));

        let (w, h) = layout.get_pixel_size();
        let (l, r, t, b) = self.padding;
        self.width = w as f64 + l + r;
        self.height = h as f64 + t + b;

        Ok(())
    }

    pub(crate) fn render(
        &self,
        surface: &Surface,
        bar_bg: &Color,
        offset: f64,
        height: f64,
    ) -> anyhow::Result<()> {
        let (l, r, t, b) = self.padding;
        let ctx = Context::new(&surface);
        let layout = pango_layout(&ctx)?;

        layout.set_text(&self.s);
        layout.set_font_description(Some(&self.font));
        ctx.translate(offset, 0.0);
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_width((self.width - l - r) as i32 * pango::SCALE);
        layout.set_height((self.height as f64 - t - b) as i32 * pango::SCALE);

        let (r, g, b) = match self.bg {
            Some(ref bg) => bg.rgb(),
            None => bar_bg.rgb(),
        };
        ctx.set_source_rgb(r, g, b);
        ctx.rectangle(0.0, 0.0, self.width, height as f64);
        ctx.fill();

        let (r, g, b) = self.fg.rgb();
        ctx.set_source_rgb(r, g, b);
        ctx.translate(l, t);
        show_layout(&ctx, &layout);

        Ok(())
    }
}

impl PartialEq<Text> for Text {
    fn eq(&self, other: &Text) -> bool {
        self.s == other.s && self.fg == other.fg && self.bg == other.bg && self.font == other.font
    }
}

fn pango_layout(ctx: &Context) -> anyhow::Result<Layout> {
    create_layout(ctx).ok_or_else(|| anyhow!("unable to create pango layout"))
}
