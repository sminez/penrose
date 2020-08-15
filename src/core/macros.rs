//! Utility macros for use in the rest of penrose.
//! Not intended for general use

/// kick off an external program as part of a key/mouse binding.
/// explicitly redirects stderr to /dev/null
#[macro_export]
macro_rules! run_external(
    ($cmd:tt) => {
        {
            Box::new(move |_: &mut $crate::manager::WindowManager| {
                $crate::helpers::spawn($cmd);
            }) as $crate::data_types::FireAndForget
        }
    };
);

/// kick off an internal method on the window manager as part of a key/mouse binding
#[macro_export]
macro_rules! run_internal(
    ($func:ident) => {
        Box::new(|wm: &mut $crate::manager::WindowManager| {
            wm.$func();
        }) as $crate::data_types::FireAndForget
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |wm: &mut $crate::manager::WindowManager| {
            wm.$func($($arg),+);
        }) as $crate::data_types::FireAndForget
    };
);

/// make creating a hash-map a little less verbose
#[macro_export]
macro_rules! map(
    {} => { ::std::collections::HashMap::new(); };

    { $($key:expr => $value:expr),+, } => {
        {
            let mut _map = ::std::collections::HashMap::new();
            $(_map.insert($key, $value);)+
            _map
        }
    };
);

/// make creating all of the key bindings less verbose
#[macro_export]
macro_rules! gen_keybindings(
    {
        $($binding:expr => $action:expr),+;
        $(forall_workspaces: $ws_array:expr => { $($ws_binding:expr => $ws_action:tt),+, })+
    } => {
        {
            let mut _map = ::std::collections::HashMap::new();
            let keycodes = $crate::helpers::keycodes_from_xmodmap();

            $(
                match $crate::helpers::parse_key_binding($binding, &keycodes) {
                    None => panic!("invalid key binding: {}", $binding),
                    Some(key_code) => _map.insert(key_code, $action),
                };
            )+

            $(for i in 0..$ws_array.len() {
                $(
                    let for_ws = format!($ws_binding, i+1);
                    match $crate::helpers::parse_key_binding(for_ws.clone(), &keycodes) {
                        None => panic!("invalid key binding: {}", for_ws),
                        Some(key_code) => _map.insert(
                            key_code,
                            run_internal!(
                                $ws_action,
                                &$crate::data_types::Selector::Index(i)
                            )
                        ),
                    };
                )+
            })+

            _map
        }
    };
);
