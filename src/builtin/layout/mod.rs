//! Built-in layouts.
use crate::{
    builtin::layout::messages::{ExpandMain, IncMain, Mirror, Rotate, ShrinkMain},
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};

pub mod messages;
pub mod transformers;

#[derive(Debug, Clone, Copy)]
enum StackPosition {
    Side,
    Bottom,
}

/// A simple [Layout] with main and secondary regions.
///
/// - `MainAndStack::side` give a main region to the left and remaining clients to the right.
/// - `MainAndStack::bottom` give a main region to the top and remaining clients to the bottom.
///
/// The ratio between the main and secondary stack regions can be adjusted by sending [ShrinkMain]
/// and [ExpandMain] messages to this layout. The number of clients in the main area can be
/// increased or decreased by sending an [IncMain] message. To flip between the side and bottom
/// behaviours you can send a [Rotate] message.
#[derive(Debug, Clone, Copy)]
pub struct MainAndStack {
    pos: StackPosition,
    max_main: u32,
    ratio: f32,
    ratio_step: f32,
    mirrored: bool,
}

impl MainAndStack {
    pub fn side(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::side_unboxed(max_main, ratio, ratio_step, false))
    }

    pub fn side_mirrored(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::side_unboxed(max_main, ratio, ratio_step, true))
    }

    pub fn side_unboxed(max_main: u32, ratio: f32, ratio_step: f32, mirrored: bool) -> Self {
        Self {
            pos: StackPosition::Side,
            max_main,
            ratio,
            ratio_step,
            mirrored,
        }
    }

    pub fn bottom(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::bottom_unboxed(max_main, ratio, ratio_step, false))
    }

    pub fn bottom_mirrored(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::bottom_unboxed(max_main, ratio, ratio_step, true))
    }

    pub fn bottom_unboxed(max_main: u32, ratio: f32, ratio_step: f32, mirrored: bool) -> Self {
        Self {
            pos: StackPosition::Bottom,
            max_main,
            ratio,
            ratio_step,
            mirrored,
        }
    }

    fn split(&self, d: u32) -> u32 {
        let ratio = if self.mirrored {
            1.0 - self.ratio
        } else {
            self.ratio
        };

        ((d as f32) * ratio) as u32
    }

    fn layout_side(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if n <= self.max_main || self.max_main == 0 {
            // In both cases we have all windows in a single stack (all main or all secondary)
            r.as_rows(n).iter().zip(s).map(|(r, c)| (*c, *r)).collect()
        } else {
            // We have two stacks so split the screen in two and then build a stack for each
            let split = self.split(r.w);
            let (mut main, mut stack) = r.split_at_width(split).expect("split point to be valid");
            if self.mirrored {
                (main, stack) = (stack, main);
            }

            main.as_rows(self.max_main)
                .into_iter()
                .chain(stack.as_rows(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }

    fn layout_bottom(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if n <= self.max_main || self.max_main == 0 {
            r.as_columns(n)
                .iter()
                .zip(s)
                .map(|(r, c)| (*c, *r))
                .collect()
        } else {
            let split = self.split(r.h);
            let (mut main, mut stack) = r.split_at_height(split).expect("split point to be valid");
            if self.mirrored {
                (main, stack) = (stack, main);
            }

            main.as_columns(self.max_main)
                .into_iter()
                .chain(stack.as_columns(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }
}

impl Default for MainAndStack {
    fn default() -> Self {
        Self {
            pos: StackPosition::Side,
            max_main: 1,
            ratio: 0.6,
            ratio_step: 0.1,
            mirrored: false,
        }
    }
}

impl Layout for MainAndStack {
    fn name(&self) -> String {
        match (self.pos, self.mirrored) {
            (StackPosition::Side, false) => "Side".to_owned(),
            (StackPosition::Side, true) => "Mirror".to_owned(),
            (StackPosition::Bottom, false) => "Bottom".to_owned(),
            (StackPosition::Bottom, true) => "Top".to_owned(),
        }
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(*self)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let positions = match self.pos {
            StackPosition::Side => self.layout_side(s, r),
            StackPosition::Bottom => self.layout_bottom(s, r),
        };

        (None, positions)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if let Some(&ExpandMain) = m.downcast_ref() {
            self.ratio += self.ratio_step;
            if self.ratio > 1.0 {
                self.ratio = 1.0;
            }
        } else if let Some(&ShrinkMain) = m.downcast_ref() {
            self.ratio -= self.ratio_step;
            if self.ratio < 0.0 {
                self.ratio = 0.0;
            }
        } else if let Some(&IncMain(n)) = m.downcast_ref() {
            if n < 0 {
                self.max_main = self.max_main.saturating_sub((-n) as u32);
            } else {
                self.max_main += n as u32;
            }
        } else if let Some(&Mirror) = m.downcast_ref() {
            self.mirrored = !self.mirrored;
        } else if let Some(&Rotate) = m.downcast_ref() {
            self.pos = match self.pos {
                StackPosition::Side => StackPosition::Bottom,
                StackPosition::Bottom => StackPosition::Side,
            };
        }

        None
    }
}

/// A simple monolce layout that gives the maximum available space to the currently
/// focused client and unmaps all other windows.
#[derive(Debug, Clone, Copy)]
pub struct Monocle;

impl Monocle {
    pub fn boxed() -> Box<dyn Layout> {
        Box::new(Monocle)
    }
}

impl Layout for Monocle {
    fn name(&self) -> String {
        "Mono".to_owned()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Self::boxed()
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, vec![(s.focus, r)])
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        builtin::layout::{messages::IncMain, *},
        core::layout::IntoMessage,
    };

    #[test]
    fn message_handling() {
        let mut l = MainAndStack::side_unboxed(1, 0.6, 0.1, false);

        l.handle_message(&IncMain(2).into_message());

        assert_eq!(l.max_main, 3);
    }
}
