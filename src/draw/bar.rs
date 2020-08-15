//! A simple status bar
use anyhow::anyhow;
use cairo;
use xcb;

use crate::draw::text::{Color, Text};

const PROP_MODE_REPLACE: u8 = xcb::PROP_MODE_REPLACE as u8;

#[derive(Clone, Debug)]
/// A status bar position
pub enum Position {
    /// Top of the screen
    Top,
    /// Bottom of the screen
    Bottom,
}

/// A simple status bar that works via hooks
pub struct StatusBar {
    conn: xcb::Connection,
    id: u32,
    surface: cairo::XCBSurface,
    // position: Position,
    w: f64,
    h: f64,
    bg: Color,
    segments: Vec<Vec<Text>>,
}
impl StatusBar {
    /// Try to initialise a new empty status bar. Can fail if we are unable to connect to the X server
    pub fn try_new(_position: Position, h: i32, bg: u32) -> anyhow::Result<Self> {
        let (conn, ix) = match xcb::Connection::connect(None) {
            Err(e) => return Err(anyhow!("unable to establish connection to X server: {}", e)),
            Ok(conn) => conn,
        };
        let screen = conn
            .get_setup()
            .roots()
            .nth(ix as usize)
            .ok_or_else(|| anyhow!("Screen index out of bounds"))?;
        let w = screen.width_in_pixels() as f64;

        let (id, surface) = new_cairo_surface(&conn, &screen, h)?;
        Ok(Self {
            conn,
            id,
            surface,
            // position,
            w,
            h: h as f64,
            bg: Color::new_from_hex(bg),
            segments: vec![],
        })
    }

    /// Add a new segment to the status bar
    pub fn add_segment(&mut self, mut segment: Vec<Text>) -> anyhow::Result<()> {
        for t in segment.iter_mut() {
            t.update_extent(&self.surface)?
        }
        self.segments.push(segment);
        Ok(())
    }

    /// Update an existing segment in the status bar, triggers re-render
    pub fn update_segment(&mut self, idx: usize, mut content: Vec<Text>) -> anyhow::Result<()> {
        if self.segments[idx] == content {
            return Ok(());
        }

        for t in content.iter_mut() {
            t.update_extent(&self.surface)?
        }

        self.redraw_all();
        Ok(())
    }

    /// Re-render all segments in this status bar
    pub fn redraw_all(&self) {
        let ctx = cairo::Context::new(&self.surface);
        ctx.translate(0.0, 0.0);
        let (r, g, b) = self.bg.rgb();
        ctx.set_source_rgb(r, g, b);
        ctx.rectangle(0.0, 0.0, self.w, self.h);
        ctx.fill();

        self.segments.iter().flat_map(|s| s).fold(0.0, |w, t| {
            match t.render(&self.surface, &self.bg, w, self.h) {
                Ok(_) => t.width + w,
                Err(e) => {
                    println!("error rendering: {}", e);
                    w
                }
            }
        });

        self.conn.flush();
        xcb::map_window(&self.conn, self.id);
    }
}

fn new_cairo_surface(
    conn: &xcb::Connection,
    screen: &xcb::Screen,
    height: i32,
) -> anyhow::Result<(u32, cairo::XCBSurface)> {
    let (id, width) = create_window(conn, screen, height as u16)?;
    let mut visualtype = get_visual_type(&conn, screen)?;

    let surface = unsafe {
        let conn_ptr = conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t;

        cairo::XCBSurface::create(
            &cairo::XCBConnection::from_raw_none(conn_ptr),
            &cairo::XCBDrawable(id),
            &cairo::XCBVisualType::from_raw_none(
                &mut visualtype.base as *mut xcb::ffi::xcb_visualtype_t
                    as *mut cairo_sys::xcb_visualtype_t,
            ),
            width,
            height,
        )
        .map_err(|err| anyhow!("Error creating surface: {}", err))?
    };

    surface.set_size(width, height).unwrap();
    Ok((id, surface))
}

fn get_visual_type(
    conn: &xcb::Connection,
    screen: &xcb::Screen,
) -> anyhow::Result<xcb::Visualtype> {
    conn.get_setup()
        .roots()
        .flat_map(|r| r.allowed_depths())
        .flat_map(|d| d.visuals())
        .find(|v| v.visual_id() == screen.root_visual())
        .ok_or_else(|| anyhow!("unable to get screen visual type"))
}

fn intern_atom(conn: &xcb::Connection, name: &str) -> anyhow::Result<u32> {
    xcb::intern_atom(conn, false, name)
        .get_reply()
        .map(|r| r.atom())
        .map_err(|err| anyhow!("unable to intern xcb atom '{}': {}", name, err))
}

fn create_window(
    conn: &xcb::Connection,
    screen: &xcb::Screen,
    height: u16,
) -> anyhow::Result<(u32, i32)> {
    let id = conn.generate_id();
    let width = screen.width_in_pixels();

    xcb::create_window(
        &conn,
        xcb::COPY_FROM_PARENT as u8,
        id,
        screen.root(),
        0,
        0,
        width,
        height,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        0,
        &[
            (xcb::CW_BACK_PIXEL, screen.black_pixel()),
            (xcb::CW_EVENT_MASK, xcb::EVENT_MASK_EXPOSURE),
        ],
    );

    xcb::change_property(
        &conn,                                      // xcb connection to X11
        PROP_MODE_REPLACE,                          // discard current prop and replace
        id,                                         // window to change prop on
        intern_atom(&conn, "_NET_WM_WINDOW_TYPE")?, // prop to change
        intern_atom(&conn, "UTF8_STRING")?,         // type of prop
        8,                                          // data format (8/16/32-bit)
        "_NET_WM_WINDOW_TYPE_DOCK".as_bytes(),      // data
    );

    xcb::map_window(&conn, id);
    conn.flush();
    Ok((id, width as i32))
}
