use crate::{
    core::{
        layout::{
            messages::{common::UnwrapTransformer, Message},
            Layout,
        },
        Xid,
    },
    geometry::Rect,
    state::{Stack, Workspace},
};

/// A wrapper round another [Layout] that is able to intercept and modify both the positions being
/// returned by the inner layout and messages being sent to it.
pub trait LayoutTransformer: Sized + 'static {
    /// The same as [Layout::name] but for [LayoutTransformer] itself.
    fn transformed_name(&self) -> String;

    /// Optionally modify any of the positions returned by the inner [Layout] before they are
    /// applied by the window manager. The dimensions of the screen being layed out are avaiable
    /// as `r`.
    fn transform_positions(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)>;

    /// Provide a mutable reference to the [Layout] wrapped by this transformer.
    fn inner_mut(&mut self) -> &mut dyn Layout;

    /// Replace the currently wrapped [Layout] with a new one.
    fn swap_inner(&mut self, new: Box<dyn Layout>) -> Box<dyn Layout>;

    /// Remove the inner [Layout] from this [LayoutTransformer].
    fn unwrap(self) -> Box<dyn Layout>;

    /// Apply the [LayoutTransformer] to its wrapped inner [Layout].
    fn run_transform<F>(&mut self, f: F, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(&mut dyn Layout) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>),
    {
        let (new, positions) = (f)(self.inner_mut());
        let transformed = Self::transform_positions(r, positions);

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
        if let Some(&UnwrapTransformer) = m.downcast_ref() {
            return Some(self.swap_inner(Box::new(NullLayout)));
        }

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

    fn layout_workspace(
        &mut self,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|inner| inner.layout_workspace(w, r.clone()), r)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|inner| inner.layout(s, r.clone()), r)
    }

    fn layout_empty(&mut self, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.run_transform(|inner| inner.layout_empty(r.clone()), r)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        self.passthrough_message(m)
    }
}

/// A valid impl of Layout that can be used as a placeholder but will panic if used.
struct NullLayout;
impl Layout for NullLayout {
    fn name(&self) -> String {
        panic!("Null layout")
    }

    fn layout_workspace(
        &mut self,
        _: &Workspace<Xid>,
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
/// # use penrose::{core::{layout::Layout, Xid}, geometry::Rect, simple_transformer};
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
        impl $crate::core::layout::LayoutTransformer for $t {
            fn transformed_name(&self) -> String {
                format!("{}<{}>", stringify!($name), self.0.name())
            }

            fn inner_mut(&mut self) -> &mut dyn $crate::core::layout::Layout {
                &mut *self.0
            }

            fn swap_inner(
                &mut self,
                mut new: Box<dyn $crate::core::layout::Layout>,
            ) -> Box<dyn $crate::core::layout::Layout> {
                std::mem::swap(&mut self.0, &mut new);
                new
            }

            fn unwrap(self) -> Box<dyn $crate::core::layout::Layout> {
                self.0
            }

            fn transform_positions(
                r: $crate::geometry::Rect,
                positions: Vec<($crate::core::Xid, $crate::geometry::Rect)>,
            ) -> Vec<($crate::core::Xid, $crate::geometry::Rect)> {
                $f(r, positions)
            }
        }
    };
}

/// Wrap an existing layout and reflect its window positions horizontally.
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
