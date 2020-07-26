use penrose::client::Client;
use penrose::data_types::WinId;
use penrose::hooks::*;
use penrose::xconnection::{MockXConn, XEvent};
use penrose::{Config, WindowManager};
use std::cell::RefCell;
use std::rc::Rc;

mod common;

macro_rules! hook_test(
    (
        expected_calls => $n:expr,
        $trait:ident,
        $structname:ident,
        $name:ident,
        $testname:ident,
        $evts:expr,
        $($t:ty),+
    ) => {
        struct $structname {
            name: &'static str,
            calls: Rc<RefCell<Vec<String>>>,
        }

        impl $trait for $structname {
            fn call(&mut self, _: &mut WindowManager, $(_: $t),+) {
                self.calls.replace_with(|cs| {
                    cs.push(format!("{}", self.name));
                    cs.to_vec()
                });
            }
        }

        #[test]
        fn $testname() {
            let calls = Rc::new(RefCell::new(vec![]));

            let hook_1 = $structname {
                name: "hook_1",
                calls: Rc::clone(&calls),
            };
            let hook_2 = $structname {
                name: "hook_2",
                calls: Rc::clone(&calls),
            };

            let mut config = Config::default();
            config.$name.push(Box::new(hook_1));
            config.$name.push(Box::new(hook_2));

            let mut events = $evts.clone();
            events.push(XEvent::KeyPress { code: common::EXIT_CODE });

            let conn = MockXConn::new(
                vec![common::simple_screen(0), common::simple_screen(1)],
                events
            );
            let mut wm = WindowManager::init(config, &conn);
            wm.grab_keys_and_run(common::exit_bindings());
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
    NewClientHook,
    TestNewClientHook,
    new_client_hooks,
    test_new_client_hooks,
    vec![XEvent::Map {
        id: 1,
        ignore: false
    }],
    &mut Client
);

hook_test!(
    expected_calls => 2, // Initial layout application and then due to the change
    LayoutChangeHook,
    TestLayoutHook,
    layout_hooks,
    test_layout_hooks,
    vec![XEvent::KeyPress {
        code: common::LAYOUT_CHANGE_CODE
    }],
    usize,
    usize
);

hook_test!(
    expected_calls => 1,
    WorkspaceChangeHook,
    TestWorkspaceChangeHook,
    workspace_change_hooks,
    test_workspace_hooks,
    vec![XEvent::KeyPress {
        code: common::WORKSPACE_CHANGE_CODE
    }],
    usize,
    usize
);

hook_test!(
    expected_calls => 1,
    ScreenChangeHook,
    TestScreenChangeHook,
    screen_change_hooks,
    test_screen_change_hooks,
    vec![XEvent::KeyPress {
        code: common::SCREEN_CHANGE_CODE
    }],
    usize
);

hook_test!(
    expected_calls => 3, // For each client and then the explicit change
    FocusChangeHook,
    TestFocusChangeHook,
    focus_hooks,
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
    ],
    WinId
);
