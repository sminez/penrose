// Check that restoring from serialised state is working
#[macro_use]
extern crate penrose;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

use penrose::{
    core::{
        config::Config,
        data_types::WinId,
        layout::{floating, side_stack, LayoutFunc},
        manager::WindowManager,
        screen::Screen,
        xconnection::{StubXConn, XEvent},
    },
    PenroseError,
};

use std::{cell::Cell, collections::HashMap};

mod common;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct EarlyExitConn {
    valid_clients: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    events: Cell<Vec<XEvent>>,
}

impl EarlyExitConn {
    fn new(valid_clients: bool, events: Vec<XEvent>) -> Self {
        Self {
            valid_clients,
            events: Cell::new(events),
        }
    }
}

impl StubXConn for EarlyExitConn {
    fn mock_current_outputs(&self) -> Vec<Screen> {
        vec![common::simple_screen(0), common::simple_screen(1)]
    }

    fn mock_wait_for_event(&self) -> Option<XEvent> {
        let mut remaining = self.events.replace(vec![]);
        if remaining.is_empty() {
            return Some(XEvent::KeyPress(common::EXIT_CODE));
        }
        let next = remaining.remove(0);
        self.events.set(remaining);
        Some(next)
    }

    fn mock_query_for_active_windows(&self) -> Vec<WinId> {
        if self.valid_clients {
            vec![1, 2, 3]
        } else {
            vec![]
        }
    }
}

fn layout_funcs() -> HashMap<&'static str, LayoutFunc> {
    map! {
        "[side]" => side_stack as LayoutFunc,
        "[----]" => floating as LayoutFunc,
    }
}

fn get_seeded_wm(valid_clients: bool) -> WindowManager<EarlyExitConn> {
    // Seeding the MockXConn with events so that we should end up with:
    //   - clients 1 on workspace 0
    //   - client 2 & 3 on workspace 1
    //   - focus on client 2
    //   - screen 0 holding workspace 1
    //   - screen 1 holding workspace 0
    let conn = EarlyExitConn::new(
        valid_clients,
        vec![
            XEvent::MapRequest {
                id: 1,
                ignore: false,
            },
            XEvent::KeyPress(common::WORKSPACE_CHANGE_CODE),
            XEvent::MapRequest {
                id: 2,
                ignore: false,
            },
            XEvent::MapRequest {
                id: 3,
                ignore: false,
            },
            XEvent::KeyPress(common::FOCUS_CHANGE_CODE),
        ],
    );

    let mut wm = WindowManager::new(Config::default(), conn, vec![]);
    wm.init();

    wm
}

#[cfg(feature = "serde")]
#[test]
fn serde_windowmanager_can_be_serialized() {
    let wm = get_seeded_wm(true);
    let as_json = serde_json::to_string(&wm);
    assert!(as_json.is_ok());
}

#[cfg(feature = "serde")]
#[test]
fn serde_windowmanager_can_be_deserialized() {
    let wm = get_seeded_wm(true);
    let as_json = serde_json::to_string(&wm).unwrap();
    let unchecked_wm: Result<WindowManager<EarlyExitConn>, serde_json::Error> =
        serde_json::from_str(&as_json);
    assert!(unchecked_wm.is_ok());
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(
    expected = "'hydrate_and_init' must be called before 'grab_keys_and_run' when restoring from serialised state"
)]
fn serde_running_without_hydrating_panics() {
    let wm = get_seeded_wm(true);
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    // Should panic due to self.hydrated being false
    unchecked_wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
}

#[cfg(feature = "serde")]
#[test]
fn serde_hydrating_when_x_state_is_wrong_errors() {
    let mut wm = get_seeded_wm(false);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();
    let res = unchecked_wm.hydrate_and_init(vec![], layout_funcs());

    match res {
        Ok(_) => panic!("this should have returned an error"),
        Err(e) => match e {
            PenroseError::MissingClientIds(ids) => assert_eq!(&ids, &[1, 2, 3]),
            _ => panic!("unexpected Error type from hydration"),
        },
    }
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(expected = "Need to call 'hydrate_and_init' when restoring from serialised state")]
fn serde_running_init_directly_panics() {
    let wm = get_seeded_wm(true);
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    // Should panic due to self.hydrated being false
    unchecked_wm.init();
}

#[cfg(feature = "serde")]
#[test]
fn serde_hydrate_and_init_works_with_serialized_state() {
    let mut wm = get_seeded_wm(true);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    let res = unchecked_wm.hydrate_and_init(vec![], layout_funcs());
    assert!(res.is_ok());
}

#[cfg(feature = "serde")]
#[test]
fn serde_running_after_hydration_works() {
    let mut wm = get_seeded_wm(true);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    unchecked_wm
        .hydrate_and_init(vec![], layout_funcs())
        .unwrap();

    unchecked_wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
}
