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
            let mut _map: ::std::collections::HashMap<_, _> = ::std::collections::HashMap::new();
            $(_map.insert($key, $value);)+
            _map
        }
    };
}

#[macro_export]
macro_rules! modify {
    ($($tokens:tt)+) => {
        Box::new($crate::bindings::Modify($($tokens)+)) as Box<dyn $crate::bindings::KeyEventHandler<_, _>>
    }
}

#[macro_export]
macro_rules! spawn {
    ($s:expr) => {
        Box::new(|_, _| $crate::util::spawn($s)) as Box<dyn $crate::bindings::KeyEventHandler<_, _>>
    };
}

#[macro_export]
macro_rules! layout_message {
    ($m:expr) => {
        Box::new(|s: &mut $crate::core::State<_, _>, _| {
            s.client_set.current_workspace_mut().broadcast_message($m);
            Ok(())
        }) as Box<dyn $crate::bindings::KeyEventHandler<_, _>>
    };
}
