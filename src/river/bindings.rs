//! Key and mouse bindings, registered with river rather than grabbed.
//!
//! River binds keys on the window manager's behalf: `get_xkb_binding` names a keysym and a
//! modifier mask and reports `pressed`/`released` for it. The window manager never sees an
//! unbound key, so there is no grab, no keymap and no `xmodmap` -- and no key type of its own,
//! since [KeySym] is already keysym plus mask on every backend.
//!
//! The consequence for key sequences is that every key of a sequence needs a binding object, not
//! just the leaders. On X11 the rest of a sequence arrives through a keyboard grab, which reports
//! whatever was pressed; here a key with no binding registered produces no event at all, only
//! `ate_unbound_key`, which carries no keysym. So the continuation keys are enabled for the
//! duration of the capture: see river-design.md §5.
//!
//! **All of this lives on the loop.** Which bindings are live has to be coherent with the key
//! press river matched against them, and `ate_unbound_key` arrives here; if the worker owned any
//! of it, that callback would be reading a slot the worker was concurrently writing. The worker
//! says only "these are the bindings" and "eat the next key, these would continue it", as ops.
use crate::{
    Result,
    core::{
        State,
        bindings::{KeySym, MouseButton, MouseEvent, MouseEventData, MouseEventKind, MouseState},
        conn::WinId,
    },
    river::{RiverConn, RiverEvent, protocol, wayland::Loop},
};
use protocol::{
    river_window_management_v1::{
        river_pointer_binding_v1::RiverPointerBindingV1, river_seat_v1::Modifiers,
    },
    river_xkb_bindings::{
        river_xkb_binding_v1::RiverXkbBindingV1, river_xkb_bindings_seat_v1::RiverXkbBindingsSeatV1,
    },
};
use std::collections::HashSet;
use tracing::{debug, trace, warn};
use wayland_client::{Connection, Proxy, QueueHandle};

/// Linux input event codes, which is what river names pointer buttons with, where penrose names
/// them with X11 button numbers.
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

/// River's modifier bits are X11's, which is why a config's `M-` and `S-` carry over unchanged.
fn modifiers(mask: u16) -> Modifiers {
    Modifiers::from_bits_truncate(mask as u32)
}

fn button_code(button: MouseButton) -> Option<u32> {
    match button {
        MouseButton::Left => Some(BTN_LEFT),
        MouseButton::Middle => Some(BTN_MIDDLE),
        MouseButton::Right => Some(BTN_RIGHT),
        // Scrolling is an axis event rather than a button, so river has nothing to bind.
        MouseButton::ScrollUp | MouseButton::ScrollDown => None,
    }
}

impl Loop {
    /// Register bindings for the given keys and mouse states, and stop matching the rest.
    ///
    /// Binding objects may be created at any time but only enabled inside a manage sequence, so
    /// the objects are made here and the enabling happens when the sequence is transmitted.
    pub(super) fn grab(&mut self, keys: Vec<KeySym>, mouse: Vec<MouseState>) {
        self.grabbed_keys = keys;
        self.grabbed_mouse = mouse;
        self.rebind_seats();
    }

    /// Eat the next non-modifier key press, and listen for the keys which would continue the
    /// sequence in progress.
    pub(super) fn capture(&mut self, continuations: Vec<KeySym>) {
        trace!(?continuations, "capturing the next key press");
        // A second call replaces the continuations rather than adding to them, and does not
        // extend the capture: river eats one key either way.
        self.capture_continuations = continuations;
        self.rebind_seats();

        if !self.capture_armed {
            self.capture_armed = true;
            self.for_each_seat(|s| {
                if let Some(xkb) = s.xkb.as_ref() {
                    xkb.ensure_next_key_eaten();
                }
            });
        }
    }

    /// Drop a capture which has not yet eaten anything.
    pub(super) fn cancel_capture(&mut self) {
        self.capture_continuations.clear();

        if self.capture_armed {
            self.capture_armed = false;
            // River documents the race: ate_unbound_key may already be in flight, in which case
            // this request does nothing and the event arrives anyway.
            self.for_each_seat(|s| {
                if let Some(xkb) = s.xkb.as_ref() {
                    xkb.cancel_ensure_next_key_eaten();
                }
            });
        }
    }

    /// A key press has been eaten by the capture, whether or not it triggered a binding.
    fn capture_consumed(&mut self) {
        self.capture_armed = false;
        self.capture_continuations.clear();
    }

    /// Create binding objects for anything a seat does not have yet.
    ///
    /// Called when the grabbed set changes, when a capture names continuation keys, and when a
    /// new seat appears. The continuations need objects as much as the leaders do: without one
    /// river has no binding to match and the key produces no event at all, only `ate_unbound_key`
    /// -- which is indistinguishable from the user typing something genuinely not in the sequence.
    pub(super) fn rebind_seats(&mut self) {
        let keys: Vec<KeySym> = self
            .grabbed_keys
            .iter()
            .chain(self.capture_continuations.iter())
            .copied()
            .collect();
        let (mouse, qh) = (self.grabbed_mouse.clone(), self.qh.clone());

        for seat in self.seats.iter_mut().filter(|s| !s.removed) {
            for &key in keys.iter() {
                seat.key_bindings.entry(key).or_insert_with(|| {
                    self.xkb
                        .get_xkb_binding(&seat.obj, key.keysym, modifiers(key.mask), &qh, key)
                });
            }

            for state in mouse.iter() {
                let Some(code) = button_code(state.button) else {
                    warn!(button = ?state.button, "river has no pointer binding for this button");
                    continue;
                };

                if !seat.mouse_bindings.contains_key(state) {
                    let binding = seat.obj.get_pointer_binding(
                        code,
                        modifiers(state.mask()),
                        &qh,
                        state.clone(),
                    );
                    seat.mouse_bindings.insert(state.clone(), binding);
                }
            }
        }
    }

