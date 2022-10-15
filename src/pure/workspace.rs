use crate::{
    layout::{IntoMessage, LayoutStack},
    pure::Stack,
};

#[derive(Default, Debug, Clone)]
pub struct Workspace<C> {
    pub(crate) id: usize,
    pub(crate) tag: String,
    pub(crate) layouts: LayoutStack,
    pub(crate) stack: Option<Stack<C>>,
}

impl<C> Workspace<C> {
    pub fn new<T>(id: usize, tag: T, layouts: LayoutStack, stack: Option<Stack<C>>) -> Self
    where
        T: Into<String>,
    {
        Self {
            id,
            tag: tag.into(),
            layouts,
            stack,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_none()
    }

    pub(crate) fn remove_focused(&mut self) -> Option<C> {
        let current = self.stack.take();
        let (focus, new_stack) = current?.remove_focused();
        self.stack = new_stack;

        Some(focus)
    }

    pub(crate) fn remove(&mut self, c: &C) -> Option<C>
    where
        C: PartialEq,
    {
        let current = self.stack.take();
        let (maybe_c, new_stack) = current?.remove(c);
        self.stack = new_stack;

        maybe_c
    }

    pub fn handle_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.handle_message(m)
    }

    pub fn broadcast_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.broadcast_message(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack;
    use simple_test_case::test_case;

    #[test_case(Some(stack!([1, 2], 3, [4, 5])), Some(5), true; "known in stack")]
    #[test_case(Some(stack!(5)), Some(5), false; "known focus only")]
    #[test_case(Some(stack!([1, 2], 3, [4])), None, true; "unknown")]
    #[test_case(None, None, false; "empty stack")]
    #[test]
    fn remove_returns_as_expected(stack: Option<Stack<u8>>, maybe_c: Option<u8>, is_some: bool) {
        let mut w = Workspace::new(0, "test", LayoutStack::default(), stack);

        assert_eq!(w.remove(&5), maybe_c);
        assert_eq!(w.stack.is_some(), is_some);
    }
}
