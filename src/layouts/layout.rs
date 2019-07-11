extern crate x11;

use x11::xlib;

pub trait Layout {
    fn focused_client(&self) -> Option<&xlib::Window>;
    fn remove_focused(&mut self) -> Option<xlib::Window>;
    fn insert_client(&mut self, win: xlib::Window);

    fn focus_next(&mut self);
    fn focus_prev(&mut self);

    fn inc_master(&mut self, px: i32);
    fn dec_master(&mut self, px: i32);
}
