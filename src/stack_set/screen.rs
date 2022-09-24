use crate::{geometry::Rect, stack_set::Workspace};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen<C> {
    pub(crate) index: usize,
    pub(crate) workspace: Workspace<C>,
    pub(crate) screen_detail: Rect,
}
