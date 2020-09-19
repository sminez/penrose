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
            }) as $crate::bindings::FireAndForget
        }
    };
);

/// kick off an internal method on the window manager as part of a key binding
#[macro_export]
macro_rules! run_internal(
    ($func:ident) => {
        Box::new(|wm: &mut $crate::manager::WindowManager| {
            wm.$func();
        }) as $crate::bindings::FireAndForget
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |wm: &mut $crate::manager::WindowManager| {
            wm.$func($($arg),+);
        }) as $crate::bindings::FireAndForget
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
    // parse a single simple key binding
    {   @parse $map:expr, $codes:expr,
        $binding:expr => $action:expr;
        $($tail:tt)*
    } => {
        match $crate::helpers::parse_key_binding($binding, &$codes) {
            None => panic!("invalid key binding: {}", $binding),
            Some(key_code) => $map.insert(key_code, $action),
        };
        gen_keybindings!(@parse $map, $codes, $($tail)*);
    };

    // parse a map block for WindowManager methods with a single arg
    {   @parse $map:expr, $codes:expr,
        map [ $from:expr ] in { $($patt:expr => $method:ident [ $to:expr ];)+ };
        $($tail:tt)*
    } => {
        {
            $(
                for (k, arg) in $from.into_iter().zip($to.clone()) {
                    let binding = format!($patt, k);
                    match $crate::helpers::parse_key_binding(binding.clone(), &$codes) {
                        None => panic!("invalid key binding: {}", binding),
                        Some(key_code) => $map.insert(key_code, run_internal!($method, arg)),
                    };
                }
            )+
            gen__keybindings!(@parse $map, $codes, $($tail)*);
        }
    };

    // parse a map by reference block for WindowManager methods with a single ref arg
    {   @parse $map:expr, $codes:expr,
        refmap [ $from:expr ] in { $($patt:expr => $method:ident [ $to:expr ];)+ };
        $($tail:tt)*
    } => {
        {
            $(
                for (k, arg) in $from.into_iter().zip($to.clone()) {
                    let binding = format!($patt, k);
                    match $crate::helpers::parse_key_binding(binding.clone(), &$codes) {
                        None => panic!("invalid key binding: {}", binding),
                        Some(key_code) => $map.insert(key_code, run_internal!($method, &arg)),
                    };
                }
            )+
            gen_keybindings!(@parse $map, $codes, $($tail)*);
        }
    };

    // base case (out of tokens)
    {   @parse $map:expr, $codes:expr,
        $($tail:tt)*
    } => {
        // shouldn't have any tokens here!
        $(compile_error!(
            stringify!(unexpected tokens in keybinging macro: $tail)
        );)*
    };

    // TODO: remove this depricated method of doing keybindings
    {   $($binding:expr => $action:expr),+;
        $(forall_workspaces: $ws_array:expr => { $($ws_binding:expr => $ws_action:tt),+, })+
    } => {
        gen_keybindings_depricated!(
            $($binding => $action),+;
            $(forall_workspaces: $ws_array => { $($ws_binding => $ws_action),+, })+
        );
    };

    // NOTE: this is the public entry point to the macro
    { $($tokens:tt)+ } => {
        {
            let mut map = ::std::collections::HashMap::new();
            let codes = $crate::helpers::keycodes_from_xmodmap();
            gen_keybindings!(@parse map, codes, $($tokens)+);
            map
        }
    };
);

/// depricated: please use [gen_keybindings] as shown in the examples.
#[deprecated(
    since = "0.0.11",
    note = "This macro will be removed entirely in an upcoming release."
)]
#[macro_export]
macro_rules! gen_keybindings_depricated(
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
                                &$crate::core::ring::Selector::Index(i)
                            )
                        ),
                    };
                )+
            })+

            _map
        }
    };
);

/// make creating all of the mouse bindings less verbose
#[macro_export]
macro_rules! gen_mousebindings(
    {
        $($kind:ident $button:ident + [$($modifier:ident),+] => $action:expr),+
    } => {
        {
            // HashMap<(MouseEventKind, MouseState), MouseEventHandler>
            let mut _map = ::std::collections::HashMap::new();

            $(
                let mut modifiers = Vec::new();
                $(modifiers.push($crate::core::bindings::ModifierKey::$modifier);)+

                let state = $crate::core::bindings::MouseState::new(
                    $crate::core::bindings::MouseButton::$button,
                    modifiers
                );

                let kind = $crate::core::bindings::MouseEventKind::$kind;
                _map.insert(
                    (kind, state),
                    Box::new($action) as $crate::bindings::MouseEventHandler
                );
            )+

            _map
        }
    };
);
