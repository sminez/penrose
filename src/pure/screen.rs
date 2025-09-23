use crate::pure::{
    Stack, Workspace,
    geometry::{Rect, RelativeRect},
};
use std::{collections::HashMap, fmt, hash::Hash};

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

impl<C> Screen<C>
where
    C: Clone + Eq + Hash,
{
    /// Produce a summary of the clients contained within this screen
    pub fn screen_clients(&self, floating: &HashMap<C, RelativeRect>) -> ScreenClients<C> {
        ScreenClients {
            floating: self
                .workspace
                .clients()
                .flat_map(|c| floating.get(c).map(|r| (c.clone(), *r)))
                .collect(),
            tiling: self
                .workspace
                .stack
                .as_ref()
                .and_then(|st| st.from_filtered(|c| !floating.contains_key(c))),
            tag: self.workspace.tag.clone(),
            r_s: self.r,
        }
    }
}

/// Used in laying out visible_client_positions
#[derive(Debug)]
pub struct ScreenClients<C> {
    /// Floating client positions on this screen
    pub floating: Vec<(C, RelativeRect)>,
    /// Clients that need to be tiled on this screen
    pub tiling: Option<Stack<C>>,
    /// The workspace tag currently visible on this screen
    pub tag: String,
    /// The dimensions of this screen
    pub r_s: Rect,
}
