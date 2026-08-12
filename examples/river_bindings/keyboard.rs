//! A virtual keyboard on a connection of its own.
//!
//! A headless seat has no keyboard, and river matches bindings against a seat's keyboard state,
//! so without this there is nothing to press.
#![allow(non_upper_case_globals, missing_docs, unused)]

use std::{os::fd::BorrowedFd, thread, time::Duration};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{wl_registry, wl_seat},
};

pub mod protocol {
    use wayland_client;
    use wayland_client::protocol::*;

    pub mod __interfaces {
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!(
            "examples/river_bindings/protocol/virtual-keyboard-unstable-v1.xml"
        );
    }

    use self::__interfaces::*;
    wayland_scanner::generate_client_code!(
        "examples/river_bindings/protocol/virtual-keyboard-unstable-v1.xml"
    );
}

use protocol::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// The state wayland events dispatch against. There is nothing to keep: a virtual keyboard only
/// sends.
#[derive(Debug)]
pub struct KeyboardState;

pub struct VirtualKeyboard {
    queue: EventQueue<KeyboardState>,
    kb: ZwpVirtualKeyboardV1,
    /// A made up millisecond clock, which is all a key event's timestamp has to be.
    time: u32,
}

/// `wl_keyboard.keymap` format for an xkb keymap in text form.
const XKB_V1: u32 = 1;

const PRESSED: u32 = 1;
const RELEASED: u32 = 0;

impl VirtualKeyboard {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
        let (globals, mut queue) =
            registry_queue_init::<KeyboardState>(&conn).map_err(|e| e.to_string())?;
        let qh = queue.handle();

        let seat: wl_seat::WlSeat = globals
            .bind(&qh, 1..=9, ())
            .map_err(|e| format!("no wl_seat: {e}"))?;
        let manager: ZwpVirtualKeyboardManagerV1 = globals.bind(&qh, 1..=1, ()).map_err(|e| {
            format!("no zwp_virtual_keyboard_manager_v1 (is river built without it?): {e}")
        })?;

        let kb = manager.create_virtual_keyboard(&seat, &qh, ());
        queue
            .roundtrip(&mut KeyboardState)
            .map_err(|e| e.to_string())?;

        Ok(Self { queue, kb, time: 1 })
    }

    pub fn keymap(&mut self, fd: BorrowedFd<'_>, size: u32) {
        self.kb.keymap(XKB_V1, fd, size);
    }

    pub fn roundtrip(&mut self) -> Result<(), String> {
        self.queue
            .roundtrip(&mut KeyboardState)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Press and release a key with no modifiers held.
    pub fn tap(&mut self, key: u32) {
        self.press(key);
        self.release(key);
        self.settle();
    }

    /// Press and release a key with a modifier held, the way a person types a shortcut.
    pub fn chord(&mut self, modifiers: u32, key: u32) {
        self.kb.modifiers(modifiers, 0, 0, 0);
        self.press(key);
        self.release(key);
        self.kb.modifiers(0, 0, 0, 0);
        self.settle();
    }

    fn press(&mut self, key: u32) {
        self.time += 10;
        self.kb.key(self.time, key, PRESSED);
    }

    fn release(&mut self, key: u32) {
        self.time += 10;
        self.kb.key(self.time, key, RELEASED);
    }

    /// River stalls input while a manage sequence is open, so the presses do not need pacing to
    /// be handled in order -- but the window manager's own actions do, and a test that raced
    /// would be worse than useless.
    fn settle(&mut self) {
        let _ = self.roundtrip();
        thread::sleep(Duration::from_millis(400));
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for KeyboardState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(KeyboardState: ignore wl_seat::WlSeat);
wayland_client::delegate_noop!(KeyboardState: ignore ZwpVirtualKeyboardManagerV1);
wayland_client::delegate_noop!(KeyboardState: ignore ZwpVirtualKeyboardV1);
