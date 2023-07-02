//! A manual tiling layout based on binary space partitioning
//!
//! The data structure used for manipulating the tree and tracking focus is a 'Zipper'
//! as described by Huet(97):
//!   https://www.st.cs.uni-saarland.de/edu/seminare/2005/advanced-fp/docs/huet-zipper.pdf
//!
//! The Haskell Wiki has a more accessible article on how this concept works for binary
//! trees (https://wiki.haskell.org/Zipper) as does Learn You A Haskell (http://learnyouahaskell.com/zippers)

use crate::{
    builtin::layout::messages::{ExpandMain, Rotate, ShrinkMain},
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};

use Context::*;
use Node::*;
use Side::*;

/// An individual node in the BSP tree
#[derive(Debug, Clone)]
pub enum Node {
    /// A leaf node that is occupied by a window
    Leaf,
    /// A split with two child nodes
    Split {
        /// The L/R ratio for the split
        ratio: f32,
        /// Whether the split is horizontal or vertical
        hsplit: bool,
        /// The left child
        l: Box<Node>,
        /// The right child
        r: Box<Node>,
    },
}

impl Node {
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
            if *ratio < 1.0 {
                *ratio = 1.0;
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
        n: Node,
    },
}

impl Context {
    fn take(&mut self) -> Self {
        let mut c = Root;
        std::mem::swap(self, &mut c);

        c
    }
}

/// A BSP-tree zipper for traversing the current client tree and making modifications
#[derive(Debug, Clone)]
pub struct Zipper {
    n: Node,
    c: Context,
}

impl Zipper {
    /// Create a new zipper from a given root node
    pub fn from_root(n: Node) -> Zipper {
        Zipper { n, c: Root }
    }

    /// Shift focus to the [Left] side of the focused [Split].
    ///
    /// If the focused [Node] is a [Leaf] then it is returned
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
    /// If the focused [Node] is a [Leaf] then it is returned
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

    /// Shift focus to the parent of the focused [Node].
    ///
    /// If the focused [Node] is already the [Root] of the tree then
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
    pub fn clone_tree(&self) -> Node {
        let mut z = self.clone();
        z.focus_root();
        z.n
    }
}

/// A manual tiling layout using binary space partitioning.
#[derive(Debug, Clone)]
pub struct BSP {
    zipper: Zipper,
    hsplit: bool,
    ratio: f32,
    ratio_step: f32,
}

impl BSP {
    /// Split the focused [Node] and then focus the [Right] side of the
    /// new child [Split].
    pub fn split(&mut self) {
        self.zipper.n.split(self.ratio, self.hsplit);
        self.zipper.focus_right();
    }

    /// Toggle the orientation of future splits between horizontal and vertical
    pub fn toggle_orientation(&mut self) {
        self.hsplit = !self.hsplit;
    }
}

impl Default for BSP {
    fn default() -> Self {
        Self {
            zipper: Zipper::from_root(Leaf),
            hsplit: false,
            ratio: 0.5,
            ratio_step: 0.1,
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
        let rs = self.zipper.clone_tree().into_rects(r);
        let positions = s.iter().zip(rs).map(|(&id, r)| (id, r)).collect();

        (None, positions)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if let Some(&ExpandMain) = m.downcast_ref() {
            self.zipper.n.expand_split(self.ratio_step)
        } else if let Some(&ShrinkMain) = m.downcast_ref() {
            self.zipper.n.shrink_split(self.ratio_step)
        } else if let Some(&Rotate) = m.downcast_ref() {
            self.zipper.n.rotate();
        };

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::print_layout_result;

    // Not actually a test at the moment: just running under --nocapture to eyeball the output
    #[test]
    fn tree_zipper_traversal() {
        let mut bsp = BSP::default();
        bsp.split();
        bsp.split();
        bsp.toggle_orientation();
        bsp.split();
        print_layout_result(&mut bsp, 6, 40, 15);

        bsp.zipper.focus_up();
        bsp.zipper.focus_up();
        bsp.zipper.n.rotate();
        print_layout_result(&mut bsp, 6, 40, 15);
    }
}
