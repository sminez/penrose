use crate::state::{Layout, Stack};

#[derive(Debug, PartialEq, Eq)]
pub struct Workspace<C> {
    pub(crate) tag: String,
    pub(crate) layout: Layout,
    pub(crate) stack: Option<Stack<C>>,
}

impl<C> Workspace<C> {
    pub fn empty<T>(tag: T, layout: Layout) -> Self
    where
        T: Into<String>,
    {
        Self {
            tag: tag.into(),
            layout,
            stack: None,
        }
    }

    pub fn with_stack<T>(tag: T, layout: Layout, stack: Stack<C>) -> Self
    where
        T: Into<String>,
    {
        Self {
            tag: tag.into(),
            layout,
            stack: Some(stack),
        }
    }
}
