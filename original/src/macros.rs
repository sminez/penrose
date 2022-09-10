//! Utility macros for use in the rest of penrose.

/// kick off an external program as part of a key/mouse binding.
///
/// NOTE: this explicitly redirects stderr to /dev/null
///
/// ```no_run
/// # #[macro_use] extern crate penrose;
/// # use penrose::__test_helpers::*;
/// # fn example() -> TestKeyHandler {
/// # Box::new(
/// run_external!("dmenu_run")
/// # )}
/// ```
#[macro_export]
macro_rules! run_external {
    ($cmd:tt) => {{
        Box::new(move |_: &mut $crate::core::manager::WindowManager<_>| {
            $crate::common::helpers::spawn($cmd)
        }) as $crate::common::bindings::KeyEventHandler<_>
    }};
}

/// Kick off an internal method on the window manager as part of a key binding
///
/// ```no_run
/// # #[macro_use] extern crate penrose;
/// # use penrose::__test_helpers::*;
/// # fn example() -> TestKeyHandler {
/// # Box::new(
/// run_internal!(cycle_client, Forward)
/// # )}
/// ```
#[macro_export]
macro_rules! run_internal {
    ($func:ident) => {
        Box::new(|wm: &mut $crate::core::manager::WindowManager<_>| {
            wm.$func()
        }) as $crate::common::bindings::KeyEventHandler<_>
    };

    ($func:ident, $($arg:expr),+) => {
        Box::new(move |wm: &mut $crate::core::manager::WindowManager<_>| {
            wm.$func($($arg),+)
        }) as $crate::common::bindings::KeyEventHandler<_>
    };
}

/// Helper for spawning external processes and ignoring the output
#[macro_export]
macro_rules! spawn {
    { $cmd:expr } => {
        $crate::core::helpers::spawn($cmd)
    };

    { $cmd:expr, $($arg:expr),+ } => {
        $crate::core::helpers::spawn_with_args($cmd, &[$($arg),+])
    };
}

/// Helper for spawning external processes and capturing the output
#[macro_export]
macro_rules! spawn_for_output {
    { $cmd:expr } => {
        $crate::core::helpers::spawn_for_output($cmd).map(|s|
            s.trim().split('\n').map(String::from).collect::<Vec<String>>()
        )
    };

    { $cmd:expr, $($arg:expr),+ } => {
        $crate::core::helpers::spawn_for_output_with_args($cmd, &[$($arg),+]).map(|s|
            s.trim().split('\n').map(String::from).collect::<Vec<String>>()
        )
    };
}

/// Make creating a HashMap a little less verbose
///
/// ```
/// # #[macro_use] extern crate penrose;
/// map! {
///     1 => "one",
///     2 => "two",
///     3 => "three",
/// };
/// ```
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

/// Generate user keybindings with optional compile time validation.
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate penrose;
/// # use penrose::__test_helpers::*;
/// # fn example() -> TestKeyBindings {
/// let key_bindings = gen_keybindings! {
///     "M-semicolon" => run_external!("dmenu_run");
///     "M-Return" => run_external!("alacritty");
///     "XF86AudioMute" => run_external!("amixer set Master toggle");
///     "M-A-Escape" => run_internal!(exit);
///
///     "M-j" => run_internal!(cycle_client, Forward);
///     "M-k" => run_internal!(cycle_client, Backward);
///     "M-S-j" => run_internal!(drag_client, Forward);
///     "M-S-k" => run_internal!(drag_client, Backward);
///     "M-S-q" => run_internal!(kill_client);
///
///     "M-Tab" => run_internal!(toggle_workspace);
///
///     "M-grave" => run_internal!(cycle_layout, Forward);
///     "M-S-grave" => run_internal!(cycle_layout, Backward);
///     "M-A-Up" => run_internal!(update_max_main, More);
///     "M-A-Down" => run_internal!(update_max_main, Less);
///     "M-A-Right" => run_internal!(update_main_ratio, More);
///     "M-A-Left" => run_internal!(update_main_ratio, Less);
///
///     map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
///         "M-{}" => focus_workspace (REF);
///         "M-S-{}" => client_to_workspace (REF);
///     };
/// };
/// # key_bindings }
/// ```
///
/// # Sections
///
/// ### Direct binding
///
/// ```no_run
/// # #[macro_use] extern crate penrose;
/// # use penrose::__test_helpers::*;
/// # fn example() -> TestKeyBindings {
/// # gen_keybindings! {
/// "M-j" => run_internal!(cycle_client, Forward);
/// "M-S-j" => run_internal!(drag_client, Forward);
/// "M-Return" => run_external!("alacritty");
/// # }};
/// ```
///
/// This is what the majority of your keybindings will look like.
///
/// Should be a string literal and an expression that satisfies the [KeyEventHandler][1] type. The
/// [run_internal] and [run_external] helper macros can be used for simplifying bindings that
/// perform common actions like spawning external programs or triggering methods on the
/// [WindowManager][2].
///
/// ### Map block
///
/// Bind a common pattern via a template.
///
/// ```no_run
/// # #[macro_use] extern crate penrose;
/// # use penrose::__test_helpers::*;
/// # fn example() -> TestKeyBindings {
/// # gen_keybindings! {
/// // VAL: values are passed to the method directly
/// map: { "Up", "Down" } to vec![More, Less] => {
///     "M-{}" => update_max_main (VAL);
/// };
///
/// // REF: values are passed to the method as references
/// map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
///     "M-{}" => focus_workspace (REF);
///     "M-S-{}" => client_to_workspace (REF);
/// };
/// # }};
/// ```
///
/// When you have a common pattern for multiple key bindings (such as focusing a given workspace)
/// you can use a `map` block to avoid having to write them all out explicitly. The required format
/// is as follows:
/// ```markdown
/// map: { "str", "literal", "key", "names" } to `impl Iterator` => {
///     "<modifiers>-{}" => `WindowManager method` ( arg, ... );
/// }
/// ```
///
/// Note that the key names _must_ be string literals, not just `&str` references. The arguments to
/// the [WindowManager][2] method can be passed by reference using `REF` or by value using `VAL`.
/// Any additional arguments can be passed explicitly if they are required by the method.
///
/// [1]: crate::core::bindings::KeyEventHandler
/// [2]: crate::core::manager::WindowManager
#[macro_export]
macro_rules! gen_keybindings {
    { $($tokens:tt)* } => {
        {
            let mut map = ::std::collections::HashMap::new();
            let codes = $crate::common::helpers::keycodes_from_xmodmap();
            let parse = $crate::xcb::helpers::parse_key_binding;
            $crate::__private!(@parsekey map, codes, parse, [], [], $($tokens)*);
            map
        }
    };
}

