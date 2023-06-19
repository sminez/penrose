//! Utility macros

/// Quickly create a [crate::Error::Custom]
/// ```
/// # use penrose::custom_error;
/// let err = custom_error!("a simple error message");
///
/// let s = "templated";
/// let err = custom_error!("a {} error message", s);
/// ```
#[macro_export]
macro_rules! custom_error {
    ($msg:expr) => {
        $crate::Error::Custom($msg.to_string())
    };

    ($template:expr, $($arg:expr),+) => {
        $crate::Error::Custom(format!($template, $($arg),+))
    };
}

/// Make creating a pre-defined HashMap a little less verbose
///
/// ```
/// # use penrose::map;
/// map! {
///     1 => "one",
///     2 => "two",
///     3 => "three",
/// };
/// ```
#[macro_export]
macro_rules! map {
    {} => { ::std::collections::HashMap::new() };

    {
        map_keys: $mapper:expr;
        $($key:expr => $value:expr),+,
    } => {
        {
            let mut _map: ::std::collections::HashMap<_, _> = ::std::collections::HashMap::new();
            $(_map.insert($mapper($key), $value);)+
            _map
        }
    };

    { $($key:expr => $value:expr),+, } => {
        {
            let mut _map: ::std::collections::HashMap<_, _> = ::std::collections::HashMap::new();
            $(_map.insert($key, $value);)+
            _map
        }
    };
}

/// Compose together a set of ManageHook query/hook rules into a single ManageHook
/// that can be added to your Config.
///
/// ```
/// use penrose::{
///     core::{hooks::ManageHook, Config},
///     extensions::hooks::manage::SetWorkspace,
///     x::query::ClassName,
///     x11rb::RustConn,
///     manage_hooks,
/// };
///
///
/// let my_manage_hook = manage_hooks! {
///     ClassName("discord") => SetWorkspace("9"),
///     ClassName("vlc") => SetWorkspace("5"),
/// };
///
/// let config: Config<RustConn> = Config {
///     manage_hook: Some(my_manage_hook),
///     ..Default::default()
/// };
/// ```
#[macro_export]
macro_rules! manage_hooks {
    { $($query:expr => $hook:expr),+, } => {
        {
            let mut _hooks = vec![];
            $(
                _hooks.push(
                    $crate::core::hooks::ManageHook::boxed(($query, $hook))
                );
            )+
            Box::new(_hooks)
        }
    };
}

// Helper for popping from the middle of a linked list
#[doc(hidden)]
#[macro_export]
macro_rules! pop_where {
    ($self:ident, $lst:ident, $($pred:tt)+) => {{
        let placeholder = ::std::mem::take(&mut $self.$lst);

        let mut remaining = ::std::collections::LinkedList::default();
        let mut popped = None;
        let pred = $($pred)+;

        for item in placeholder.into_iter() {
            if pred(&item) {
                popped = Some(item);
            } else {
                remaining.push_back(item);
            }
        }

        ::std::mem::swap(&mut $self.$lst, &mut remaining);

        popped
    }};
}
