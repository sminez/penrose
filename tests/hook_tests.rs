use penrose::client::Client;
use penrose::data_types::WinId;
use penrose::hooks::Hook;
use penrose::xconnection::{MockXConn, XEvent};
use penrose::{Config, WindowManager};
use std::cell::RefCell;
use std::rc::Rc;

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
                cs.push(self.name.into());
            }
            cs.to_vec()
        });
    }
}

impl Hook for TestHook {
    fn new_client(&mut self, _: &mut WindowManager, c: Client) -> Option<Client> {
        self.mark_called("new_client");
        Some(c)
    }

    fn remove_client(&mut self, _: &mut WindowManager, _: WinId) {
        self.mark_called("remove_client");
    }

    fn layout_change(&mut self, _: &mut WindowManager, _: usize, _: usize) {
        self.mark_called("layout_change");
    }

    fn workspace_change(&mut self, _: &mut WindowManager, _: usize, _: usize) {
        self.mark_called("workspace_change");
    }

    fn screen_change(&mut self, _: &mut WindowManager, _: usize) {
        self.mark_called("screen_change");
    }

    fn focus_change(&mut self, _: &mut WindowManager, _: WinId) {
        self.mark_called("focus_change");
    }
}

macro_rules! hook_test(
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

            let mut config = Config::default();
            config.hooks.push(Box::new(hook_1));
            config.hooks.push(Box::new(hook_2));

            let mut events = $evts.clone();
            events.push(XEvent::KeyPress { code: common::EXIT_CODE });

            let conn = MockXConn::new(
                vec![common::simple_screen(0), common::simple_screen(1)],
                events
            );
            let mut wm = WindowManager::init(config, &conn);
            wm.grab_keys_and_run(common::test_bindings());
            drop(wm);

            assert_eq!(
                Rc::try_unwrap(calls).unwrap().into_inner(),
                ["hook_1", "hook_2"].repeat($n)
            );
        }
    };
);

hook_test!(
    expected_calls => 1,
    "new_client",
    test_new_client_hooks,
    vec![XEvent::Map {
        id: 1,
        ignore: false
    }]
);

hook_test!(
    expected_calls => 1,
    "remove_client",
    test_remove_client_hooks,
    vec![
        XEvent::Map {
            id: 1,
            ignore: false
        },
        XEvent::KeyPress {
            code: common::KILL_CLIENT_CODE
        }
    ]
);

hook_test!(
    expected_calls => 2, // Initial layout application and then due to the change
    "layout_change",
    test_layout_hooks,
    vec![XEvent::KeyPress {
        code: common::LAYOUT_CHANGE_CODE
    }]
);

hook_test!(
    expected_calls => 1,
    "workspace_change",
    test_workspace_hooks,
    vec![XEvent::KeyPress {
        code: common::WORKSPACE_CHANGE_CODE
    }]
);

hook_test!(
    expected_calls => 1,
    "screen_change",
    test_screen_change_hooks,
    vec![XEvent::KeyPress {
        code: common::SCREEN_CHANGE_CODE
    }]
);

hook_test!(
    expected_calls => 3, // For each client and then the explicit change
    "focus_change",
    test_focus_hooks,
    vec![
        XEvent::Map {
            id: 1,
            ignore: false
        },
        XEvent::Map {
            id: 2,
            ignore: false
        },
        XEvent::KeyPress {
            code: common::FOCUS_CHANGE_CODE
        }
    ]
);
