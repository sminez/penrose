use crate::state::{Layout, Stack};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace<C> {
    pub(crate) tag: String,
    pub(crate) layout: Layout,
    pub(crate) stack: Option<Stack<C>>,
}

impl<C> Workspace<C> {
    pub fn new<T>(tag: T, layout: Layout, stack: Option<Stack<C>>) -> Self
    where
        T: Into<String>,
    {
        Self {
            tag: tag.into(),
            layout,
            stack,
        }
    }

    pub fn empty<T>(tag: T, layout: Layout) -> Self
    where
        T: Into<String>,
    {
        Self::new(tag, layout, None)
    }
}
