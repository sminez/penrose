// Check that restoring from serialised state is working
#[macro_use]
extern crate penrose;

use penrose::{
    core::{
        config::Config,
        data_types::WinId,
        layout::{floating, side_stack, LayoutFunc},
        manager::WindowManager,
        screen::Screen,
        xconnection::{MockXConn, StubXConn, XEvent},
    },
    PenroseError,
};

use std::collections::HashMap;

mod common;

struct EarlyExitConn {
    on_init: bool,
    valid_clients: bool,
}
impl StubXConn for EarlyExitConn {
    fn mock_current_outputs(&self) -> Vec<Screen> {
        if self.on_init {
            panic!("panic on call to current_outputs");
        }

        vec![common::simple_screen(0), common::simple_screen(1)]
    }

    fn mock_wait_for_event(&self) -> Option<XEvent> {
        if !self.on_init {
            panic!("panic on call to wait_for_event");
        }

        None
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

fn get_wm() -> WindowManager {
    // Seeding the MockXConn with events so that we should end up with:
    //   - clients 1 on workspace 0
    //   - client 2 & 3 on workspace 1
    //   - focus on client 2
    //   - screen 0 holding workspace 1
    //   - screen 1 holding workspace 0
    let conn = MockXConn::new(
        vec![common::simple_screen(0), common::simple_screen(1)],
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
            XEvent::KeyPress(common::EXIT_CODE),
        ],
        vec![],
    );

    let mut wm = WindowManager::new(Config::default(), Box::new(conn), vec![]);
    wm.init();

    wm
}

#[cfg(feature = "serde")]
#[test]
fn serde_windowmanager_can_be_serialized() {
    let wm = get_wm();
    let as_json = serde_json::to_string(&wm);
    assert!(as_json.is_ok());
}

#[cfg(feature = "serde")]
#[test]
fn serde_windowmanager_can_be_deserialized() {
    let wm = get_wm();
    let as_json = serde_json::to_string(&wm).unwrap();
    let unchecked_wm: Result<WindowManager, serde_json::Error> = serde_json::from_str(&as_json);
    assert!(unchecked_wm.is_ok());
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(
    expected = "StubConn is not usable as a real XConn impl: call hydrate_and_init instead"
)]
fn serde_running_without_hydrating_panics() {
    let wm = get_wm();
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager = serde_json::from_str(&as_json).unwrap();
    unchecked_wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
}

#[cfg(feature = "serde")]
#[test]
fn serde_hydrating_when_x_state_is_wrong_errors() {
    let mut wm = get_wm();
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager = serde_json::from_str(&as_json).unwrap();
    let res = unchecked_wm.hydrate_and_init(
        Box::new(EarlyExitConn {
            on_init: true,
            valid_clients: false,
        }),
        vec![],
        layout_funcs(),
    );

    match res {
        Ok(_) => panic!("this should have caused a errored"),
        Err(e) => match e {
            PenroseError::MissingClientIds(ids) => assert_eq!(&ids, &[1, 2, 3]),
            _ => panic!("unexpected Error type from hydration"),
        },
    }
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(
    expected = "StubConn is not usable as a real XConn impl: call hydrate_and_init instead"
)]
fn serde_running_init_directly_panics() {
    let wm = get_wm();
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager = serde_json::from_str(&as_json).unwrap();
    unchecked_wm.init();
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(expected = "panic on call to current_outputs")]
fn serde_hydrate_and_init_works_with_serialized_state() {
    let mut wm = get_wm();
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager = serde_json::from_str(&as_json).unwrap();

    // Should panic on the call to current_outputs in WindowManager::init()
    unchecked_wm
        .hydrate_and_init(
            Box::new(EarlyExitConn {
                on_init: true,
                valid_clients: true,
            }),
            vec![],
            layout_funcs(),
        )
        .unwrap();
}

#[cfg(feature = "serde")]
#[test]
#[should_panic(expected = "panic on call to wait_for_event")]
fn serde_running_after_hydration_works() {
    let mut wm = get_wm();
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager = serde_json::from_str(&as_json).unwrap();

    unchecked_wm
        .hydrate_and_init(
            Box::new(EarlyExitConn {
                on_init: false,
                valid_clients: true,
            }),
            vec![],
            layout_funcs(),
        )
        .unwrap();

    // Should panic on the call to XConn::wait_for_event()
    unchecked_wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
}
