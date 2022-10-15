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
