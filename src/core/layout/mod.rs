//! Layout for window positioning
use crate::{
    core::Xid,
    geometry::Rect,
    handle_message,
    state::{Stack, Workspace},
};

pub mod messages;

use messages::{common::*, Message};

// TODO: need versions of these with access to state?
pub trait Layout {
    fn name(&self) -> String;

    // TODO: might want / need this to take and return self rather than a mut ref
    //       so that it is possible for layouts to replace themselves with a new one?
    fn layout_workspace(
        self: Box<Self>,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        match &w.stack {
            Some(s) => self.layout(s, r),
            None => self.layout_empty(r),
        }
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>);

    fn layout_empty(self: Box<Self>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>);

    fn handle_message(&mut self, m: &Message);
}

/// A simple layout that places the main region on the left and tiles remaining
/// windows in a single column to the right.
pub struct SideStack {
    max_main: u32,
    ratio: f32,
    ratio_step: f32,
}

impl Default for SideStack {
    fn default() -> Self {
        Self {
            max_main: 1,
            ratio: 0.6,
            ratio_step: 0.1,
        }
    }
}

impl Layout for SideStack {
    fn name(&self) -> String {
        "SideStack".to_owned()
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        let n = s.len() as u32;

        let positions = if n <= self.max_main || self.max_main == 0 {
            // In both cases we have all windows in a single stack (all main or all secondary)
            r.as_rows(n).iter().zip(s).map(|(r, c)| (*c, *r)).collect()
        } else {
            // We have two stacks so split the secreen in two and then build a stack for each
            let split = ((r.w as f32) * self.ratio) as u32;
            let (main, stack) = r.split_at_width(split).unwrap();

            main.as_rows(self.max_main)
                .into_iter()
                .chain(stack.as_rows(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        };

        (self, positions)
    }

    fn layout_empty(self: Box<Self>, _r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        (self, vec![])
    }

    fn handle_message(&mut self, m: &Message) {
        handle_message! {
            message: m;

            ExpandMain => {
                self.ratio += self.ratio_step;
                if self.ratio > 1.0 {
                    self.ratio = 1.0;
                }
            },

            ShrinkMain => {
                self.ratio -= self.ratio_step;
                if self.ratio < 0.0 {
                    self.ratio = 0.0;
                }
            },

            IncMain(n) => {
                if n < 0 {
                    self.max_main = self.max_main.saturating_sub((-n) as u32);
                } else {
                    self.max_main += n as u32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        messages::{common::IncMain, AsMessage},
        *,
    };

    #[test]
    fn message_handling() {
        let mut l = SideStack {
            max_main: 1,
            ratio: 0.6,
            ratio_step: 0.1,
        };

        l.handle_message(&IncMain(2).as_message());

        assert_eq!(l.max_main, 3);
    }
}
