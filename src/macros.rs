/// log the reason why we we're dying and run cleanup (if any)
#[macro_export]
macro_rules! die(
    ($msg:expr) => ({
        eprintln!("FATAL :: {}", $msg);
        ::std::process::exit(42);
     });

    ($fmt:expr, $($arg:expr),*) => ({
        eprintln!("FATAL :: {}", format!($fmt, $($arg,)*));
        ::std::process::exit(42);
     });
);

#[macro_export]
macro_rules! warn(
    ($msg:expr) => { eprintln!("WARN :: {}", $msg); };
    ($fmt:expr, $($arg:tt),*) => {
        eprintln!("WARN :: {}", format!($fmt, $($arg)*));
    };
);

#[macro_export]
macro_rules! log(
    ($msg:expr) => { eprintln!("INFO :: {}", $msg); };
    ($fmt:expr, $($arg:expr),*) => {
        eprintln!("INFO :: {}", format!($fmt, $($arg,)*));
    };
);

/// kick off an external program as part of a key/mouse binding
#[macro_export]
macro_rules! run_external(
    ($cmd:tt) => {
        {
            let parts: Vec<&str> = $cmd.split_whitespace().collect();
            if parts.len() > 1 {
                Box::new(move |_: &mut $crate::manager::WindowManager| {
                    match ::std::process::Command::new(parts[0]).args(&parts[1..]).spawn() {
                        Ok(_) => (),
                        Err(e) => warn!("error spawning external program: {}", e),
                    };
                }) as $crate::data_types::FireAndForget
            } else {
                Box::new(move |_: &mut $crate::manager::WindowManager| {
                    match ::std::process::Command::new(parts[0]).spawn() {
                        Ok(_) => (),
                        Err(e) => warn!("error spawning external program: {}", e),
                    };
                }) as $crate::data_types::FireAndForget
            }
        }
    };
);

/// kick off an internal method on the window manager as part of a key/mouse binding
#[macro_export]
macro_rules! run_internal(
    ($func:ident) => {
        Box::new(|wm: &mut $crate::manager::WindowManager| {
            log!("calling method ({})", stringify!($func));
            wm.$func()
        })
    };

    ($func:ident, $arg:tt) => {
        Box::new(move |wm: &mut $crate::manager::WindowManager| wm.$func($arg))
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

/// make creating a hash-map a little less verbose
#[macro_export]
macro_rules! gen_keybindings(
    {
        $($binding:expr => $action:expr),+;
        forall_tags: $tag_array:expr => { $($tag_binding:expr => $tag_action:tt),+, }
    } => {
        {
            let mut _map = ::std::collections::HashMap::new();
            let keycodes = $crate::helpers::keycodes_from_xmodmap();

            $(
                match $crate::helpers::parse_key_binding($binding, &keycodes) {
                    Some(key_code) => _map.insert(key_code, $action),
                    None => die!("invalid key binding: {}", $binding),
                };
            )+

            for i in 1..$tag_array.len() {
                $(
                    let for_tag = format!($tag_binding, i);
                    match $crate::helpers::parse_key_binding(for_tag.clone(), &keycodes) {
                        Some(key_code) => _map.insert(key_code, run_internal!($tag_action, i)),
                        None => die!("invalid key binding: {}", for_tag),
                    };
                )+
            }

            _map
        }
    };
);
