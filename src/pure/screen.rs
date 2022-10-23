use crate::{geometry::Rect, pure::Workspace};

#[derive(Default, Debug, Clone)]
pub struct Screen<C> {
    pub(crate) index: usize,
    pub workspace: Workspace<C>,
    pub(crate) r: Rect,
}

impl<C> Screen<C> {
    pub fn index(&self) -> usize {
        self.index
    }

    // TODO: add logic for reserving space for a bar etc
    pub fn visible_rect(&self) -> Rect {
        self.r
    }
}
