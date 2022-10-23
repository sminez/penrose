use crate::{
    core::Xid,
    geometry::Rect,
    layout::{
        messages::{common::UnwrapTransformer, Message},
        Layout,
    },
    pure::Stack,
};
use std::mem::swap;

/// A wrapper round another [Layout] that is able to intercept and modify both the positions being
/// returned by the inner layout and messages being sent to it.
pub trait LayoutTransformer: Clone + Sized + 'static {
    /// The same as [Layout::name] but for [LayoutTransformer] itself.
    fn transformed_name(&self) -> String;

    /// Provide a mutable reference to the [Layout] wrapped by this transformer.
    fn inner_mut(&mut self) -> &mut Box<dyn Layout>;

    /// Replace the currently wrapped [Layout] with a new one.
    fn swap_inner(&mut self, mut new: Box<dyn Layout>) -> Box<dyn Layout> {
        swap(self.inner_mut(), &mut new);

        new
    }

    /// Remove the inner [Layout] from this [LayoutTransformer].
    fn unwrap(self) -> Box<dyn Layout>;

    /// Modify the initial [Rect] that will be passed to the inner [Layout].
    ///
    /// The default implementation of this method leaves the initial Rect unchanged.
    fn transform_initial(&self, r: Rect) -> Rect {
        r
    }

    /// Optionally modify any of the positions returned by the inner [Layout] before they are
    /// applied by the window manager. The dimensions of the screen being layed out are avaiable
    /// as `r`.
    ///
    /// The default implementation of this method leaves the positions returned by the inner layout
    /// unchanged.
    fn transform_positions(&mut self, _r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
        positions
    }

    /// Apply the [LayoutTransformer] to its wrapped inner [Layout].
    #[allow(clippy::type_complexity)]
    fn run_transform<F>(&mut self, f: F, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(Rect, &mut Box<dyn Layout>) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>),
    {
        let r = self.transform_initial(r);
        let (new, positions) = (f)(r, self.inner_mut());
        let transformed = self.transform_positions(r, positions);

        if let Some(l) = new {
            self.swap_inner(l);
        }

        (None, transformed)
    }

    /// Pass a message on to the wrapped inner [Layout].
    ///
    /// The default implementation of this method will return `Some(inner_layout)` if it
    /// receives an [UnwrapTransformer] [Message] using the `unwrap` method of this trait.
    fn passthrough_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if let Some(new) = self.inner_mut().handle_message(m) {
            self.swap_inner(new);
        }

        None
    }
}

impl<LT> Layout for LT
where
    LT: LayoutTransformer,
{
    fn name(&self) -> String {
        self.transformed_name()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(self.clone())
    }

    fn layout_workspace(
        &mut self,
        tag: &str,
        stack: &Option<Stack<Xid>>,
        r: Rect,
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|r, inner| inner.layout_workspace(tag, stack, r), r)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|r, inner| inner.layout(s, r), r)
    }

    fn layout_empty(&mut self, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|r, inner| inner.layout_empty(r), r)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        // TODO: find a nicer way to do this
        if let Some(&UnwrapTransformer) = m.downcast_ref() {
            return Some(self.swap_inner(Box::new(NullLayout)));
        }

        self.passthrough_message(m)
    }
}

/// A valid impl of Layout that can be used as a placeholder but will panic if used.
struct NullLayout;
impl Layout for NullLayout {
    fn name(&self) -> String {
        panic!("Null layout")
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        panic!("Null layout")
    }

    fn layout_workspace(
        &mut self,
        _: &str,
        _: &Option<Stack<Xid>>,
        _: Rect,
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        panic!("Null layout")
    }

    fn layout(&mut self, _: &Stack<Xid>, _: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        panic!("Null layout")
    }

    fn layout_empty(&mut self, _: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        panic!("Null layout")
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        panic!("Null layout")
    }
}

/// Quickly define a [LayoutTransformer] from a single element tuple struct and a
/// transformation function: `fn(Rect, Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)>`.
///
/// The struct must have a single field which is a `Box<dyn Layout>`.
///
/// # Example
/// ```no_run
/// # use penrose::{layout::Layout, Xid, geometry::Rect, simple_transformer};
/// #[derive(Clone)]
/// pub struct MyTransformer(Box<dyn Layout>);
///
/// fn my_transformation_function(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
///     todo!("transformation implementation goes here")
/// }
///
/// simple_transformer!("MyTransform", MyTransformer, my_transformation_function);
/// ```
#[macro_export]
macro_rules! simple_transformer {
    ($prefix:expr, $t:ident, $f:ident) => {
        impl $t {
            pub fn wrap(
                layout: Box<dyn $crate::layout::Layout>,
            ) -> Box<dyn $crate::layout::Layout> {
                Box::new(Self(layout))
            }
        }

        impl $crate::layout::LayoutTransformer for $t {
            fn transformed_name(&self) -> String {
                format!("{}<{}>", $prefix, self.0.name())
            }

            fn inner_mut(&mut self) -> &mut Box<dyn $crate::layout::Layout> {
                &mut self.0
            }

            fn unwrap(self) -> Box<dyn $crate::layout::Layout> {
                self.0
            }

            fn transform_positions(
                &mut self,
                r: $crate::geometry::Rect,
                positions: Vec<($crate::core::Xid, $crate::geometry::Rect)>,
            ) -> Vec<($crate::core::Xid, $crate::geometry::Rect)> {
                $f(r, positions)
            }
        }
    };
}

