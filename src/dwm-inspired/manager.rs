use crate::client::Client;
use crate::config;
use crate::layout::LayoutKind;
use crate::monitor::Monitor;
use crate::util::Region;
use crate::x;
use std::ffi::CString;
use std::ptr;
use x11::xlib;

pub struct WindowManager {
    // X11
    display: *mut xlib::Display,
    default_screen: i32,
    root: xlib::Window,
    wm_protocols: xlib::Atom,
    wm_delete_window: xlib::Atom,

    // wm state
    running: bool,
    monitors: Vec<Monitor>,
    m_active: usize, // index into monitors
    clients: Vec<Client>,
    c_active: usize, // index into clients
}

impl WindowManager {
    pub fn new() -> WindowManager {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                panic!("unable to open display");
            }

            let screen = xlib::XDefaultScreen(display);
            let root = xlib::XRootWindow(display, screen);

            // Load atoms
            let proto_str = CString::new("WM_PROTOCOLS").unwrap();
            let del_win_str = CString::new("WM_DELETE_WINDOW").unwrap();
            let wm_delete_window = xlib::XInternAtom(display, del_win_str.as_ptr(), xlib::False);
            let wm_protocols = xlib::XInternAtom(display, proto_str.as_ptr(), xlib::False);

            if wm_delete_window == 0 || wm_protocols == 0 {
                panic!("unable to load atoms");
            }

            let mut protocols = [wm_delete_window];
            let sub = xlib::XSetWMProtocols(display, root, &mut protocols[0] as *mut xlib::Atom, 1);
            if sub == xlib::False {
                panic!("can't set WM protocols");
            }

            WindowManager {
                display: display,
                default_screen: screen,
                root: root,
                wm_protocols: wm_protocols,
                wm_delete_window: wm_delete_window,
                running: true,
                monitors: vec![],
                m_active: 0,
                clients: vec![],
                c_active: 0,
            }
        }
    }

    pub fn run(&mut self) {
        let mut reader = x::XEventReader::new();

        while self.running {
            let event = reader.next(self.display);

            match event.get_type() {
                xlib::ClientMessage => {
                    let xclient: xlib::XClientMessageEvent = From::from(event);

                    if xclient.message_type == self.wm_protocols && xclient.format == 32 {
                        let protocol = xclient.data.get_long(0) as xlib::Atom;

                        if protocol == self.wm_delete_window {
                            self.running = false;
                        }
                    }
                }
                // TODO: add the other event handlers
                _ => eprintln!("got unknown event: {:?}", event),
            }
        }
    }

    pub fn configure_window(&mut self, c: &mut Client, w: usize) {
        x::unsafe_configure_window(self.display, c, w);
    }

    fn apply_size_hints(&self, r: Region, c: &mut Client, interact: bool) -> bool {
        interact
    }

    pub fn resize(&mut self, r: Region, c: &mut Client, interact: bool) {
        if self.apply_size_hints(r, c, interact) {
            self.resize_client(r, c)
        }
    }

    fn resize_client(&mut self, r: Region, c: &mut Client) {
        let m = &self.monitors[self.m_active];
        let n_tiled = self.clients_for_monitor(&m);
        let kind = m.layout_kind();

        let (offset, incr, border) = if c.is_floating || kind == LayoutKind::Floating {
            (0, 0, c.border_width)
        } else {
            if n_tiled == 1 || kind == LayoutKind::Gapless {
                (0, -2 * config::BORDER_PX as isize, 0)
            } else {
                (config::GAP_PX, 2 * config::GAP_PX as isize, c.border_width)
            }
        };

        c.old_region = c.region;
        c.region = Region::new(
            r.x + offset,
            r.y + offset,
            (r.w as isize - incr) as usize,
            (r.h as isize - incr) as usize,
        );

        self.configure_window(c, border);
    }

    fn clients_for_monitor(&self, m: &Monitor) -> usize {
        self.clients
            .iter()
            .filter(|c| c.is_tiled_on_monitor(m))
            .count()
    }

    fn layout_monitor(&mut self, index: usize) {
        self.clients = self.monitors[index]
            .get_layout_actions(self.clients.clone())
            .iter_mut()
            .map(|ra| match ra.r {
                Some(region) => {
                    self.resize(region, &mut ra.c, false);
                    ra.c
                }
                None => ra.c,
            })
            .collect();
    }
}
