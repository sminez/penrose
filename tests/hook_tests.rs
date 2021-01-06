// Check that each Hook variant is called at the expected points
use penrose::core::{
    client::Client,
    config::Config,
    data_types::{Region, WinId},
    hooks::{Hook, Hooks},
    manager::WindowManager,
    xconnection::{MockXConn, XConn, XEvent},
};

use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod common;

struct TestHook {
    method: &'static str,
    calls: Rc<RefCell<Vec<String>>>,
}

impl TestHook {
    fn mark_called(&mut self, method: &str) {
        self.calls.replace_with(|cs| {
            if method == self.method {
                cs.push(method.to_string());
            }
            cs.to_vec()
        });
    }
}

// Helper for stubbing out Hook trait methods so that we can trace calls
macro_rules! __impl_test_hook {
    { $($name:ident => $($t:ty),*;)+ } => {
        impl<X: XConn> Hook<X> for TestHook {
            $(fn $name(&mut self, _: &mut WindowManager<X>, $(_: $t),*) {
                self.mark_called(stringify!($name));
            })+
        }
    }
}

__impl_test_hook! {
    client_name_updated => WinId,&str, bool;
    client_added_to_workspace => WinId, usize;
    event_handled => ;
    focus_change => WinId;
    layout_applied => usize, usize;
    layout_change => usize, usize;
    new_client => &mut Client;
    randr_notify => ;
    remove_client => WinId;
    screen_change => usize;
    screens_updated => &[Region];
    startup => ;
    workspace_change => usize, usize;
    workspaces_updated => &[&str], usize;
}

penrose::test_cases! {
    hook_triggers;
    args: (method: &'static str, n_calls: usize, events: Vec<XEvent>);

    case: client_name_updated => ("client_name_updated", 2, vec![
        XEvent::PropertyNotify { id: 1, atom: "WM_NAME".into(), is_root: false },
        XEvent::PropertyNotify { id: 1, atom: "_NET_WM_NAME".into(), is_root: false },
    ]);
    case: client_added_to_workspace => ("client_added_to_workspace", 2, vec![
        XEvent::MapRequest { id: 1, ignore: false },
        XEvent::KeyPress(common::CLIENT_TO_WORKSPACE_CODE)
    ]);
    case: event_handled => ("event_handled", 2, vec![XEvent::ScreenChange]);
    case: focus_change => ("focus_change", 3, vec![
        XEvent::MapRequest { id: 1, ignore: false },
        XEvent::MapRequest { id: 2, ignore: false },
        XEvent::KeyPress(common::FOCUS_CHANGE_CODE)
    ]);
    case: layout_applied => ("layout_applied", 3, vec![XEvent::KeyPress(common::LAYOUT_CHANGE_CODE)]);
    case: layout_change => ("layout_change", 1, vec![XEvent::KeyPress(common::LAYOUT_CHANGE_CODE)]);
    case: new_client => ("new_client", 1, vec![XEvent::MapRequest { id: 1, ignore: false}]);
    case: randr_notify => ("randr_notify", 1, vec![XEvent::RandrNotify]);
    case: remove_client => ("remove_client", 1, vec![
        XEvent::MapRequest { id: 1, ignore: false},
        XEvent::KeyPress(common::KILL_CLIENT_CODE)
    ]);
    case: screen_change => ("screen_change", 1, vec![XEvent::KeyPress(common::SCREEN_CHANGE_CODE)]);
    case: screens_updated => ("screens_updated", 1, vec![XEvent::RandrNotify]);
    case: startup => ("startup", 1, vec![]);
    case: workspace_change => ("workspace_change", 1, vec![XEvent::KeyPress(common::WORKSPACE_CHANGE_CODE)]);
    case: workspaces_updated => ("workspaces_updated", 1, vec![XEvent::KeyPress(common::ADD_WORKSPACE_CODE)]);

    body: {
        let calls = Rc::new(RefCell::new(vec![]));
        let hooks: Hooks<MockXConn> = vec![Box::new(TestHook {
            method,
            calls: Rc::clone(&calls),
        })];

        let mut events = events;
        events.push(XEvent::KeyPress(common::EXIT_CODE));

        let screens = vec![common::simple_screen(0), common::simple_screen(1)];
        let conn = MockXConn::new(screens, events, vec![]);
        let mut wm = WindowManager::new(Config::default(), conn, hooks);

        wm.init();
        wm.grab_keys_and_run(common::test_bindings(), HashMap::new());
        drop(wm);

        let actual_calls = Rc::try_unwrap(calls).unwrap().into_inner();
        assert_eq!(actual_calls, [method].repeat(n_calls));
    }
}
