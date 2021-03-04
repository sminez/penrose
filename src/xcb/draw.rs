/*!
 * API layer implementing [Draw][crate::draw::Draw] and [DrawContext][crate::draw::DrawContext]
 * using XCB, pango and cairo.
 *
 * This is a reference implementation and requires that you have the relevant C dependencies
 * installed on your system for it to work.
 */
use crate::{
    core::{
        data_types::{Region, WinType},
        xconnection::{Prop, WindowState, XClientHandler, Xid},
    },
    draw::{Color, Draw, DrawContext, DrawError, Result},
    xcb::{Api, XcbError},
};

use pangocairo::functions::{create_layout, show_layout};

use std::collections::HashMap;

#[cfg(feature = "keysyms")]
use crate::core::xconnection::{KeyPressParseAttempt, XKeyboardHandler};

fn pango_layout(ctx: &cairo::Context) -> Result<pango::Layout> {
    Ok(create_layout(ctx).ok_or_else(|| XcbError::Pango("unable to create layout".into()))?)
}

/// An XCB based [Draw] implementation backed by pango and cairo
#[derive(Debug)]
pub struct XcbDraw {
    api: Api,
    fonts: HashMap<String, pango::FontDescription>,
    surfaces: HashMap<Xid, cairo::XCBSurface>,
}

crate::__xcb_impl_xclienthandler!(XcbDraw);
crate::__xcb_impl_xclientproperties!(XcbDraw);
#[cfg(feature = "keysyms")]
crate::__xcb_impl_xkeyboardhandler!(XcbDraw);

impl XcbDraw {
    /// Create a new empty [XcbDraw]. Fails if unable to connect to the X server
    pub fn new() -> Result<Self> {
        Ok(Self {
            api: Api::new()?,
            fonts: HashMap::new(),
            surfaces: HashMap::new(),
        })
    }

    /// Get a handle on the underlying [XCB Connection][::xcb::Connection] used by [Api]
    /// to communicate with the X server.
    pub fn xcb_connection(&self) -> &xcb::Connection {
        &self.api.conn()
    }

    /// Get an immutable handle on the underlying [Api] to communicate with the X server.
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// Get a mutable handle on the underlying [Api] to communicate with the X server.
    pub fn api_mut(&mut self) -> &mut Api {
        &mut self.api
    }
}

impl Draw for XcbDraw {
    type Ctx = XcbDrawContext;

    fn new_window(&mut self, ty: WinType, r: Region, managed: bool) -> Result<Xid> {
        let (_, _, w, h) = r.values();
        let id = self.api.create_window(ty, r, managed)?;
        let xcb_screen = self.api.screen(0)?;
        let depth = self.api.get_depth(&xcb_screen)?;
        let mut visualtype = self.api.get_visual_type(&depth)?;

        let surface = unsafe {
            let conn_ptr = self.api.conn().get_raw_conn() as *mut cairo_sys::xcb_connection_t;

            cairo::XCBSurface::create(
                &cairo::XCBConnection::from_raw_none(conn_ptr),
                &cairo::XCBDrawable(id),
                &cairo::XCBVisualType::from_raw_none(
                    &mut visualtype.base as *mut xcb::ffi::xcb_visualtype_t
                        as *mut cairo_sys::xcb_visualtype_t,
                ),
                w as i32,
                h as i32,
            )?
        };

        surface.set_size(w as i32, h as i32)?;
        self.surfaces.insert(id, surface);

        Ok(id)
    }

    fn screen_sizes(&self) -> Result<Vec<Region>> {
        Ok(self.api.screen_sizes()?)
    }

    fn register_font(&mut self, font_name: &str) {
        self.fonts.insert(
            font_name.into(),
            pango::FontDescription::from_string(font_name),
        );
    }

    fn context_for(&self, id: Xid) -> Result<Self::Ctx> {
        let ctx = cairo::Context::new(
            self.surfaces
                .get(&id)
                .ok_or(XcbError::UnintialisedSurface(id))?,
        );

        Ok(Self::Ctx {
            ctx,
            font: None,
            fonts: self.fonts.clone(),
        })
    }

    fn temp_context(&self, w: u32, h: u32) -> Result<Self::Ctx> {
        let xcb_screen = self.api.screen(0)?;
        let depth = self.api.get_depth(&xcb_screen)?;
        let mut visualtype = self.api.get_visual_type(&depth)?;

        let surface = unsafe {
            let conn_ptr = self.api.conn().get_raw_conn() as *mut cairo_sys::xcb_connection_t;

            cairo::XCBSurface::create(
                &cairo::XCBConnection::from_raw_none(conn_ptr),
                &cairo::XCBDrawable(self.api.root()),
                &cairo::XCBVisualType::from_raw_none(
                    &mut visualtype.base as *mut xcb::ffi::xcb_visualtype_t
                        as *mut cairo_sys::xcb_visualtype_t,
                ),
                w as i32,
                h as i32,
            )?
        };

        let surface = surface.create_similar(cairo::Content::Color, w as i32, h as i32)?;
        let ctx = cairo::Context::new(&surface);

        Ok(Self::Ctx {
            ctx,
            font: None,
            fonts: self.fonts.clone(),
        })
    }

    fn flush(&self, id: Xid) -> Result<()> {
        if let Some(s) = self.surfaces.get(&id) {
            s.flush()
        };
        self.map_client(id)?;
        self.api.flush();
        Ok(())
    }
}

/// An XCB based drawing context using pango and cairo
#[derive(Clone, Debug)]
pub struct XcbDrawContext {
    ctx: cairo::Context,
    font: Option<pango::FontDescription>,
    fonts: HashMap<String, pango::FontDescription>,
}

impl DrawContext for XcbDrawContext {
    fn font(&mut self, font_name: &str, point_size: i32) -> Result<()> {
        let mut font = self
            .fonts
            .get_mut(font_name)
            .ok_or_else(|| DrawError::UnknownFont(font_name.into()))?
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
        layout.set_ellipsize(pango::EllipsizeMode::End);

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
