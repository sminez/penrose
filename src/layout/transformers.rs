use crate::{
    core::Xid,
    geometry::Rect,
    layout::{
        messages::{common::UnwrapTransformer, Message},
        Layout,
    },
    stack_set::Stack,
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
    fn run_transform<F>(&mut self, f: F, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(Rect, &mut Box<dyn Layout>) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>),
    {
        let r = self.transform_initial(r);
        let (new, positions) = (f)(r.clone(), self.inner_mut());
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
        self.run_transform(|r, inner| inner.layout_workspace(tag, stack, r.clone()), r)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|r, inner| inner.layout(s, r.clone()), r)
    }

    fn layout_empty(&mut self, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|r, inner| inner.layout_empty(r.clone()), r)
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
/// simple_transformer!(MyTransformer, my_transformation_function);
/// ```
#[macro_export]
macro_rules! simple_transformer {
    ($t:ident, $f:ident) => {
        impl $crate::layout::LayoutTransformer for $t {
            fn transformed_name(&self) -> String {
                format!("{}<{}>", stringify!($name), self.0.name())
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
simple_transformer!(ReflectHorizontal, reflect_horizontal);

fn reflect_horizontal(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.y + r.h / 2;

    positions
        .into_iter()
        .map(|(id, r)| {
            let Rect { x, y, w, h } = r;
            let x = 2 * mid - x;

            (id, Rect { x, y, w, h })
        })
        .collect()
}

/// Wrap an existing layout and reflect its window positions vertically.
#[derive(Clone)]
pub struct ReflectVertical(pub Box<dyn Layout>);
simple_transformer!(ReflectVertical, reflect_vertical);

fn reflect_vertical(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.x + r.w / 2;

    positions
        .into_iter()
        .map(|(id, r)| {
            let Rect { x, y, w, h } = r;
            let y = 2 * mid - y;

            (id, Rect { x, y, w, h })
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

fn shrink(r: Rect, px: u32) -> Rect {
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
