use crate::Workspace;

#[derive(Debug, PartialEq, Eq)]
pub struct Screen {
    pub(crate) index: usize,
    pub(crate) workspace: Workspace,
    pub(crate) screen_detail: ScreenDetail,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScreenDetail {}
