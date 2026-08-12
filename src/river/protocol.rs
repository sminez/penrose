//! The river protocols, expanded from the XML in `src/river/protocol/`.
//!
//! Nothing here is written by hand: `wayland-scanner` generates the proxies at compile time from
//! the vendored XML, and the module nesting exists so that the cross protocol type references in
//! that XML resolve. See `protocol/SOURCE` for the river commit these were copied from.
#![allow(
    non_upper_case_globals,
    missing_docs,
    missing_debug_implementations,
    unused
)]

pub mod river_window_management_v1 {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("src/river/protocol/river-window-management-v1.xml");
    }

    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("src/river/protocol/river-window-management-v1.xml");
}

// Named without the _v1 to avoid colliding with the generated inner module of that name.
pub mod river_xkb_bindings {
    use wayland_client;
    use wayland_client::protocol::*;

    // river-xkb-bindings-v1.xml references river_seat_v1, which lives in the window management
    // protocol. The generated modules look for it in `super`, so it is re-exported here.
    pub use super::river_window_management_v1::river_seat_v1;

    pub mod __interfaces {
        use super::super::river_window_management_v1::__interfaces::*;
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("src/river/protocol/river-xkb-bindings-v1.xml");
    }

    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("src/river/protocol/river-xkb-bindings-v1.xml");
}

// Named without the _v1 to avoid colliding with the generated inner module of that name.
pub mod river_layer_shell {
    use wayland_client;
    use wayland_client::protocol::*;

    // river-layer-shell-v1.xml references river_output_v1 and river_seat_v1 from the window
    // management protocol.
    pub use super::river_window_management_v1::{river_output_v1, river_seat_v1};

    pub mod __interfaces {
        use super::super::river_window_management_v1::__interfaces::*;
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("src/river/protocol/river-layer-shell-v1.xml");
    }

    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("src/river/protocol/river-layer-shell-v1.xml");
}
