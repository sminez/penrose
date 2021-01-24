//! API layer implementing [Draw][crate::draw::Draw] and [DrawContext][crate::draw::DrawContext]
//! using just x11rb.

use crate::{
    core::{
        data_types::{PropVal, Region, WinId, WinType},
        xconnection::Atom,
    },
    draw::{Color, Draw, DrawContext, DrawError, Result},
    x11rb::{common::{current_screens, replace_prop, Atoms}, Result as X11Result, X11rbError},
};

use x11rb::{
    connection::Connection,
    protocol::xproto::{
        AtomEnum, Char2b, ChangeGCAux, CreateGCAux, CreateWindowAux, ConnectionExt as _, EventMask,
        Font, Gcontext, PropMode, Rectangle, WindowClass,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt,
};

use std::{
    cell::Cell,
    collections::HashMap,
    rc::Rc,
};

/// An x11rb based [Draw] implementation that directly does X11 drawing.
///
/// Expect bad text output!
#[derive(Debug)]
pub struct X11rbDraw<C> {
    conn: Rc<C>,
    atoms: Atoms,
    fonts: HashMap<String, Font>,
}

impl X11rbDraw<RustConnection> {
    /// Create a new empty [X11rbDraw]. Fails if unable to connect to the X server.
    pub fn new() -> Result<Self> {
        let (conn, _) = RustConnection::connect(None).map_err(|err| X11rbError::from(err))?;
        Ok(Self::new_for_connection(Rc::new(conn))?)
    }
}

impl<C: Connection> X11rbDraw<C> {
    /// Create a new empty [X11rbDraw] for the given X connection.
    pub fn new_for_connection(conn: Rc<C>) -> Result<Self> {
        let atoms = Atoms::new(&*conn)?;
        Ok(Self {
            conn,
            atoms,
            fonts: HashMap::new(),
        })
    }
}

impl<C: Connection> Draw for X11rbDraw<C> {
    type Ctx = X11rbDrawContext<C>;

    fn new_window(&mut self, ty: WinType, r: Region, managed: bool) -> Result<WinId> {
        let id = self.conn.generate_id().map_err(|err| X11rbError::from(err))?;
        let screen = &self.conn.setup().roots[0];
        let (x, y, w, h) = r.values();
        let mut aux = CreateWindowAux::new();
        let mut kind = None;
        let class;

        if !managed {
            aux = aux.override_redirect(1);
        }

        match ty {
            WinType::CheckWin => class = WindowClass::INPUT_OUTPUT,
            WinType::InputOnly => class = WindowClass::INPUT_ONLY,
            WinType::InputOutput(a) => {
                let window_type = self.conn.intern_atom(false, Atom::NetWmWindowType.as_ref().as_bytes())
                    .map_err(|err| X11rbError::from(err))?;
                let atom = self.conn.intern_atom(false, a.as_ref().as_bytes())
                    .map_err(|err| X11rbError::from(err))?;
                let window_type = window_type.reply()
                    .map_err(|err| X11rbError::from(err))?
                    .atom;
                let atom = atom.reply()
                    .map_err(|err| X11rbError::from(err))?
                    .atom;
                class = WindowClass::INPUT_OUTPUT;
                kind = Some((window_type, atom));
                aux = aux.border_pixel(screen.black_pixel)
                    .event_mask(EventMask::EXPOSURE | EventMask::KEY_PRESS);
            }
        }

        self.conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT, // depth
            id,
            screen.root,
            x as _,
            y as _,
            w as _,
            h as _,
            0, // border width
            class,
            x11rb::COPY_FROM_PARENT,
            &aux,
        ).map_err(|err| X11rbError::from(err))?;

        if let Some((window_type, atom)) = kind {
            self.conn.change_property32(PropMode::REPLACE, id, window_type, AtomEnum::ATOM, &[atom])
                .map_err(|err| X11rbError::from(err))?;
            self.conn.map_window(id)
                .map_err(|err| X11rbError::from(err))?;
        }

        Ok(id)
    }

    fn screen_sizes(&self) -> Result<Vec<Region>> {
        let root = self.conn.setup().roots[0].root;
        Ok(current_screens(&*self.conn, root)?
            .into_iter()
            .map(|s| s.region(false))
            .collect())
    }

    fn register_font(&mut self, font_name: &str) {
        let font = self.conn.generate_id().unwrap();
        self.conn.open_font(font, font_name.as_bytes()).unwrap();
        self.fonts.insert(font_name.to_string(), font);
    }

    fn context_for(&self, id: WinId) -> Result<Self::Ctx> {
        Ok(X11rbDrawContext::new(Rc::clone(&self.conn), id, self.fonts.clone())?)
    }

    fn temp_context(&self, w: u32, h: u32) -> Result<Self::Ctx> {
        let _ = (w, h);
        todo!()
    }

    fn flush(&self, _id: WinId) {
        self.conn.flush().unwrap();
    }

    fn map_window(&self, id: WinId) {
        self.conn.map_window(id).unwrap();
    }

    fn unmap_window(&self, id: WinId) {
        self.conn.unmap_window(id).unwrap();
    }

    fn destroy_window(&mut self, id: WinId) {
        self.conn.destroy_window(id).unwrap();
    }

    fn replace_prop(&self, id: WinId, prop: Atom, val: PropVal<'_>) {
        let atom = self.atoms.known_atom(prop);
        replace_prop(&*self.conn, id, atom, val).unwrap();
    }
}

