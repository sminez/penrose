//! A manual tiling layout based on binary space partitioning
//!
//! The data structure used for manipulating the tree and tracking focus is a 'Zipper'
//! as described by Huet(97):
//!   https://www.st.cs.uni-saarland.de/edu/seminare/2005/advanced-fp/docs/huet-zipper.pdf
//!
//! The Haskell Wiki has a more accessible article on how this concept works for binary
//! trees (https://wiki.haskell.org/Zipper) as does Learn You A Haskell (http://learnyouahaskell.com/zippers)
//!
//! NOTE: This layout is very much a work in progress!
use crate::{
    builtin::layout::messages::{ExpandMain, Rotate, ShrinkMain},
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};

// TODO:
//  - Keep internal client and focus state in sync with the Workspace Stack
//  - Removal of focused client -> collapsing the current split
//  - Shrink / expand split needs to operate on the split holding the focused
//    node rather than on nodes directly
//    - The current impl means that you have tomove focus to the parent before
//      resizing will work
//  - NSEW navigation of windows
//  - Auto-balancing of the tree

use AutoSplit::*;
use BspTree::*;
use Context::*;
use Side::*;

/// An individual node in a BSP tree for a given screen.
///
/// A [BspTree] represents a specific sub-tree for a selected node,
/// containing all children under it as well.
#[derive(Debug, Clone)]
pub enum BspTree {
    /// A leaf node that is occupied by a window
    Leaf,
    /// A split with two child nodes
    Split {
        /// The L/R ratio for the split
        ratio: f32,
        /// Whether the split is horizontal or vertical
        hsplit: bool,
        /// The left child
        l: Box<BspTree>,
        /// The right child
        r: Box<BspTree>,
    },
}

impl BspTree {
    /// The number of [Leaf] elements in this [BspTree].
    pub fn len(&self) -> usize {
        match self {
            Leaf => 1,
            Split { l, r, .. } => l.len() + r.len(),
        }
    }

    /// A [BspTree] is never empty: there is always at least one [Leaf] node.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Split this node into two children
    pub fn split(&mut self, ratio: f32, hsplit: bool) {
        *self = match self {
            Leaf => Split {
                ratio,
                hsplit,
                l: Box::new(Leaf),
                r: Box::new(Leaf),
            },
            Split { .. } => Split {
                ratio,
                hsplit,
                l: Box::new(self.clone()),
                r: Box::new(Leaf),
            },
        }
    }

    /// Expand the size of the [Left] side of a split.
    ///
    /// For a [Leaf] this is a no-op.
    pub fn expand_split(&mut self, step: f32) {
        if let Split { ratio, .. } = self {
            *ratio += step;
            if *ratio > 1.0 {
                *ratio = 1.0;
            }
        }
    }

    /// Shrink the size of the [Left] side of a split.
    ///
    /// For a [Leaf] this is a no-op.
    pub fn shrink_split(&mut self, step: f32) {
        if let Split { ratio, .. } = self {
            *ratio -= step;
            if *ratio < 0.0 {
                *ratio = 0.0;
            }
        }
    }

    /// Rotate the orientation of a [Split] node
    ///
    /// For a [Leaf] this is a no-op.
    pub fn rotate(&mut self) {
        if let Split { hsplit, l, r, .. } = self {
            *hsplit = !*hsplit;
            l.rotate();
            r.rotate();
        }
    }

    fn take(&mut self) -> Self {
        let mut n = Leaf;
        std::mem::swap(self, &mut n);

        n
    }

