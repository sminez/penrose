//! A manual tiling layout based on binary space partitioning

// TODO:
// - Need a default way of filling the tree if we switch to this layout without
//   a tree being present.
//   - Auto-fill is probably easiest as the fibonacci layout?
// - Need to be able to update state in an intuitive way when switching back to
//   this layout from another one if the number of clients has changed.
//   - Drop / extend from the leaves?

#[derive(Debug, Default, Clone)]
enum DownNode {
    #[default]
    Leaf,
    Split {
        ratio: f32,
        hsplit: bool,
        left: Box<DownNode>,
        right: Box<DownNode>,
    },
}

impl DownNode {
    fn split(&mut self, ratio: f32, hsplit: bool) {
        *self = match self {
            DownNode::Leaf => DownNode::Split {
                ratio,
                hsplit,
                left: Default::default(),
                right: Default::default(),
            },

            DownNode::Split { .. } => DownNode::Split {
                ratio,
                hsplit,
                left: Box::new(self.clone()),
                right: Default::default(),
            },
        };
    }
}

#[derive(Debug, Clone)]
struct Parent {
    parent: Option<Box<Parent>>,
    other: Box<Side>,
    ratio: f32,
    hsplit: bool,
}

#[derive(Debug, Default, Clone)]
enum UpNode {
    #[default]
    Leaf,
    Parent(Parent),
}

#[derive(Debug, Clone)]
enum Side {
    Left(UpNode),
    Right(UpNode),
}

#[derive(Debug, Default, Clone)]
pub struct BspZipper {
    focus: DownNode,
    parent: Option<Parent>,
}

impl BspZipper {
    fn focus_parent(&mut self) {
        let mut p = match self.parent.take() {
            Some(ref mut p) => p,
            None => return,
        };

        match p.side {
            Side::Left(sibling) => {}

            Side::Right(sibling) => {}
        }
    }

    fn focus_left(&mut self) {}

    fn focus_right(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_works() {
        let mut z = BspZipper::default();
        z.focus.split(0.4, true);
        z.focus.split(0.7, false);
        assert_eq!(format!("{z:?}"), "");
    }
}
