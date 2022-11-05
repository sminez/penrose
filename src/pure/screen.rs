use crate::pure::{geometry::Rect, Workspace};
use std::fmt;

/// A wrapper around a single [Workspace] that includes the physical screen
/// size as a [Rect].
#[derive(Default, Debug, Clone)]
pub struct Screen<C> {
    pub(crate) index: usize,
    /// The [Workspace] current visible on this screen
    pub workspace: Workspace<C>,
    pub(crate) r: Rect,
}

impl<C: fmt::Display> fmt::Display for Screen<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Screen({}: {:?}):\n  - workspace: {}",
            self.index, self.r, self.workspace
        )
    }
}

impl<C> Screen<C> {
    /// The index of this screen.
    ///
    /// Indices are assigned from left to right based on the absolute position of
    /// their top left corner.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The physical screen size of this [Screen] as a [Rect].
    pub fn geometry(&self) -> Rect {
        self.r
    }
}