    fn into_rects(self, parent: Rect) -> Vec<Rect> {
        match self {
            Leaf => vec![parent],
            Split {
                ratio,
                hsplit,
                l,
                r,
            } => {
                let (rl, rr) = if hsplit {
                    parent.split_at_height_perc(ratio).unwrap()
                } else {
                    parent.split_at_width_perc(ratio).unwrap()
                };

                let mut rects = l.into_rects(rl);
                rects.extend(r.into_rects(rr));

                rects
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone)]
enum Context {
    Root,
    Branch {
        ratio: f32,
        hsplit: bool,
        s: Side,
        c: Box<Context>,
        n: BspTree,
    },
}

impl Context {
    fn take(&mut self) -> Self {
        let mut c = Root;
        std::mem::swap(self, &mut c);

        c
    }

    fn len(&self) -> usize {
        match self {
            Root => 0,
            Branch { c, n, .. } => c.len() + n.len(),
        }
    }
}

/// A BSP-tree zipper for traversing the current client tree and making modifications
#[derive(Debug, Clone)]
pub struct Zipper {
    n: BspTree,
    c: Context,
}

impl Zipper {
    /// The number of [Leaf] elements contained in the underlying [BspTree].
    pub fn len(&self) -> usize {
        self.n.len() + self.c.len()
    }

    /// The underlying [BspTree] is never empty: there is always at least one [Leaf] node.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Create a new zipper from a given root node
    pub fn from_root(n: BspTree) -> Zipper {
        Zipper { n, c: Root }
    }

    /// Shift focus to the [Left] side of the focused [Split].
    ///
    /// If the focused [BspTree] is a [Leaf] then it is returned
    /// unchanged.
    pub fn focus_left(&mut self) {
        let (n, c) = (self.n.take(), self.c.take());
        (self.n, self.c) = match (n, c) {
            (Leaf, c) => (Leaf, c),
            (
                Split {
                    ratio,
                    hsplit,
                    l,
                    r,
                },
                c,
            ) => (
                *l,
                Branch {
                    ratio,
                    hsplit,
                    s: Left,
                    c: Box::new(c),
                    n: *r,
                },
            ),
        }
    }

    /// Shift focus to the [Right] side of the focused [Split].
    ///
    /// If the focused [BspTree] is a [Leaf] then it is returned
    /// unchanged.
    pub fn focus_right(&mut self) {
        let (n, c) = (self.n.take(), self.c.take());
        (self.n, self.c) = match (n, c) {
            (Leaf, c) => (Leaf, c),
            (
                Split {
                    ratio,
                    hsplit,
                    l,
                    r,
                },
                c,
            ) => (
                *r,
                Branch {
                    ratio,
                    hsplit,
                    s: Right,
                    c: Box::new(c),
                    n: *l,
                },
            ),
        }
    }

    /// Shift focus to the parent of the focused [BspTree].
    ///
    /// If the focused [BspTree] is already the [Root] of the tree then
    /// it is returned unchanged.
    pub fn focus_up(&mut self) {
        let (n, c) = (self.n.take(), self.c.take());

        (self.n, self.c) = match c {
            Root => (n, Root),
            Branch {
                s,
                c,
                n: cn,
                ratio,
                hsplit,
            } => {
                let (l, r) = match s {
                    Left => (Box::new(n), Box::new(cn)),
                    Right => (Box::new(cn), Box::new(n)),
                };
                (
                    Split {
                        ratio,
                        hsplit,
                        l,
                        r,
                    },
                    *c,
                )
            }
        }
    }

    /// Focus the root of the tree
    pub fn focus_root(&mut self) {
        if matches!(self.c, Root) {
            return;
        }
        self.focus_up();
        self.focus_root();
    }

    /// Return a clone of the underlying BSP tree
    pub fn clone_tree(&self) -> BspTree {
        let mut z = self.clone();
        z.focus_root();
        z.n
    }
}

/// How splits should be created when new multiple clients are added at once.
///
/// This happens when [BSP] attempts to layout a workspace and discovers that there
/// are more clients than expected from the previous layout attempt:
#[derive(Debug, Clone)]
pub enum AutoSplit {
    /// Always split horizontally
    Horizontal,
    /// Always split vertically
    Vertical,
    /// Use the current split orientation used for manual splits
    Current,
    /// Use the current orientation then toggle for the next client
    Alternate,
}

/// A manual tiling layout using binary space partitioning.
#[derive(Debug, Clone)]
pub struct BSP {
    zipper: Option<Zipper>,
    hsplit: bool,
    auto_split: AutoSplit,
    ratio: f32,
    ratio_step: f32,
    // TODO: remove this!
    /// Only show the focused node
    pub focused_only: bool,
}

impl BSP {
    /// Split the focused [BspTree] and then focus the [Right] side of the new child [Split].
    pub fn split(&mut self) {
        self._split(self.hsplit)
    }

    fn _split(&mut self, hsplit: bool) {
        match self.zipper.as_mut() {
            None => self.zipper = Some(Zipper::from_root(Leaf)),
            Some(z) => {
                z.n.split(self.ratio, hsplit);
                z.focus_right();
            }
        }
    }

    /// Toggle the orientation of future splits between horizontal and vertical
    pub fn toggle_orientation(&mut self) {
        self.hsplit = !self.hsplit;
    }

    /// Rotate the focused node
    pub fn rotate(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.n.rotate()
        }
    }

    /// Expand the left side of the current split
    pub fn expand_split(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.n.expand_split(self.ratio_step)
        }
    }

