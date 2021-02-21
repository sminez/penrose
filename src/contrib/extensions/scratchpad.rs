//! A scratchpad that holds a single client
use crate::{
    core::{
        bindings::KeyEventHandler,
        client::Client,
        data_types::Region,
        helpers::spawn,
        hooks::Hook,
        manager::WindowManager,
        ring::Selector,
        xconnection::{XConn, Xid},
    },
    Result,
};

use std::{cell::RefCell, fmt, rc::Rc};

/// Spawn and manage a single [Client] which can then be shown above the current layout.
///
/// The [get_hook][Scratchpad::get_hook] method must be called to pass the associated [Hook] to your
/// [WindowManager] before calling init in order to register the necessary hooks to spawn, capture
/// and manage the embedded client. The client is spawned when 'toggle' is called and there is no
/// existing client, after that 'toggle' will show/hide the client on the active screen. If the
/// client is removed, calling 'toggle' again will spawn a new client in the same way.
#[derive(Clone, PartialEq)]
pub struct Scratchpad {
    client: Rc<RefCell<Option<Xid>>>,
    pending: Rc<RefCell<bool>>,
    visible: Rc<RefCell<bool>>,
    prog: String,
    w: f32,
    h: f32,
}

impl fmt::Debug for Scratchpad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scratchpad")
            .field("client_id", &self.client.borrow())
            .field("pending", &self.pending.borrow())
            .field("visible", &self.visible.borrow())
            .field("prog", &self.prog)
            .field("w", &self.w)
            .field("h", &self.h)
            .finish()
    }
}

impl Scratchpad {
    /// Create a new Scratchpad for holding 'prog'. 'w' and 'h' are the percentage width and height
    /// of the active screen that you want the client to take up when visible.
    /// NOTE: this function will panic if 'w' or 'h' are not within the range 0.0 - 1.0
    pub fn new<S>(prog: S, w: f32, h: f32) -> Scratchpad
    where
        S: Into<String>,
    {
        if !(0.0..=1.0).contains(&w) || !(0.0..=1.0).contains(&h) {
            panic!("Scratchpad: w & h must be between 0.0 and 1.0");
        }

        Scratchpad {
            client: Rc::new(RefCell::new(None)),
            pending: Rc::new(RefCell::new(false)),
            visible: Rc::new(RefCell::new(false)),
            prog: prog.into(),
            w,
            h,
        }
    }

    fn boxed_clone(&self) -> Box<Self> {
        Box::new(Self {
            client: Rc::clone(&self.client),
            pending: Rc::clone(&self.pending),
            visible: Rc::clone(&self.visible),
            prog: self.prog.clone(),
            w: self.w,
            h: self.h,
        })
    }

    /// Construct the associated [Hook] for adding to the [WindowManager].
    ///
    /// NOTE: If the hook is not registered, [Scratchpad] will not be able to
    ///       capture and manage spawned [Client] windows.
    pub fn get_hook(&self) -> Box<Self> {
        self.boxed_clone()
    }

    /// Show / hide the bound client. If there is no client currently, then spawn one.
    pub fn toggle<X: XConn>(&self) -> KeyEventHandler<X> {
        let mut clone = self.boxed_clone();
        Box::new(move |wm: &mut WindowManager<X>| clone.toggle_client(wm))
    }

    fn toggle_client<X: XConn>(&mut self, wm: &mut WindowManager<X>) -> Result<()> {
        let id = match *self.client.borrow() {
            Some(id) => id,
            None => {
                self.pending.replace(true);
                self.visible.replace(false);
                return spawn(&self.prog); // caught by new_client
            }
        };

        if *self.visible.borrow() {
            self.visible.replace(false);
            wm.hide_client(id)?;
        } else {
            self.visible.replace(true);
            wm.layout_screen(wm.active_screen_index())?; // caught by layout_change
            self.grab_focus(wm);
        }

        Ok(())
    }

    fn region_for_screen(&self, region: Region) -> Region {
        let (sx, sy, sw, sh) = region.values();
        let w = (sw as f32 * self.w) as u32;
        let h = (sh as f32 * self.h) as u32;
        let x = sx + (sw - w) / 2;
        let y = sy + (sh - h) / 2;

        Region::new(x, y, w, h)
    }

    fn grab_focus<X: XConn>(&self, wm: &mut WindowManager<X>) {
        if let Some(id) = *self.client.borrow() {
            if let Err(e) = wm.focus_client(&Selector::WinId(id)) {
                error!("Scratchpad was unable to focus its client: {:?}", e);
            }
        }
    }
}

impl<X: XConn> Hook<X> for Scratchpad {
    fn new_client(&mut self, wm: &mut WindowManager<X>, c: &mut Client) -> Result<()> {
        if *self.pending.borrow() && self.client.borrow().is_none() {
            self.pending.replace(false);
            self.client.replace(Some(c.id()));
            c.externally_managed();
            c.set_floating(true);
            return self.toggle_client(wm);
        }

        Ok(())
    }

    fn remove_client(&mut self, _: &mut WindowManager<X>, id: Xid) -> Result<()> {
        let client = match *self.client.borrow() {
            Some(id) => id,
            None => return Ok(()),
        };

        if id == client {
            self.client.replace(None);
            self.visible.replace(false);
        }

        Ok(())
    }

    fn layout_applied(
        &mut self,
        wm: &mut WindowManager<X>,
        _: usize,
        screen_index: usize,
    ) -> Result<()> {
        if let Some(id) = *self.client.borrow() {
            if *self.visible.borrow() {
                if let Some(region) = wm.screen_size(screen_index) {
                    // stack above other clients
                    wm.position_client(id, self.region_for_screen(region), true)?;
                }
                wm.show_client(id)?;
            }
        }

        Ok(())
    }

    fn workspace_change(&mut self, wm: &mut WindowManager<X>, _: usize, _: usize) -> Result<()> {
        if *self.visible.borrow() {
            self.grab_focus(wm);
        }
        Ok(())
    }
}