/// An x11rb based drawing context that directly does X11 drawing.
///
/// Expect bad text output!
#[derive(Debug)]
pub struct X11rbDrawContext<C> {
    conn: Rc<C>,
    gc: Gcontext,
    target: WinId,
    // FIXME: This Cell is a work-around that should be fixed in the DrawContext trait instead (&self vs &mut self)
    offset: Cell<(f64, f64)>,
    font: Option<Font>,
    fonts: HashMap<String, Font>,
}

impl<C: Connection> X11rbDrawContext<C> {
    fn new(conn: Rc<C>, target: WinId, fonts: HashMap<String, Font>) -> X11Result<Self> {
        let gc = conn.generate_id()?;
        conn.create_gc(gc, target, &CreateGCAux::new())?;
        Ok(Self {
            conn,
            gc,
            target,
            offset: Cell::new((0., 0.)),
            font: None,
            fonts,
        })
    }

    fn coords(&self, x: f64, y: f64) -> (i16, i16) {
        let (dx, dy) = self.offset.get();
        ((dx + x).ceil() as _, (dy + y).ceil() as _)
    }
}

impl<C: Connection> DrawContext for X11rbDrawContext<C> {
    fn font(&mut self, font_name: &str, point_size: i32) -> Result<()> {
        let font = *self.fonts.get(font_name).ok_or_else(|| DrawError::UnknownFont(font_name.to_string()))?;
        warn!("X11 core fonts do not have a separate point size argument, ignoring {}", point_size);
        self.conn.change_gc(self.gc, &ChangeGCAux::new().font(font)).unwrap();
        self.font = Some(font);
        Ok(())
    }

    fn color(&mut self, color: &Color) {
        // FIXME: use RENDER instead of core rendering
        // FIXME: The following is incorrect, but good enough for now
        // FIXME: This kind of code should be on Color and not here (at least partly)
        let (r, g, b) = color.rgb();
        let (r, g, b) = (r * 255., g * 255., b * 255.);
        let (r, g, b) = (r as u8 as u32, g as u8 as u32, b as u8 as u32);
        let pixel = (r << 16) | (g << 8) | b;
        self.conn.change_gc(self.gc, &ChangeGCAux::new().foreground(pixel)).unwrap();
    }

    fn clear(&mut self) {
        // FIXME: This only works for windows and not for drawables
        self.conn.clear_area(false, self.target, 0, 0, 0, 0).unwrap();
    }

    fn translate(&self, dx: f64, dy: f64) {
        let (x, y) = self.offset.get();
        self.offset.set((x + dx, y + dy));
    }

    fn set_x_offset(&self, x: f64) {
        let (_, y) = self.offset.get();
        self.offset.set((x, y));
    }

    fn set_y_offset(&self, y: f64) {
        let (x, _) = self.offset.get();
        self.offset.set((x, y));
    }

    fn rectangle(&self, x: f64, y: f64, w: f64, h: f64) {
        // FIXME: Get rid of the rounding and use RENDER instead
        let (x, y) = self.coords(x, y);
        let (width, height) = (w.floor() as u16, h.floor() as u16);
        let rect = Rectangle { x, y, width, height };
        self.conn.poly_rectangle(self.target, self.gc, &[rect]).unwrap();
    }

    fn text(&self, s: &str, h_offset: f64, padding: (f64, f64)) -> Result<(f64, f64)> {
        let (w, h) = self.text_extent(s)?;
        let (x, y) = self.coords(padding.0, h_offset);
        // FIXME: Using utf8 is not correct here, but at least ASCII should still work
        // FIXME: This also draws a background with the current background color
        self.conn.image_text8(self.target, self.gc, x, y, s.as_bytes())
            .map_err(|err| X11rbError::from(err))?;
        Ok((w + padding.0 + padding.1, h))
    }

    fn text_extent(&self, s: &str) -> Result<(f64, f64)> {
        // FIXME: Using utf8 is not correct here, but at least ASCII should still work
        let text = s.bytes().map(|b| Char2b { byte1: b, byte2: 0 }).collect::<Vec<_>>();
        let extents = self.conn.query_text_extents(self.font.unwrap(), &text)
            .map_err(|err| X11rbError::from(err))?
            .reply()
            .map_err(|err| X11rbError::from(err))?;
        // FIXME No idea if this is correct
        let width = extents.overall_width;
        let height = extents.overall_ascent + extents.overall_descent;
        Ok((width as _, height as _))
    }

    fn flush(&self) {
        self.conn.flush().unwrap();
    }
}
