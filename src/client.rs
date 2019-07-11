extern crate x11;

use output::Monitor;
use util::Region;
use x11::xlib;

pub struct ClientList<'a> {
    clients: Vec<&'a Client<'a>>,
    stack: Vec<&'a Client<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct Client<'a> {
    name: String,
    tags: u8,
    next: &'a Client<'a>,
    snext: &'a Client<'a>,
    x_window: &'a xlib::Window,

    position: Region,
    old_position: Region,

    min_alpha: f32,
    max_alpha: f32,

    base_width: i32,
    max_width: i32,
    min_width: i32,
    inc_width: i32,

    base_height: i32,
    max_height: i32,
    min_height: i32,
    inc_height: i32,

    border_width: i32,
    old_border_width: i32,

    is_fixed: bool,
    is_floating: bool,
    is_urgent: bool,
    never_focus: bool,
    old_state: bool,
    is_fullscreen: bool,
    is_pinned: bool,
}

impl<'a> Client<'a> {
    // static void applyrules(Client *c);
    // fn apply_rules(&mut self, display: &mut xlib::Display) {
    //     self.is_floating = false;
    //     self.tags = 0;

    //     // Call out to xlib to get the class hints
    //     let mut class_hint = xlib::XClassHint {
    //         res_class: &mut 0,
    //         res_name: &mut 0,
    //     };

    //     unsafe {
    //         xlib::XGetClassHint(display, *self.x_window, &mut class_hint);
    //     }

    //     let class = class_hint.res_class;
    //     let instance = class_hint.res_name;
    // }

    // static void attach(Client *c);
    fn attatch(&'a mut self, monitor: &'a mut Monitor<'a>) {
        self.next = monitor.client_list.clients[1];
        monitor.client_list.clients.insert(0, self);
    }

    // static void detach(Client *c);
    fn detatch(&'a self, monitor: &'a mut Monitor<'a>) {
        if let Some(ix) = monitor.client_list.clients.iter().position(|i| *i == self) {
            monitor.client_list.clients.remove(ix);
        }
    }
}

// static int applysizehints(Client *c, int *x, int *y, int *w, int *h, int interact);
// static void attachstack(Client *c);
// static void configure(Client *c);
// static void detachstack(Client *c);
// static void focus(Client *c);
// static Atom getatomprop(Client *c, Atom prop);
// static void grabbuttons(Client *c, int focused);
// static Client *nexttiled(Client *c, Monitor *m);
// static void pop(Client *);
// static void removesystrayicon(Client *i);
// static void resize(Client *c, int x, int y, int w, int h, int interact);
// static void sendmon(Client *c, Monitor *m);
// static void setclientstate(Client *c, long state);
// static void setfocus(Client *c);
// static void setfullscreen(Client *c, int fullscreen);
// static void seturgent(Client *c, int urg);
// static void showhide(Client *c);
// static void unfocus(Client *c, int setfocus);
// static void unmanage(Client *c, int destroyed);
// static void updatesizehints(Client *c);
// static void updatesystrayicongeom(Client *i, int w, int h);
// static void updatesystrayiconstate(Client *i, XPropertyEvent *ev);
// static void updatetitle(Client *c);
// static void updatewindowtype(Client *c);
// static void updatewmhints(Client *c);
// static void warp(const Client *c);
// static Client *wintoclient(Window w);
// static Client *wintosystrayicon(Window w);
