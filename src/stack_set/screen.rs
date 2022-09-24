use crate::Workspace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen<C, D> {
    pub(crate) index: usize,
    pub(crate) workspace: Workspace<C>,
    pub(crate) screen_detail: D,
}
