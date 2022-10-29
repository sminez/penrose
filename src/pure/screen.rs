use crate::pure::{geometry::Rect, Workspace};

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

    pub fn geometry(&self) -> Rect {
        self.r
    }
}
