use super::BTN_LEFT;
use crate::wayland::state::WaylandState;
use smithay::{
    desktop::Window,
    input::pointer::{
        AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
        GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
        GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab,
        PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

pub struct MoveSurfaceGrab {
    pub start_data: GrabStartData<WaylandState>,
    pub start_point: Point<i32, Logical>,
    pub win: Window,
}

impl PointerGrab<WaylandState> for MoveSurfaceGrab {
    fn start_data(&self) -> &GrabStartData<WaylandState> {
        &self.start_data
    }

    fn motion(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        _focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        // No client has pointer focus while the grab is active
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_loc = (self.start_point.to_f64() + delta).to_i32_round();
        data.space.map_element(self.win.clone(), new_loc, true);
    }

    fn relative_motion(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event)
    }

    fn button(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        // TODO: handle more button presses as needed
        if !handle.current_pressed().contains(&BTN_LEFT) {
            // User released BTN_LEFT -> release the grab.
            handle.unset_grab(data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
    ) {
        handle.frame(data)
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event)
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event)
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event)
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event)
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event)
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event)
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event)
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut WaylandState,
        handle: &mut PointerInnerHandle<'_, WaylandState>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event)
    }
}
