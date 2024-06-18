//! A simple UI for view the results of a given Layout implementation
use crate::{Draw, Result};
use penrose::{
    builtin::layout::transformers::Gaps,
    core::layout::Layout,
    pure::{geometry::Rect, Stack},
    x::{Atom, WinType, XConn},
    x11rb::RustConn,
    Color, Xid,
};
use std::{thread::sleep, time::Duration};

const FONT: &str = "mono";

/// A simple way to view the output of specific [Layout] implementations outside of a running
/// window manager.
#[derive(Debug)]
pub struct LayoutViewer {
    drw: Draw,
    win: Xid,
    r: Rect,
    focused: Color,
    unfocused: Color,
    text: Color,
}

impl LayoutViewer {
    /// Construct a new [LayoutViewer] with a specified color scheme and window size.
    pub fn new(
        r: Rect,
        bg: impl Into<Color>,
        focused: impl Into<Color>,
        unfocused: impl Into<Color>,
        text: impl Into<Color>,
    ) -> Result<Self> {
        let conn = RustConn::new()?;
        let screen_rects = conn.screen_details()?;
        let r_screen = screen_rects.last().unwrap();

        let mut drw = Draw::new(FONT, 14, bg)?;
        let win = drw.new_window(
            WinType::InputOutput(Atom::NetWindowTypeDock),
            r.centered_in(r_screen).unwrap_or(r_screen.shrink_in(30)),
            false,
        )?;

        Ok(Self {
            drw,
            win,
            r,
            focused: focused.into(),
            unfocused: unfocused.into(),
            text: text.into(),
        })
    }

    /// Run a [Layout] for a given client stack and display the result for specified number of
    /// milliseconds.
    pub fn render_layout_with_stack(
        &mut self,
        layout: &mut Box<dyn Layout>,
        stack: &Stack<Xid>,
        display_ms: u64,
    ) -> Result<()> {
        let focus = *stack.focused();
        let (_, positions) = layout.layout(stack, self.r);

        let mut ctx = self.drw.context_for(self.win)?;
        ctx.fill_bg(self.r)?;

        for (id, r_w) in positions {
            let color = if id == focus {
                self.focused
            } else {
                self.unfocused
            };
            ctx.fill_rect(r_w, color)?;

            ctx.set_offset((r_w.x + r_w.w / 2) as i32, (r_w.y + r_w.h / 2) as i32);
            ctx.draw_text(&id.to_string(), 0, (0, 0), self.text)?;
            ctx.reset_offset();
        }

        self.drw.flush(self.win)?;
        sleep(Duration::from_millis(display_ms));

        Ok(())
    }

    /// Show the layout result for a set of [Layout]s using a given stack while rotating focus
    /// between the clients.
    pub fn showcase_layouts(
        &mut self,
        mut s: Stack<Xid>,
        layouts: &[Box<dyn Layout>],
        gap_px: u32,
        display_ms: u64,
    ) -> Result<()> {
        for l in layouts.iter() {
            let mut l = Gaps::wrap(l.boxed_clone(), gap_px, gap_px);
            for _ in 0..s.len() {
                self.render_layout_with_stack(&mut l, &s, display_ms)?;
                s.focus_down();
            }
        }

        Ok(())
    }
}
