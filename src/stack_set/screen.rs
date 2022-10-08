use crate::{geometry::Rect, stack_set::Workspace};

#[derive(Default, Debug, Clone)]
pub struct Screen<C> {
    pub(crate) index: usize,
    pub(crate) workspace: Workspace<C>,
    pub(crate) r: Rect,
}

impl<C> Screen<C> {
    // TODO: add logic for reserving space for a bar etc
    pub fn visible_rect(&self) -> Rect {
        self.r
    }
}
