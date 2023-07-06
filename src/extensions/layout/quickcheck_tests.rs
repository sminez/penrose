//! Quickcheck based tests to ensure that Layouts behave well for arbitrary inputs
//!
//! These tests do not assert anything about the behaviour of any of the individual
//! layouts, only that they do not panic and crash the window manager when asked to
//! layout unexpected inputs.
//!
//! NOTE: See penrose::builtin::layout::quickcheck_tests for the Arbitrary impls for
//!       Stack<Xid> and Rect.
use crate::{
    core::layout::Layout,
    extensions::layout::{Fibonacci, Tatami},
    pure::{geometry::Rect, Stack},
    Xid,
};
use quickcheck_macros::quickcheck;

#[quickcheck]
fn fibonacci_doesnt_panic(r: Rect, stack: Stack<Xid>, ratio: u8) -> bool {
    let ratio = ((ratio % 10) as f32) / 10.0;
    let (_, positions) = Fibonacci::new(40, ratio, 0.1).layout(&stack, r);

    !positions.is_empty()
}

#[quickcheck]
fn tatami_doesnt_panic(r: Rect, stack: Stack<Xid>, ratio: u8) -> bool {
    let ratio = ((ratio % 10) as f32) / 10.0;
    let (_, positions) = Tatami::new(ratio, 0.1).layout(&stack, r);

    !positions.is_empty()
}
