// Check that each Hook variant is called at the expected points
#[macro_use]
extern crate penrose;

use penrose::{
    core::{
        client::Client,
        config::Config,
        data_types::Region,
        hooks::{Hook, Hooks},
        manager::WindowManager,
        screen::Screen,
        xconnection::{Atom, Prop, XConn, XEvent, Xid},
    },
    logging_error_handler, PenroseError, Result,
};

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

mod common;

pub struct TestXConn {
    screens: Vec<Screen>,
    events: Cell<Vec<XEvent>>,
    focused: Cell<Xid>,
    unmanaged_ids: Vec<Xid>,
}

impl TestXConn {
    /// Set up a new [MockXConn] with pre-defined [Screen]s and an event stream to pull from
    pub fn new(screens: Vec<Screen>, events: Vec<XEvent>, unmanaged_ids: Vec<Xid>) -> Self {
        Self {
            screens,
            events: Cell::new(events),
            focused: Cell::new(0),
            unmanaged_ids,
        }
    }
}

__impl_stub_xcon! {
    for TestXConn;

    client_properties: {
        fn mock_get_prop(&self, _id: Xid, name: &str) -> Result<Prop> {
            if name == Atom::NetWmName.as_ref() {
                Ok(Prop::UTF8String(vec!["mock name".into()]))
            } else {
                Err(PenroseError::Raw("mocked".into()))
            }
        }
    }
    client_handler: {
        fn mock_focus_client(&self, id: Xid) -> Result<()> {
            self.focused.replace(id);
            Ok(())
        }
    }
    client_config: {}
    event_handler: {
        fn mock_wait_for_event(&self) -> Result<XEvent> {
            let mut remaining = self.events.replace(vec![]);
            if remaining.is_empty() {
                return Err(PenroseError::Raw("mock conn closed".into()));
            }
            let next = remaining.remove(0);
            self.events.set(remaining);
            Ok(next)
        }
    }
    state: {
        fn mock_current_screens(&self) -> Result<Vec<Screen>> {
            Ok(self.screens.clone())
        }

        fn mock_focused_client(&self) -> Result<Xid> {
            Ok(self.focused.get())
        }
    }
    conn: {
        fn mock_is_managed_client(&self, id: Xid) -> bool {
            !self.unmanaged_ids.contains(&id)
        }
    }
}

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
        impl<X> Hook<X> for TestHook
        where
            X: XConn,
        {
            $(fn $name(&mut self, _: &mut WindowManager<X>, $(_: $t),*) -> Result<()> {
                self.mark_called(stringify!($name));
                Ok(())
            })+
        }
    }
}

__impl_test_hook! {
    client_name_updated => Xid, &str, bool;
    client_added_to_workspace => Xid, usize;
    event_handled => ;
    focus_change => Xid;
    layout_applied => usize, usize;
    layout_change => usize, usize;
    new_client => &mut Client;
    randr_notify => ;
    remove_client => Xid;
    screen_change => usize;
    screens_updated => &[Region];
    startup => ;
    workspace_change => usize, usize;
    workspaces_updated => &[&str], usize;
}

test_cases! {
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
        let hooks: Hooks<TestXConn> = vec![Box::new(TestHook {
            method,
            calls: Rc::clone(&calls),
        })];

        let mut events = events;
        events.push(XEvent::KeyPress(common::EXIT_CODE));

        let screens = vec![common::simple_screen(0), common::simple_screen(1)];
        let conn = TestXConn::new(screens, events, vec![]);
        let mut wm = WindowManager::new(Config::default(), conn, hooks, logging_error_handler());

        wm.init().unwrap();
        wm.grab_keys_and_run(common::test_bindings(), HashMap::new()).unwrap();
        drop(wm);

        let actual_calls = Rc::try_unwrap(calls).unwrap().into_inner();
        assert_eq!(actual_calls, [method].repeat(n_calls));
    }
}