/// Wrap an existing layout and reflect its window positions horizontally.
#[derive(Clone)]
pub struct ReflectHorizontal(pub Box<dyn Layout>);
simple_transformer!("Reflected", ReflectHorizontal, reflect_horizontal);

fn reflect_horizontal(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.x + r.w / 2;

    positions
        .into_iter()
        .map(|(id, mut r)| {
            r.x = if r.x <= mid {
                2 * (mid - r.x) - r.w
            } else {
                2 * mid - r.x - r.w
            };

            (id, r)
        })
        .collect()
}

/// Wrap an existing layout and reflect its window positions vertically.
#[derive(Clone)]
pub struct ReflectVertical(pub Box<dyn Layout>);
simple_transformer!("Flipped", ReflectVertical, reflect_vertical);

fn reflect_vertical(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.y + r.h / 2;

    positions
        .into_iter()
        .map(|(id, mut r)| {
            r.y = if r.y <= mid {
                2 * (mid - r.y) - r.h
            } else {
                2 * mid - r.y - r.h
            };

            (id, r)
        })
        .collect()
}

/// Simple gaps around the window placement of the enclosed [Layout].
///
/// `outer_px` controls the width of the gap around the edge of the screen and `inner_px`
/// controls the gap around each individual window. Set both equal to one another to have
/// a consistant gap size in all places.
#[derive(Clone)]
pub struct Gaps {
    pub layout: Box<dyn Layout>,
    pub outer_px: u32,
    pub inner_px: u32,
}

impl Gaps {
    pub fn wrap(layout: Box<dyn Layout>, outer_px: u32, inner_px: u32) -> Box<dyn Layout> {
        Box::new(Self {
            layout,
            outer_px,
            inner_px,
        })
    }
}

fn shrink(r: Rect, px: u32) -> Rect {
    if r.w == 0 || r.h == 0 {
        return r;
    }

    Rect {
        x: r.x + px,
        y: r.y + px,
        w: r.w - 2 * px,
        h: r.h - 2 * px,
    }
}

impl LayoutTransformer for Gaps {
    fn transformed_name(&self) -> String {
        self.layout.name()
    }

    fn inner_mut(&mut self) -> &mut Box<dyn Layout> {
        &mut self.layout
    }

    fn unwrap(self) -> Box<dyn Layout> {
        self.layout
    }

    fn transform_initial(&self, r: Rect) -> Rect {
        shrink(r, self.outer_px)
    }

    fn transform_positions(&mut self, _: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
        positions
            .into_iter()
            .map(|(id, r)| (id, shrink(r, self.inner_px)))
            .collect()
    }
}

/// Reserve `px` pixels at the top of the screen.
///
/// Typically used for providing space for a status bar.
#[derive(Clone)]
pub struct ReserveTop {
    pub layout: Box<dyn Layout>,
    pub px: u32,
}

impl ReserveTop {
    pub fn wrap(layout: Box<dyn Layout>, px: u32) -> Box<dyn Layout> {
        Box::new(Self { layout, px })
    }
}

impl LayoutTransformer for ReserveTop {
    fn transformed_name(&self) -> String {
        self.layout.name()
    }

    fn inner_mut(&mut self) -> &mut Box<dyn Layout> {
        &mut self.layout
    }

    fn unwrap(self) -> Box<dyn Layout> {
        self.layout
    }

    fn transform_initial(&self, mut r: Rect) -> Rect {
        if r.w == 0 || r.h == 0 {
            return r;
        }

        r.y += self.px;
        r.h -= self.px;

        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(Rect::new(0, 0, 100, 200), Rect::new(0, 0, 100, 200); "fullscreen is idempotent")]
    #[test_case(Rect::new(0, 0, 40, 100), Rect::new(60, 0, 40, 100); "not crossing midpoint left")]
    #[test_case(Rect::new(60, 0, 40, 100), Rect::new(0, 0, 40, 100); "not crossing midpoint right")]
    #[test_case(Rect::new(0, 0, 60, 100), Rect::new(40, 0, 60, 100); "crossing midpoint")]
    #[test_case(Rect::new(0, 0, 50, 100), Rect::new(50, 0, 50, 100); "on midpoint")]
    #[test]
    fn reflect_horizontal(original: Rect, expected: Rect) {
        let r = Rect::new(0, 0, 100, 200);
        let transformed = reflect_horizontal(r, vec![(Xid(1), original)]);

        assert_eq!(transformed, vec![(Xid(1), expected)]);
    }

    #[test_case(Rect::new(0, 0, 100, 200), Rect::new(0, 0, 100, 200); "fullscreen is idempotent")]
    #[test_case(Rect::new(0, 0, 50, 80), Rect::new(0, 120, 50, 80); "not crossing midpoint above")]
    #[test_case(Rect::new(0, 120, 50, 80), Rect::new(0, 0, 50, 80); "not crossing midpoint below")]
    #[test_case(Rect::new(0, 0, 50, 120), Rect::new(0, 80, 50, 120); "crossing midpoint")]
    #[test_case(Rect::new(0, 0, 50, 100), Rect::new(0, 100, 50, 100); "on midpoint")]
    #[test]
    fn reflect_vertical(original: Rect, expected: Rect) {
        let r = Rect::new(0, 0, 100, 200);
        let transformed = reflect_vertical(r, vec![(Xid(1), original)]);

        assert_eq!(transformed, vec![(Xid(1), expected)]);
    }
}
