//! Debugging utilities for diagnosing issues with penrose.
use crate::{
    core::{hooks::StateHook, State},
    extensions::util::notify_send,
    x::XConn,
    Result,
};

/// Use `notify-send` to display details about the current Window Manager each
/// time there is a refresh
#[derive(Default, Debug, Clone, Copy)]
pub struct NotfyState(pub CurrentStateConfig);

impl<X: XConn> StateHook<X> for NotfyState {
    fn call(&mut self, state: &mut State<X>, _: &X) -> Result<()> {
        let msg = summarise_state(state, &self.0);

        notify_send("Current State", msg)
    }
}

/// All fields default to true
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentStateConfig {
    pub focused_screen: bool,
    pub focused_client: bool,
    pub focused_tag: bool,
    pub n_clients: bool,
    pub n_mapped_clients: bool,

    pub line_per_stat: bool,
}

impl Default for CurrentStateConfig {
    fn default() -> Self {
        Self {
            focused_screen: true,
            focused_client: true,
            focused_tag: true,
            n_clients: true,
            n_mapped_clients: true,
            line_per_stat: true,
        }
    }
}

/// Summarise the current state of the window manager as simple key value pairs.
pub fn summarise_state<X: XConn>(state: &State<X>, cfg: &CurrentStateConfig) -> String {
    let mut fields = Vec::new();

    if cfg.focused_screen {
        fields.push(format!("screen={}", state.client_set.screens.focus.index()));
    }

    if cfg.focused_client {
        let c = state.client_set.current_client();

        fields.push(format!(
            "client={}",
            c.map_or("None".to_owned(), |c| (*c).to_string())
        ));
    }

    if cfg.focused_tag {
        fields.push(format!("tag={}", state.client_set.current_tag()));
    }

    if cfg.n_clients {
        fields.push(format!("n_clients={}", state.client_set.clients().count()));
    }

    if cfg.n_mapped_clients {
        fields.push(format!("n_mapped={}", state.mapped.len()));
    }

    if cfg.line_per_stat {
        fields.join("\n")
    } else {
        fields.join(", ")
    }
}
