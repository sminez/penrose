//! A simple systray for the builtin status bar
use crate::{bar::widgets::Widget, Result};
use penrose::{
    core::State,
    x::{event::ClientMessage, XConn, XEvent},
    Xid,
};

// TODO:
// - Check if any more atoms are required for this
// - Config for systray position and monitor following
// - Add handler for resize requests? -> Yup!
// - Add an event hook for taking control of systray clients
// - Widget::on_event needs to return a bool (breaking change)

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Icon {
    id: Xid,
    w: u32,
    h: u32,
}

/// A simple embedded systray for use in the status bar provided by this crate
#[derive(Debug)]
pub struct Systray {
    window_id: Xid,
    h: u32,
    spacing: u32,
    require_draw: bool,
    prefered_screen: Option<usize>,
    icons: Vec<Icon>,
}

impl Systray {
    /// Check if a given window ID is managed as an icon by this systray
    fn window_is_systray_icon(&self, id: Xid) -> bool {
        self.icons.iter().any(|ico| ico.id == id)
    }

    fn remove_icon(&mut self, id: Xid) {
        self.icons.retain(|ico| ico.id != id);
        self.require_draw = true;
    }

    fn update_icon_geometry(&mut self, id: Xid, w: u32, h: u32) {
        if let Some(ico) = self.icons.iter_mut().find(|ico| ico.id == id) {
            ico.h = self.h;
            if w == h {
                ico.w = self.h;
            } else if h == self.h {
                ico.w = w;
            } else {
                ico.w = ((self.h as f32) * (w as f32) / (h as f32)) as u32;
            }

            // TODO: apply size hints
            // might need to force icon size/positioning if this fails?
        }
    }

    fn handle_client_message<X: XConn>(&self, msg: ClientMessage, _: &X) -> Result<()> {
        if !(msg.id == self.window_id && msg.dtype == "NetSystemTrayOP") {
            return Ok(());
        }

        // - Check msg data[1] to see if it's SYSTEM_TRAY_REQUEST_DOCK
        //   -> Need to determine the datatype being used here
        // - get window attributes
        // - add to self.icons
        // - select inputs
        // - reparent to the systray window
        // - set background colour
        // - send embed messages (looks like theres a few?)
        // - set client state to normal

        Ok(())
    }
}

// FIXME: does this want / need to be built into the bar itself rather than a widget?
// Widget has a lot of assumptions about being a text based widget rather than something
// like what we need for the systray
impl<X: XConn> Widget<X> for Systray {
    fn draw(
        &mut self,
        ctx: &mut crate::Context,
        screen: usize,
        screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> crate::Result<()> {
        todo!()
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn current_extent(&mut self, _: &mut crate::Context, h: f64) -> Result<(f64, f64)> {
        if self.icons.is_empty() {
            return Ok((0.0, 0.0));
        }

        let w: u32 = self.icons.iter().map(|ico| ico.w).sum();

        Ok(((w + self.spacing) as f64, h))
    }

    fn is_greedy(&self) -> bool {
        false
    }

    fn on_event(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<()> {
        if let XEvent::ResizeRequest(r) = event {
            if self.window_is_systray_icon(r.id) {
                self.update_icon_geometry(r.id, r.width, r.height);
                self.require_draw = true;
            }
        }

        if let XEvent::Destroy(id) = event {
            self.remove_icon(*id);
            self.require_draw = true;
        }

        todo!("property notify, configure notify, client message, unmap notify")
    }

    fn on_new_client(&mut self, id: Xid, state: &mut State<X>, x: &X) -> Result<()> {
        if self.window_is_systray_icon(id) {
            todo!("send embed message");
        }

        Ok(())
    }
}
