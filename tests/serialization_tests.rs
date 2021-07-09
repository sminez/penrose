// Check that restoring from serialised state is working
#[macro_use]
extern crate penrose;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

use penrose::{
    core::{
        client::Client,
        config::Config,
        layout::{floating, side_stack, LayoutFunc},
        manager::WindowManager,
        screen::Screen,
        xconnection::{Atom, Prop, Result, XError, XEvent, Xid},
    },
    logging_error_handler, PenroseError,
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

__impl_stub_xcon! {
    for EarlyExitConn;

    atom_queries: {}
    client_properties: {
        fn mock_get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
            if name == Atom::NetWmName.as_ref() {
                Ok(Prop::UTF8String(vec!["mock name".into()]))
            } else {
                Err(XError::MissingProperty(name.into(), id))
            }
        }
    }
    client_handler: {}
    client_config: {}
    event_handler: {
        fn mock_wait_for_event(&self) -> Result<XEvent> {
            let mut remaining = self.events.replace(vec![]);
            if remaining.is_empty() {
                return Ok(XEvent::KeyPress(common::EXIT_CODE));
            }
            let next = remaining.remove(0);
            self.events.set(remaining);
            Ok(next)
        }
    }
    state: {
        fn mock_current_screens(&self) -> Result<Vec<Screen>> {
            Ok(vec![common::simple_screen(0), common::simple_screen(1)])
        }

        fn mock_active_clients(&self) -> Result<Vec<Xid>> {
            Ok(if self.valid_clients {
                vec![1, 2, 3]
            } else {
                vec![]
            })
        }
    }
    conn: {
        fn mock_is_managed_client(&self, _c: &Client) -> bool {
            true
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
            XEvent::MapRequest(1, false),
            XEvent::KeyPress(common::WORKSPACE_CHANGE_CODE),
            XEvent::MapRequest(2, false),
            XEvent::MapRequest(3, false),
            XEvent::KeyPress(common::FOCUS_CHANGE_CODE),
        ],
    );

    let mut wm = WindowManager::new(Config::default(), conn, vec![], logging_error_handler());
    wm.init().unwrap();

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
    let unchecked_wm: std::result::Result<WindowManager<EarlyExitConn>, serde_json::Error> =
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
    unchecked_wm
        .grab_keys_and_run(common::test_bindings(), HashMap::new())
        .unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn serde_hydrating_when_x_state_is_wrong_errors() {
    let mut wm = get_seeded_wm(false);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new())
        .unwrap();
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();
    let res = unchecked_wm.hydrate_and_init(vec![], logging_error_handler(), layout_funcs());

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
    unchecked_wm.init().unwrap();
}

#[cfg(feature = "serde")]
#[test]
fn serde_hydrate_and_init_works_with_serialized_state() {
    let mut wm = get_seeded_wm(true);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new())
        .unwrap();
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    let res = unchecked_wm.hydrate_and_init(vec![], logging_error_handler(), layout_funcs());
    assert!(res.is_ok());
}

#[cfg(feature = "serde")]
#[test]
fn serde_running_after_hydration_works() {
    let mut wm = get_seeded_wm(true);
    wm.grab_keys_and_run(common::test_bindings(), HashMap::new())
        .unwrap();
    let as_json = serde_json::to_string(&wm).unwrap();
    let mut unchecked_wm: WindowManager<EarlyExitConn> = serde_json::from_str(&as_json).unwrap();

    unchecked_wm
        .hydrate_and_init(vec![], logging_error_handler(), layout_funcs())
        .unwrap();

    unchecked_wm
        .grab_keys_and_run(common::test_bindings(), HashMap::new())
        .unwrap();
}
