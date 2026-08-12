//! Configure workspaces to auto-spawn a set of windows if they are empty when they gain focus
use crate::{
    Result,
    core::{State, conn::Conn, hooks::StateHook},
    util::spawn,
};

/// Specify a workspace by `tag` and use a named layout to spawn a set of default programs
/// on it if it gains focus while currently empty.
///
/// The programs are spawned in the order they are specified in [DefaultWorkspace::boxed] meaning
/// that the final program `progs` will have focus.
#[derive(Debug, Clone)]
pub struct DefaultWorkspace {
    tag: String,
    layout_name: String,
    progs: Vec<String>,
}

impl DefaultWorkspace {
    /// Create a new boxed `DefaultWorkspace` that can be added to your Config as a refresh hook.
    pub fn boxed<C>(
        tag: impl Into<String>,
        layout_name: impl Into<String>,
        progs: Vec<impl Into<String>>,
    ) -> Box<dyn StateHook<C>>
    where
        C: Conn,
    {
        Box::new(Self {
            tag: tag.into(),
            layout_name: layout_name.into(),
            progs: progs.into_iter().map(|p| p.into()).collect(),
        })
    }
}

impl<C> StateHook<C> for DefaultWorkspace
where
    C: Conn,
{
    fn call(&mut self, state: &mut State<C>, _c: &mut C) -> Result<()> {
        let on_screen_and_empty = std::iter::once(&state.diff.after.focused)
            .chain(state.diff.after.visible.iter())
            .find(|s| s.tag == self.tag)
            .map(|s| s.clients.is_empty())
            .unwrap_or(false);

        if on_screen_and_empty
            && !state
                .diff
                .previous_visible_tags()
                .contains(&self.tag.as_str())
        {
            state.client_set.set_layout_by_name(&self.layout_name);
            self.progs.iter().try_for_each(spawn)?;
        }

        Ok(())
    }
}
