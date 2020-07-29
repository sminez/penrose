//! A scratchpad that holds a single client
use crate::core::client::Client;
use crate::core::data_types::{Config, Region, WinId};
use crate::core::helpers::spawn;
use crate::core::hooks::Hook;
use crate::core::manager::WindowManager;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Scratchpad {
    client: Rc<RefCell<Option<WinId>>>,
    pending: Rc<RefCell<bool>>,
    visible: Rc<RefCell<bool>>,
    prog: &'static str,
    w: f32,
    h: f32,
}

impl Scratchpad {
    pub fn new(prog: &'static str, w: f32, h: f32) -> Box<Scratchpad> {
        if w < 0.0 || w > 1.0 || h < 0.0 || h > 1.0 {
            panic!("Scratchpad: w & h must be between 0.0 and 1.0");
        }

        Box::new(Scratchpad {
            client: Rc::new(RefCell::new(None)),
            pending: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(false)),
            prog,
            w,
            h,
        })
    }

    pub fn register(&mut self, conf: &mut Config) {
        conf.hooks.push(Box::new(Scratchpad {
            client: Rc::clone(&self.client),
            pending: Rc::clone(&self.pending),
            visible: Rc::clone(&self.visible),
            prog: self.prog,
            w: self.w,
            h: self.h,
        }))
    }

    pub fn toggle(&mut self, wm: &mut WindowManager) {
        let id = match *self.client.borrow() {
            Some(id) => id,
            None => {
                self.pending.replace(true);
                self.visible.replace(false);
                spawn(self.prog); // caught by new_client
                return;
            }
        };

        if *self.visible.borrow() {
            wm.hide_client(id);
            self.visible.replace(false);
        } else {
            wm.show_client(id);
            wm.layout_screen(wm.active_screen_index()); // caught by layout_change
            self.visible.replace(true);
        }
    }
}

impl Hook for Scratchpad {
    fn new_client(&mut self, wm: &mut WindowManager, c: Client) -> Option<Client> {
        if *self.pending.borrow() && self.client.borrow().is_none() {
            self.pending.replace(false);
            self.client.replace(Some(c.id()));
            self.toggle(wm);
            None
        } else {
            Some(c)
        }
    }

    fn remove_client(&mut self, _: &mut WindowManager, id: WinId) {
        let client = match *self.client.borrow() {
            Some(id) => id,
            None => return,
        };

        if id == client {
            self.client.replace(None);
            self.visible.replace(false);
        }
    }

    fn layout_change(&mut self, wm: &mut WindowManager, _: usize, screen_index: usize) {
        match *self.client.borrow() {
            None => return, // no active scratchpad client
            Some(id) => {
                let region = wm.screen_size(screen_index);
                let (sx, sy, sw, sh) = region.values();
                let w = (sw as f32 * self.w) as u32;
                let h = (sh as f32 * self.h) as u32;
                let x = sx + (sw - w) / 2;
                let y = sy + (sh - h) / 2;
                wm.position_client(id, Region::new(x, y, w, h));
            }
        }
    }
}
