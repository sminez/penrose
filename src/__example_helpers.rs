//! Internal doc-test example helpers. NOT A PUBLIC API
//!
//! # WARNING
//!
//! The contents of this module can and will be modified in breaking ways that will not be refleted
//! in the semantic versioning of Penrose itself. This module is intended purely for supporting
//! internal doc tests and reducing boilerplate.
pub use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, KeyEventHandler, MouseBindings},
        client::Client,
        config::Config,
        data_types::{Region, ResizeAction},
        layout::{Layout, LayoutConf},
        ring::Selector,
        screen::Screen,
        workspace::Workspace,
        xconnection::{StubXConn, XConn, XEvent},
    },
    logging_error_handler, Backward, Forward, Less, More, PenroseError, Result, WinId,
    WindowManager,
};

pub use std::{cell::Cell, collections::HashMap, fmt};

/// NOTE: Boiler plate example set up for WindowManager doc-tests
/// # Example
///
/// ```
/// # use penrose::__example_helpers::*;
/// # fn example(mut manager: ExampleWM) -> Result<()> {
/// // EXAMPLE HERE
/// # Ok(())
/// # }
/// # example(example_windowmanager(1, vec![])).unwrap();
/// ```

/// NOTE: Boiler plate example set up for WindowManager doc-tests which
///       require some set up of state.
/// # Example
///
/// ```
/// # use penrose::__example_helpers::*;
/// # fn example(mut manager: ExampleWM) -> Result<()> {
/// // EXAMPLE HERE
/// # Ok(())
/// # }
/// # let mut manager = example_windowmanager(1, n_clients(3));
/// # manager.init().unwrap();
/// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
/// # example(manager).unwrap();
/// ```

pub type ExampleWM = WindowManager<ExampleXConn>;
pub type ExampleKeyBindings = KeyBindings<ExampleXConn>;
pub type ExampleKeyHandler = KeyEventHandler<ExampleXConn>;
pub type ExampleMouseBindings = MouseBindings<ExampleXConn>;

pub const EXIT_CODE: KeyCode = KeyCode { mask: 0, code: 0 };
pub const LAYOUT_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 1 };
pub const WORKSPACE_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 2 };
pub const SCREEN_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 3 };
pub const FOCUS_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 4 };
pub const KILL_CLIENT_CODE: KeyCode = KeyCode { mask: 0, code: 5 };
pub const ADD_WORKSPACE_CODE: KeyCode = KeyCode { mask: 0, code: 6 };
pub const CLIENT_TO_WORKSPACE_CODE: KeyCode = KeyCode { mask: 0, code: 7 };

pub fn example_windowmanager(n_screens: u32, events: Vec<XEvent>) -> ExampleWM {
    let conn = ExampleXConn::new(n_screens, events, vec![]);
    let conf = Config {
        layouts: example_layouts(),
        ..Default::default()
    };
    let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
    wm.init().unwrap();

    wm
}

pub fn example_screens(n: u32) -> Vec<Screen> {
    (0..n)
        .map(|i| Screen::new(Region::new(1080 * n, 800 * n, 1080, 800), i as usize))
        .collect()
}

pub fn example_layouts() -> Vec<Layout> {
    vec![
        Layout::new("first", LayoutConf::default(), row_layout, 1, 0.6),
        Layout::new("second", LayoutConf::default(), row_layout, 1, 0.6),
    ]
}

pub fn row_layout(
    clients: &[&Client],
    _focused: Option<WinId>,
    monitor_region: &Region,
    _max_main: u32,
    _ratio: f32,
) -> Vec<ResizeAction> {
    monitor_region
        .as_rows(clients.len() as u32)
        .iter()
        .zip(clients)
        .map(|(r, c)| (c.id(), Some(*r)))
        .collect()
}

pub fn n_clients(n: u32) -> Vec<XEvent> {
    (0..n)
        .map(|id| XEvent::MapRequest { id, ignore: false })
        .collect()
}

pub fn example_key_bindings() -> ExampleKeyBindings {
    map! {
        EXIT_CODE =>
            Box::new(|wm: &mut ExampleWM| wm.exit()) as ExampleKeyHandler,
        LAYOUT_CHANGE_CODE =>
            Box::new(|wm| wm.cycle_layout(Forward)),
        WORKSPACE_CHANGE_CODE =>
            Box::new(|wm| wm.focus_workspace(&Selector::Index(1))),
        ADD_WORKSPACE_CODE =>
            Box::new(|wm| wm.push_workspace(Workspace::new("new", example_layouts()))),
        SCREEN_CHANGE_CODE =>
            Box::new(|wm| wm.cycle_screen(Forward)),
        FOCUS_CHANGE_CODE =>
            Box::new(|wm| wm.cycle_client(Forward)),
        KILL_CLIENT_CODE =>
            Box::new(|wm| wm.kill_client()),
        CLIENT_TO_WORKSPACE_CODE =>
            Box::new(|wm| wm.client_to_workspace(&Selector::Index(1))),
    }
}

pub fn example_mouse_bindings() -> ExampleMouseBindings {
    map! {}
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ExampleXConn {
    #[cfg_attr(feature = "serde", serde(skip))]
    events: Cell<Vec<XEvent>>,
    focused: Cell<WinId>,
    n_screens: Cell<u32>,
    unmanaged_ids: Vec<WinId>,
}

impl fmt::Debug for ExampleXConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExampleXConn")
            .field("n_screens", &self.n_screens.get())
            .field("remaining_events", &self.remaining_events())
            .field("focused", &self.focused.get())
            .field("unmanaged_ids", &self.unmanaged_ids)
            .finish()
    }
}

impl ExampleXConn {
    /// Set up a new [MockXConn] with pre-defined [Screen]s and an event stream to pull from
    pub fn new(n_screens: u32, events: Vec<XEvent>, unmanaged_ids: Vec<WinId>) -> Self {
        Self {
            events: Cell::new(events),
            focused: Cell::new(0),
            n_screens: Cell::new(n_screens),
            unmanaged_ids,
        }
    }

    pub fn remaining_events(&self) -> Vec<XEvent> {
        let remaining = self.events.replace(vec![]);
        self.events.set(remaining.clone());
        remaining
    }

    pub fn set_screen_count(&mut self, n: u32) {
        self.n_screens.set(n);
    }

    pub fn current_screen_count(&self) -> u32 {
        self.n_screens.get()
    }
}

impl StubXConn for ExampleXConn {
    fn mock_wait_for_event(&self) -> Result<XEvent> {
        let mut remaining = self.events.replace(vec![]);
        if remaining.is_empty() {
            return Ok(XEvent::KeyPress(EXIT_CODE));
        }
        let next = remaining.remove(0);
        self.events.set(remaining);
        Ok(next)
    }

    fn mock_current_outputs(&self) -> Vec<Screen> {
        let num_screens = self.n_screens.get();
        (0..(num_screens))
            .map(|n| Screen::new(Region::new(800 * n, 600 * n, 800, 600), n as usize))
            .collect()
    }

    fn mock_focused_client(&self) -> WinId {
        self.focused.get()
    }

    fn mock_focus_client(&self, id: WinId) {
        self.focused.replace(id);
    }

    fn mock_is_managed_window(&self, id: WinId) -> bool {
        !self.unmanaged_ids.contains(&id)
    }
}
