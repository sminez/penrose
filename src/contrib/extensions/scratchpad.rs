//! A scratchpad that holds a single client
use crate::core::{
    client::Client,
    data_types::{Config, FireAndForget, Region, WinId},
    helpers::spawn,
    hooks::Hook,
    manager::WindowManager,
};

use std::{cell::RefCell, rc::Rc};

/**
 * A Scratchpad spawns and manages a single Client which can then be shown above the current layout
 * using the 'toggle' method when bound to a key combination in your main.rs. The
 * Scratchpad.register method must be called before creating your WindowManager struct in order to
 * register the necessary hooks to spawn, capture and manage the embedded client. The client is
 * spawned when 'toggle' is called and there is no existing client, after that 'toggle' will
 * show/hide the client on the active screen. If the client is removed, calling 'toggle' again will
 * spawn a new client in the same way.
 */
pub struct Scratchpad {
    client: Rc<RefCell<Option<WinId>>>,
    pending: Rc<RefCell<bool>>,
    visible: Rc<RefCell<bool>>,
    prog: &'static str,
    w: f32,
    h: f32,
}

impl Scratchpad {
    /// Create a new Scratchpad for holding 'prog'. 'w' and 'h' are the percentage width and height
    /// of the active screen that you want the client to take up when visible.
    /// NOTE: this function will panic if 'w' or 'h' are not within the range 0.0 - 1.0
    pub fn new(prog: &'static str, w: f32, h: f32) -> Scratchpad {
        if w < 0.0 || w > 1.0 || h < 0.0 || h > 1.0 {
            panic!("Scratchpad: w & h must be between 0.0 and 1.0");
        }

        Scratchpad {
            client: Rc::new(RefCell::new(None)),
            pending: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(false)),
            prog,
            w,
            h,
        }
    }

    fn boxed_clone(&self) -> Box<Scratchpad> {
        Box::new(Scratchpad {
            client: Rc::clone(&self.client),
            pending: Rc::clone(&self.pending),
            visible: Rc::clone(&self.visible),
            prog: self.prog,
            w: self.w,
            h: self.h,
        })
    }

    /// Register the required hooks for managing this Scratchpad. Must be called before
    /// WindowManager.init.
    pub fn register(&self, conf: &mut Config) {
        conf.hooks.push(self.boxed_clone())
    }

    /// Show / hide the bound client. If there is no client currently, then spawn one.
    pub fn toggle(&self) -> FireAndForget {
        let mut clone = self.boxed_clone();
        Box::new(move |wm: &mut WindowManager| clone.toggle_client(wm))
    }

    fn toggle_client(&mut self, wm: &mut WindowManager) {
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
    fn new_client(&mut self, wm: &mut WindowManager, c: &mut Client) {
        if *self.pending.borrow() && self.client.borrow().is_none() {
            self.pending.replace(false);
            self.client.replace(Some(c.id()));
            c.externally_managed();
            self.toggle_client(wm);
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

    fn layout_applied(&mut self, wm: &mut WindowManager, _: usize, screen_index: usize) {
        match *self.client.borrow() {
            None => return, // no active scratchpad client
            Some(id) => {
                if let Some(region) = wm.screen_size(screen_index) {
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
}
