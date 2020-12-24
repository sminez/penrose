//! Helpers and utilities for using XCB as a back end for penrose
use crate::{
    bindings::{KeyCode, MouseState},
    data_types::{Point, Region, WinId},
    screen::Screen,
    xconnection::{Atom, XEvent},
    Result,
};

pub mod api;
pub mod xconn;

pub enum PropVal<'a> {
    Atom(&'a [u32]),
    Cardinal(&'a [u32]),
    Str(&'a str),
    Window(&'a [WinId]),
}

pub enum WinType {
    CheckWin,
    InputOnly,
    InputOutput(Atom),
}

pub enum WinConfig {
    BorderPx(u32),
    Position(Region),
    StackAbove,
}
impl WinConfig {
    pub fn as_data(&self) -> Vec<(u16, u32)> {
        match self {
            Self::BorderPx(px) => vec![(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, *px)],
            Self::Position(region) => {
                let (x, y, w, h) = region.values();
                vec![
                    (xcb::CONFIG_WINDOW_X as u16, x),
                    (xcb::CONFIG_WINDOW_Y as u16, y),
                    (xcb::CONFIG_WINDOW_WIDTH as u16, w),
                    (xcb::CONFIG_WINDOW_HEIGHT as u16, h),
                ]
            }
            Self::StackAbove => vec![(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
        }
    }
}

pub enum WinAttr {
    BorderColor(u32),
    ClientEventMask,
    RootEventMask,
}
impl WinAttr {
    pub fn as_data(&self) -> Vec<(u32, u32)> {
        let client_event_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_STRUCTURE_NOTIFY;

        let root_event_mask = xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xcb::EVENT_MASK_BUTTON_MOTION;

        match self {
            Self::BorderColor(c) => vec![(xcb::CW_BORDER_PIXEL, *c)],
            Self::ClientEventMask => vec![(xcb::CW_EVENT_MASK, client_event_mask)],
            Self::RootEventMask => vec![(xcb::CW_EVENT_MASK, root_event_mask)],
        }
    }
}

pub trait XcbApi {
    // atoms
    fn atom(&self, name: &str) -> Result<u32>;
    fn known_atom(&self, atom: Atom) -> u32;
    // properties
    fn delete_prop(&self, id: WinId, prop: Atom);
    fn get_atom_prop(&self, id: WinId, atom: Atom) -> Result<u32>;
    fn get_str_prop(&self, id: WinId, name: &str) -> Result<String>;
    fn replace_prop(&self, id: WinId, prop: Atom, val: PropVal);
    // clients / windows
    fn create_window(&self, ty: WinType, r: Region, screen: usize, managed: bool) -> Result<WinId>;
    fn configure_window(&self, id: WinId, conf: &[WinConfig]);
    fn current_clients(&self) -> Result<Vec<WinId>>;
    fn destroy_window(&self, id: WinId);
    fn focused_client(&self) -> Result<WinId>;
    fn map_window(&self, id: WinId);
    fn mark_focused_window(&self, id: WinId);
    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()>;
    fn set_window_attributes(&self, id: WinId, attrs: &[WinAttr]);
    fn unmap_window(&self, id: WinId);
    fn window_geometry(&self, id: WinId) -> Result<Region>;
    // screens
    fn current_screens(&self) -> Result<Vec<Screen>>;
    fn screen_sizes(&self) -> Result<Vec<Region>>;
    // input
    fn cursor_position(&self) -> Point;
    fn grab_keys(&self, keys: &[&KeyCode]);
    fn grab_mouse_buttons(&self, states: &[&MouseState]);
    fn ungrab_keys(&self);
    fn ungrab_mouse_buttons(&self);
    // misc
    fn flush(&self) -> bool;
    fn root(&self) -> WinId;
    fn set_notify_mask(&self) -> Result<()>;
    fn wait_for_event(&self) -> Option<XEvent>;
    fn warp_cursor(&self, id: WinId, x: usize, y: usize);
}
