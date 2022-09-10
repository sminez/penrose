use crate::{Client, Layout, Stack};

#[derive(Debug, PartialEq, Eq)]
pub struct Workspace {
    pub(crate) tag: String,
    pub(crate) layout: Layout,
    pub(crate) stack: Option<Stack<Client>>,
}

impl Workspace {
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
}