/// Make creating all of the mouse bindings less verbose
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
                $(modifiers.push($crate::common::bindings::ModifierKey::$modifier);)+

                let state = $crate::common::bindings::MouseState::new(
                    $crate::common::bindings::MouseButton::$button,
                    modifiers
                );

                let kind = $crate::common::bindings::MouseEventKind::$kind;
                _map.insert(
                    (kind, state),
                    Box::new($action) as $crate::common::bindings::MouseEventHandler<_>
                );
            )+

            _map
        }
    };
}

/// Quickly create a simple string error
#[macro_export]
macro_rules! perror {
    ($msg:expr) => {
        $crate::Error::Raw($msg.to_string())
    };

    ($template:expr, $($arg:expr),+) => {
        $crate::Error::Raw(format!($template, $($arg),+))
    };
}

/// Helper for converting Vec<String> -> &[&str]
#[macro_export]
macro_rules! str_slice {
    ($string_vec:expr) => {
        &$string_vec.iter().map(AsRef::as_ref).collect::<Vec<&str>>()
    };
}

/// Helper for quickly converting args to debug strings
#[macro_export]
macro_rules! strings {
    { $($arg:expr),+ } => { vec![$(format!("{:?}", $arg)),+] }
}

/// Cast elements of a slice as a given type
#[macro_export]
macro_rules! cast_slice {
    ($s:expr, $t:ty) => {
        $s.iter().map(|&v| v as $t).collect::<Vec<$t>>()
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
            $(VecImplInto $vecintofield:ident : $vecintoty:ty;)?
            $(ImplInto $intofield:ident : $intoty:ty;)?
            $(ImplTry $errty:ty; $tryfield:ident : $tryty:ty;)?
            $(Concrete $field:ident : $ty:ty;)?
            => $default:expr;
        )+
    } => {
        $(#[$struct_outer])*
        pub struct $name {
            $(
                pub(crate)
                $($vecintofield : Vec<$vecintoty>,)?
                $($intofield : $intoty,)?
                $($tryfield : $tryty,)?
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
                $(pub fn $vecintofield(&self) -> &Vec<$vecintoty> {
                        &self.$vecintofield
                })?
                $(pub fn $intofield(&self) -> &$intoty {
                        &self.$intofield
                })?
                $(pub fn $tryfield(&self) -> &$tryty {
                        &self.$tryfield
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
                        $($vecintofield: $default.into_iter().map(|e| e.into()).collect(),)?
                        $($intofield: $default.into(),)?
                        $($tryfield: $default.try_into().unwrap(),)?
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
                $(pub fn $vecintofield<T, U>(&mut self, val: T) -> &mut $builder_name
                where
                    T: IntoIterator<Item = U>,
                    U: Into<$vecintoty>,
                {
                    self.inner.$vecintofield = val.into_iter().map(|elem| elem.into()).collect();
                    self
                })?
                $(pub fn $intofield<T>(&mut self, val: T) -> &mut $builder_name
                where
                    T: Into<$intoty>
                {
                    self.inner.$intofield = val.into();
                    self
                })?
                $(pub fn $tryfield<T>(&mut self, val: T) -> crate::Result<&mut $builder_name>
                where
                    T: TryInto<$tryty, Error=$errty>,
                {
                    self.inner.$tryfield = val.try_into()?;
                    Ok(self)
                })?
                $(pub fn $field(&mut self, val: $ty) -> &mut $builder_name {
                    self.inner.$field = val;
                    self
                })?
            )+
        }
    }
}

// __impl_stub_xcon! {
//     for Foo;

//     client_properties: {}
//     client_handler: {}
//     client_config: {}
//     event_handler: {}
//     state: {}
//     conn: {}
// }
#[doc(hidden)]
#[macro_export]
macro_rules! __impl_stub_xcon {
    {
        for $struct:ident;

        atom_queries: { $($atomquery:tt)* }
        client_properties: { $($cprops:tt)* }
        client_handler: { $($chandler:tt)* }
        client_config: { $($cconfig:tt)* }
        event_handler: { $($ehandler:tt)* }
        state: { $($state:tt)* }
        conn: { $($conn:tt)* }
    } => {
        impl $crate::xconnection::StubXAtomQuerier for $struct { $($atomquery)* }
        impl $crate::xconnection::StubXClientProperties for $struct { $($cprops)* }
        impl $crate::xconnection::StubXClientHandler for $struct { $($chandler)* }
        impl $crate::xconnection::StubXClientConfig for $struct { $($cconfig)* }
        impl $crate::xconnection::StubXEventHandler for $struct { $($ehandler)* }
        impl $crate::xconnection::StubXState for $struct { $($state)* }
        impl $crate::xconnection::StubXConn for $struct { $($conn)* }
    }
}

// Helper to avoid polluting the documented patterns in other public macros
#[doc(hidden)]
#[macro_export]
macro_rules! __private {

    /*
     *  @parsekey :: handle each of the valid cases in an invocation of gen_keybindings
     */

    {   @parsekey $map:expr, $codes:expr, $parse:expr,
        [ $($patt:expr,)* ], [ $(($($template:expr),+; $($name:expr),+)),* ],
        map: { $($str:expr),+ } to $to:expr => {
            $( $binding:expr => $method:ident ( $($params:tt)* ); )+
        };
        $($tail:tt)*
    } => {
        {
            let keynames = &[$($str),+];
            $(
                for (name, arg) in keynames.iter().zip($to.into_iter()) {
                    let binding = format!($binding, name);
                    match $parse(binding.clone(), &$codes) {
                        None => panic!("invalid key binding: {}", binding),
                        Some(key_code) => $map.insert(
                            key_code,
                            run_internal!(
                                $method,
                                $crate::__private!(@parsemapparams arg; []; $($params,)*)
                            )
                        ),
                    };
                }
            )+

            $crate::__private!(@parsekey $map, $codes, $parse,
                [ $($patt,)* ], [ $(($($template),+; $($name),+),)* ($($binding),+; $($str),+) ],
                $($tail)*
            );
        }
    };

    // parse a single simple key binding (validated if $validate is true)
    {   @parsekey $map:expr, $codes:expr, $parse:expr,
        [ $($patt:expr,)* ], [ $(($($template:expr),+; $($name:expr),+)),* ],
        $binding:expr => $action:expr;
        $($tail:tt)*
    } => {
        match $parse($binding.to_string(), &$codes) {
            None => panic!("invalid key binding: {}", $binding),
            Some(key_code) => $map.insert(key_code, $action),
        };
        $crate::__private!(@parsekey $map, $codes, $parse,
            [ $binding, $($patt,)* ], [ $(($($template),+; $($name),+)),* ],
            $($tail)*
        );
    };

    // base case (should be out of tokens)
    {   @parsekey $map:expr, $codes:expr, $parse:expr,
        [ $($patt:expr,)* ], [ $(($($template:expr),+; $($name:expr),+)),* ],
        $($tail:tt)*
    } => {
        $(compile_error!(stringify!("unexpected tokens in gen_keybindings macro: " $tail));)*
        $crate::validate_user_bindings!(
            ( $($patt),* )
            ( $((($($template),+) ($($name),+)))* )
        )
    };

    /*
     *  @parsemapparams :: run variable replacement for a `map` block in `gen_keybindings`
     */

    { @parsemapparams $replacement:expr; [ $(,$arg:expr)* ];
      REF, $($params:tt)*
    } => {
        $crate::__private!(@parsemapparams $replacement; [$($arg),* , &$replacement]; $($params)*)
    };

    { @parsemapparams $replacement:expr; [ $(,$arg:expr)* ];
      VAL, $($params:tt)*
    } => {
        $crate::__private!(@parsemapparams $replacement; [$($arg),* , $replacement]; $($params)*)
    };

    { @parsemapparams $replacement:expr; [ $(,$arg:expr),* ];
      $expr:expr, $($params:tt)*
    } => {
        $crate::__private!(@parsemapparams $replacement; [$($arg),* , $expr]; $($params)*)
    };

    { @parsemapparams $replacement:expr; [ $(,$arg:expr)* ]; } => { $($arg),* };
}
