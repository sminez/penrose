// Check that each Hook variant is called at the expected points
use penrose::core::{
    client::Client,
    config::Config,
    data_types::WinId,
    hooks::{Hook, Hooks},
    manager::WindowManager,
    xconnection::{MockXConn, XConn, XEvent},
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod common;

struct TestHook {
    name: &'static str,
    method: &'static str,
    calls: Rc<RefCell<Vec<String>>>,
}
impl TestHook {
    fn mark_called(&mut self, method: &str) {
        self.calls.replace_with(|cs| {
            if method == self.method {
                cs.push(format!("{}:{}", self.name, method));
            }
            cs.to_vec()
        });
    }
}

impl<X: XConn> Hook<X> for TestHook {
    fn new_client(&mut self, _: &mut WindowManager<X>, _: &mut Client) {
        self.mark_called("new_client");
    }

    fn remove_client(&mut self, _: &mut WindowManager<X>, _: WinId) {
        self.mark_called("remove_client");
    }

    fn client_name_updated(&mut self, _: &mut WindowManager<X>, _: WinId, _: &str, _: bool) {
        self.mark_called("client_name_updated");
    }

    fn client_added_to_workspace(&mut self, _: &mut WindowManager<X>, _: WinId, _: usize) {
        self.mark_called("client_added_to_workspace");
    }

    fn layout_applied(&mut self, _: &mut WindowManager<X>, _: usize, _: usize) {
        self.mark_called("layout_applied");
    }

    fn layout_change(&mut self, _: &mut WindowManager<X>, _: usize, _: usize) {
        self.mark_called("layout_change");
    }

    fn workspace_change(&mut self, _: &mut WindowManager<X>, _: usize, _: usize) {
        self.mark_called("workspace_change");
    }

    fn workspaces_updated(&mut self, _: &mut WindowManager<X>, _: &[&str], _: usize) {
        self.mark_called("workspaces_updated");
    }

    fn screen_change(&mut self, _: &mut WindowManager<X>, _: usize) {
        self.mark_called("screen_change");
    }

    fn focus_change(&mut self, _: &mut WindowManager<X>, _: WinId) {
        self.mark_called("focus_change");
    }

    fn startup(&mut self, _: &mut WindowManager<X>) {
        self.mark_called("startup");
    }

    fn event_handled(&mut self, _: &mut WindowManager<X>) {
        self.mark_called("event_handled");
    }
}

macro_rules! hook_test {
    (expected_calls => $n:expr, $method: expr, $testname:ident, $evts:expr) => {
        #[test]
        fn $testname() {
            let calls = Rc::new(RefCell::new(vec![]));

            let hook_1 = TestHook {
                name: "hook_1",
                method: $method,
                calls: Rc::clone(&calls),
            };

            let hook_2 = TestHook {
                name: "hook_2",
                method: $method,
                calls: Rc::clone(&calls),
            };

            let config = Config::default();
            let hooks: Hooks<MockXConn> = vec![Box::new(hook_1), Box::new(hook_2)];

            let mut events = $evts.clone();
            events.push(XEvent::KeyPress(common::EXIT_CODE));

            let conn = MockXConn::new(
                vec![common::simple_screen(0), common::simple_screen(1)],
                events,
                vec![],
            );
            let mut wm = WindowManager::new(config, conn, hooks);
            wm.init();
            wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
            drop(wm);

            assert_eq!(
                Rc::try_unwrap(calls).unwrap().into_inner(),
                [
                    format!("{}:{}", "hook_1", $method).as_str(),
                    format!("{}:{}", "hook_2", $method).as_str()
                ]
                .repeat($n)
            );
        }
    };
}

hook_test!(
    expected_calls => 1,
    "new_client",
    test_new_client_hooks,
    vec![XEvent::MapRequest {
        id: 1,
        ignore: false
    }]
);

hook_test!(
    expected_calls => 1,
    "remove_client",
    test_remove_client_hooks,
    vec![
        XEvent::MapRequest {
            id: 1,
            ignore: false
        },
        XEvent::KeyPress(common::KILL_CLIENT_CODE)
    ]
);

hook_test!(
    expected_calls => 2,
    "client_name_updated",
    test_client_name_update_hooks,
    vec![
        XEvent::PropertyNotify {
            id: 1,
            atom: "WM_NAME".to_string(),
            is_root: false
        },
        XEvent::PropertyNotify {
            id: 1,
            atom: "_NET_WM_NAME".to_string(),
            is_root: false
        },
    ]
);

hook_test!(
    expected_calls => 2,  // Initial mapping and then on the move to WS1
    "client_added_to_workspace",
    test_client_to_workspace_hooks,
    vec![
        XEvent::MapRequest { id: 1, ignore: false },
        XEvent::KeyPress(common::CLIENT_TO_WORKSPACE_CODE)
    ]
);

hook_test!(
    expected_calls => 3, // Initial layout application for each screen and then due to the change
    "layout_applied",
    test_layout_applied_hooks,
    vec![XEvent::KeyPress(common::LAYOUT_CHANGE_CODE)]
);

hook_test!(
    expected_calls => 1,
    "layout_change",
    test_layout_change_hooks,
    vec![XEvent::KeyPress(common::LAYOUT_CHANGE_CODE)]
);

hook_test!(
    expected_calls => 1,
    "workspace_change",
    test_workspace_change_hooks,
    vec![XEvent::KeyPress(common::WORKSPACE_CHANGE_CODE)]
);

hook_test!(
    expected_calls => 1,
    "workspaces_updated",
    test_workspace_update_hooks,
    vec![XEvent::KeyPress(common::ADD_WORKSPACE_CODE)]
);

hook_test!(
    expected_calls => 1,
    "screen_change",
    test_screen_change_hooks,
    vec![XEvent::KeyPress(common::SCREEN_CHANGE_CODE)]
);

hook_test!(
    expected_calls => 3, // For each client and then the explicit change
    "focus_change",
    test_focus_hooks,
    vec![
        XEvent::MapRequest {
            id: 1,
            ignore: false
        },
        XEvent::MapRequest {
            id: 2,
            ignore: false
        },
        XEvent::KeyPress(common::FOCUS_CHANGE_CODE)]
);

hook_test!(
    expected_calls => 1,
    "startup",
    test_startup_hooks,
    vec![]
);

hook_test!(
    expected_calls => 2, // one from startup events, and the second from screen change
    "event_handled",
    test_event_handled_hooks,
    vec![XEvent::ScreenChange]
);