    /// Shrink the left side of the current split
    pub fn shrink_split(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.n.shrink_split(self.ratio_step)
        }
    }

    /// Move focus to the parent node
    pub fn focus_up(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.focus_up()
        }
    }

    /// Move focus to the left side of the current split
    pub fn focus_left(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.focus_left()
        }
    }

    /// Move focus to the right side of the current split
    pub fn focus_right(&mut self) {
        if let Some(z) = self.zipper.as_mut() {
            z.focus_right()
        }
    }

    /// Add an additional `n` clients to the tree using [AutoSplit].
    pub fn auto_split(&mut self, n: usize) {
        for _ in 0..n {
            match self.auto_split {
                Horizontal => self._split(true),
                Vertical => self._split(false),
                Current => self._split(self.hsplit),
                Alternate => {
                    self._split(self.hsplit);
                    self.toggle_orientation();
                }
            }
        }
    }
}

impl Default for BSP {
    fn default() -> Self {
        Self {
            zipper: None,
            hsplit: false,
            auto_split: Alternate,
            ratio: 0.5,
            ratio_step: 0.1,
            focused_only: false,
        }
    }
}

impl Layout for BSP {
    fn name(&self) -> String {
        if self.hsplit { "BSP-" } else { "BSP|" }.to_string()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(self.clone())
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let rs = self
            .zipper
            .as_ref()
            .map(|z| z.clone_tree().into_rects(r))
            .unwrap_or_default();
        let mut positions: Vec<_> = s.iter().zip(rs).map(|(&id, r)| (id, r)).collect();

        if self.focused_only {
            if let Some(z) = self.zipper.as_ref() {
                let pos = z.c.len();
                positions = vec![positions[pos]];
            }
        }

        (None, positions)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        let z = self.zipper.as_mut()?;

        if let Some(&ExpandMain) = m.downcast_ref() {
            z.n.expand_split(self.ratio_step)
        } else if let Some(&ShrinkMain) = m.downcast_ref() {
            z.n.shrink_split(self.ratio_step)
        } else if let Some(&Rotate) = m.downcast_ref() {
            z.n.rotate();
        };

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Not actually a test at the moment: just running under --nocapture to eyeball the output
    // #[test]
    // fn tree_zipper_traversal() {
    //     use crate::util::print_layout_result;

    //     let mut bsp = BSP::default();
    //     bsp.split();
    //     bsp.split();
    //     bsp.toggle_orientation();
    //     bsp.split();
    //     print_layout_result(&mut bsp, 6, 40, 15);

    //     bsp.zipper.focus_up();
    //     bsp.zipper.focus_up();
    //     bsp.zipper.n.rotate();
    //     print_layout_result(&mut bsp, 6, 40, 15);
    // }

    #[test]
    fn zipper_len_matches_tree_len() {
        let mut bsp = BSP::default();
        bsp.auto_split(42);

        let z = bsp.zipper.take().unwrap();

        assert_eq!(z.len(), 42);
        assert_eq!(z.clone_tree().len(), 42);
    }
}
