//! Quickcheck based tests to ensure that Layouts behave well for arbitrary inputs
//!
//! These tests do not assert anything about the behaviour of any of the individual
//! layouts, only that they do not panic and crash the window manager when asked to
//! layout unexpected inputs.
use crate::{
    builtin::layout::{
        transformers::{ReflectHorizontal, ReflectVertical},
        CenteredMain, Grid, MainAndStack, Monocle,
    },
    core::layout::Layout,
    pure::{geometry::Rect, Stack},
    stack, Xid,
};
use quickcheck::{Arbitrary, Gen};
use quickcheck_macros::quickcheck;
use std::collections::HashSet;

// Focus is always `42` and elements are unique.
impl Arbitrary for Stack<Xid> {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut up: Vec<Xid> = HashSet::<u32>::arbitrary(g)
            .into_iter()
            .filter(|&n| n != 42)
            .map(Into::into)
            .collect();

        let focus = Xid(42);
        if up.is_empty() {
            return stack!(focus); // return a minimal stack as we don't allow empty
        }

        let split_at = usize::arbitrary(g) % (up.len());
        let down = up.split_off(split_at);

        Self::new(up, focus, down)
    }
}

impl Arbitrary for Rect {
    fn arbitrary(g: &mut Gen) -> Self {
        // - ensuring that the dimensions of a screen being laid out aren't completely massive
        // - width and height are at least 100px
        // >> this is a bit of a hack but zero width/height screens aren't something layouts
        //    should have to consider as valid input
        Rect::new(
            u8::arbitrary(g) as u32,
            u8::arbitrary(g) as u32,
            (u8::arbitrary(g) as u32) + 100,
            (u8::arbitrary(g) as u32) + 100,
        )
    }
}

#[quickcheck]
fn monocle_doesnt_panic(r: Rect, stack: Stack<Xid>) -> bool {
    let (_, positions) = Monocle.layout(&stack, r);

    !positions.is_empty()
}

#[quickcheck]
fn grid_doesnt_panic(r: Rect, stack: Stack<Xid>) -> bool {
    let (_, positions) = Grid.layout(&stack, r);

    !positions.is_empty()
}

mod main_and_stack {
    use super::*;

    #[quickcheck]
    fn side_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = MainAndStack::side_unboxed(n, ratio, 0.1, false).layout(&stack, r);

        !positions.is_empty()
    }

    #[quickcheck]
    fn side_mirrored_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = MainAndStack::side_unboxed(n, ratio, 0.1, true).layout(&stack, r);

        !positions.is_empty()
    }

    #[quickcheck]
    fn bottom_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = MainAndStack::bottom_unboxed(n, ratio, 0.1, false).layout(&stack, r);

        !positions.is_empty()
    }

    #[quickcheck]
    fn bottom_mirrored_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = MainAndStack::bottom_unboxed(n, ratio, 0.1, true).layout(&stack, r);

        !positions.is_empty()
    }
}

mod centered_main {
    use super::*;

    #[quickcheck]
    fn vertical_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = CenteredMain::vertical_unboxed(n, ratio, 0.1).layout(&stack, r);

        !positions.is_empty()
    }

    #[quickcheck]
    fn horizontal_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) = CenteredMain::horizontal_unboxed(n, ratio, 0.1).layout(&stack, r);

        !positions.is_empty()
    }
}

mod transformers {
    use super::*;

    #[quickcheck]
    fn reflect_h_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) =
            ReflectHorizontal::wrap(MainAndStack::side(n, ratio, 0.1)).layout(&stack, r);

        !positions.is_empty()
    }

    #[quickcheck]
    fn reflect_v_doesnt_panic(r: Rect, stack: Stack<Xid>, n: u32, ratio: u8) -> bool {
        let ratio = ((ratio % 10) as f32) / 10.0;
        let (_, positions) =
            ReflectVertical::wrap(MainAndStack::side(n, ratio, 0.1)).layout(&stack, r);

        !positions.is_empty()
    }
}
