//! Utility macros for use in the rest of penrose.
//! Not intended for general use

/// kick off an external program as part of a key/mouse binding.
/// explicitly redirects stderr to /dev/null
#[macro_export]
macro_rules! run_external {
    ($cmd:tt) => {{
        Box::new(move |_: &mut $crate::core::manager::WindowManager<_>| {
            $crate::core::helpers::spawn($cmd)
        }) as $crate::core::bindings::KeyEventHandler<_>
    }};
}

/// kick off an internal method on the window manager as part of a key binding
#[macro_export]
macro_rules! run_internal {
    ($func:ident) => {
        Box::new(|wm: &mut $crate::core::manager::WindowManager<_>| {
            wm.$func()
        }) as $crate::core::bindings::KeyEventHandler<_>
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |wm: &mut $crate::core::manager::WindowManager<_>| {
            wm.$func($($arg),+)
        }) as $crate::core::bindings::KeyEventHandler<_>
    };
}

/// make creating a hash-map a little less verbose
#[macro_export]
macro_rules! map {
    {} => { ::std::collections::HashMap::new() };

    { $($key:expr => $value:expr),+, } => {
        {
            let mut _map = ::std::collections::HashMap::new();
            $(_map.insert($key, $value);)+
            _map
        }
    };
}

/// make creating all of the key bindings less verbose
#[macro_export]
macro_rules! gen_keybindings {
    // parse a single simple key binding
    {   @parse $map:expr, $codes:expr,
        $binding:expr => $action:expr;
        $($tail:tt)*
    } => {
        match $crate::xcb::helpers::parse_key_binding($binding, &$codes) {
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
                    match $crate::core::helpers::parse_key_binding(binding.clone(), &$codes) {
                        None => panic!("invalid key binding: {}", binding),
                        Some(key_code) => $map.insert(key_code, run_internal!($method, arg)),
                    };
                }
            )+
            gen_keybindings!(@parse $map, $codes, $($tail)*);
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
                    match $crate::xcb::helpers::parse_key_binding(binding.clone(), &$codes) {
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

    // NOTE: this is the public entry point to the macro
    { $($tokens:tt)+ } => {
        {
            let mut map = ::std::collections::HashMap::new();
            let codes = $crate::core::helpers::keycodes_from_xmodmap();
            gen_keybindings!(@parse map, codes, $($tokens)+);
            map
        }
    };
}

/// make creating all of the mouse bindings less verbose
#[macro_export]
macro_rules! gen_mousebindings {
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
                    Box::new($action) as $crate::core::bindings::MouseEventHandler<_>
                );
            )+

            _map
        }
    };
}

// Helper for converting Vec<String> -> &[&str]
macro_rules! str_slice {
    ($string_vec:expr) => {
        &$string_vec.iter().map(AsRef::as_ref).collect::<Vec<&str>>();
    };
}

// Auto generate a struct and associated builder struct with getter methods
// on the generated (private) struct fields but no setters.
//
// NOTE: requires that you provide a `validate` method on the builder and
//       some way of getting an initial value of the real struct (i.e. impl
//       Default)
#[doc(hidden)]
#[macro_export]
macro_rules! __with_builder_and_getters {
    {
        $(#[$struct_outer:meta])*
        $name:ident;
        $(#[$builder_outer:meta])*
        $builder_name:ident;

        $(
            $(#[$field_outer:meta])*
            $(VecImplInto $intofield:ident : $intoty:ty;)?
            $(Concrete $field:ident : $ty:ty;)?
            => $default:expr;
        )+
    } => {
        $(#[$struct_outer])*
        pub struct $name {
            $(
                pub(crate)
                $($intofield : Vec<$intoty>,)?
                $($field: $ty,)?
            )+
        }

        impl $name {
            /// Make a new associated builder struct containing the field values of this struct
            pub fn builder(&self) -> $builder_name {
                $builder_name {
                    inner: self.clone(),
                }
            }

            $(
                /// Obtain a reference to
                $(#[$field_outer])*
                $(pub fn $intofield(&self) -> &Vec<$intoty> {
                        &self.$intofield
                })?
                $(pub fn $field(&self) -> &$ty {
                        &self.$field
                })?
            )+
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $($intofield: $default,)?
                        $($field: $default,)?
                    )+
                }
            }
        }

        $(#[$builder_outer])*
        pub struct $builder_name {
            inner: $name,
        }

        impl $builder_name {
            /// Validate and build the underlying struct
            pub fn build(&self) -> std::result::Result<$name, String> {
                self.validate()?;
                Ok(self.inner.clone())
            }

            $(
                /// Set the value of
                $(#[$field_outer])*
                $(pub fn $intofield(&mut self, val: impl IntoIterator<Item = impl Into<$intoty>>) -> &mut $builder_name {
                    self.inner.$intofield = val.into_iter().map(|elem| elem.into()).collect();
                    self
                })?
                $(pub fn $field(&mut self, val: $ty) -> &mut $builder_name {
                    self.inner.$field = val;
                    self
                })?
            )+
        }
    }
}

/// Helper for writing paramaterised test cases
#[doc(hidden)]
#[macro_export]
macro_rules! test_cases {
    {
        $test_name:ident;
        args: ( $($arg:ident: $t:ty),* $(,)? );

        $(
            case: $case_name:ident => ( $($param:expr),* );
        )+
        body: $body:expr
    } => {
        paste::paste! {
            fn [<$test_name _helper>]($($arg: $t),*) {
                $body
            }
        }

        $(
            paste::paste! {
                #[test]
                fn [<$test_name _ $case_name>]() {
                    [<$test_name _helper>]($($param),*)
                }
            }
        )+
    };
}