    /// Enable what should be live and disable what should not.
    ///
    /// The leaders stay enabled during a capture: pressing one mid sequence is how a sequence
    /// that goes nowhere is abandoned, and core needs the press to work that out.
    pub(super) fn transmit_bindings(&mut self) {
        // Nothing at all while the session is locked. River matches keys against these before the
        // lock screen sees them, so leaving them live would hand every shortcut in the config to
        // whoever is in front of a locked machine.
        let (enabled, mouse_enabled): (HashSet<KeySym>, HashSet<&MouseState>) = if self
            .session_locked
        {
            (HashSet::new(), HashSet::new())
        } else {
            (
                self.grabbed_keys
                    .iter()
                    .chain(self.capture_continuations.iter())
                    .copied()
                    .collect(),
                self.grabbed_mouse.iter().collect(),
            )
        };

        for seat in self.seats.iter().filter(|s| !s.removed) {
            for (key, binding) in seat.key_bindings.iter() {
                if enabled.contains(key) {
                    binding.enable();
                } else {
                    binding.disable();
                }
            }

            for (state, binding) in seat.mouse_bindings.iter() {
                if mouse_enabled.contains(state) {
                    binding.enable();
                } else {
                    binding.disable();
                }
            }
        }
    }
}

/// Abandon a key sequence which has been ended by a key that is not part of it.
///
/// River's `ate_unbound_key` means "a key was eaten and it was not one of yours", which is
/// exactly the abort signal, and core has no way to say it: `dispatch_key` takes a key and there
/// is no value which means "not a key". The capture itself is already gone -- the loop dropped it
/// when the key was eaten -- so all that is left is the pending keys.
pub(super) fn abandon_key_sequence(state: &mut State<RiverConn>, _: &mut RiverConn) -> Result<()> {
    if !state.pending_keys.is_empty() {
        debug!(pending = ?state.pending_keys, "abandoning key sequence: unbound key");
        state.pending_keys.clear();
    }

    Ok(())
}

impl wayland_client::Dispatch<RiverXkbBindingV1, KeySym> for Loop {
    fn event(
        l: &mut Self,
        _: &RiverXkbBindingV1,
        event: protocol::river_xkb_bindings::river_xkb_binding_v1::Event,
        key: &KeySym,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_xkb_bindings::river_xkb_binding_v1::Event;

        if let Event::Pressed = event {
            // A binding firing while a capture is armed is the capture being spent: river eats
            // the key and delivers it here rather than as ate_unbound_key.
            if l.capture_armed {
                l.capture_consumed();
            }

            l.send_event(RiverEvent::KeyPress(*key));
        }
    }
}

impl wayland_client::Dispatch<RiverXkbBindingsSeatV1, ()> for Loop {
    fn event(
        l: &mut Self,
        _: &RiverXkbBindingsSeatV1,
        event: protocol::river_xkb_bindings::river_xkb_bindings_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_xkb_bindings::river_xkb_bindings_seat_v1::Event;

        if let Event::AteUnboundKey = event {
            l.capture_consumed();
            l.send_event(RiverEvent::UnboundKey);
        }
    }
}

impl wayland_client::Dispatch<RiverPointerBindingV1, MouseState> for Loop {
    fn event(
        l: &mut Self,
        binding: &RiverPointerBindingV1,
        event: protocol::river_window_management_v1::river_pointer_binding_v1::Event,
        state: &MouseState,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use protocol::river_window_management_v1::river_pointer_binding_v1::Event;

        let kind = match event {
            Event::Pressed => MouseEventKind::Press,
            Event::Released => MouseEventKind::Release,
        };

        // River tells us that a binding fired, not where the pointer was: the seat's last
        // reported position is the closest thing to X11's event coordinates.
        let rpt = l
            .seats
            .iter()
            .find(|s| s.mouse_bindings.values().any(|b| b.id() == binding.id()))
            .map(|s| s.pointer)
            .unwrap_or_default();

        let id = l
            .render
            .positions
            .iter()
            .find(|(id, p)| {
                l.windows
                    .get(id)
                    .and_then(|w| w.facts.dimensions)
                    .is_some_and(|(w, h)| {
                        (p.x..p.x + w as i32).contains(&rpt.x)
                            && (p.y..p.y + h as i32).contains(&rpt.y)
                    })
            })
            .map(|(&id, _)| id)
            .unwrap_or(WinId(0));

        let wpt = l
            .render
            .positions
            .get(&id)
            .map(|p| crate::pure::geometry::Point {
                x: rpt.x - p.x,
                y: rpt.y - p.y,
            })
            .unwrap_or(rpt);

        l.send_event(RiverEvent::MouseEvent(MouseEvent {
            data: MouseEventData { id, rpt, wpt },
            state: state.clone(),
            kind,
        }));
    }
}
