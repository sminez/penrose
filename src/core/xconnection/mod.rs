//! An abstraciton layer for talking to an underlying X server.
//!
//! An implementation of the [XConn] trait is required for running a [WindowManager][1]. The choice
//! of back end (e.g. xlib, xcb...) is an implementation detail that does not surface in the
//! `WindowManager` itself. All low level details of working with the X server should be captured in
//! this trait, though accessing backend specific functionality is possible by writing an impl
//! block for `WindowManager<YourXConn>` if desired.
//!
//! [1]: crate::core::manager::WindowManager
use crate::{
    core::{
        bindings::{KeyBindings, KeyPress, MouseBindings},
        client::Client,
        data_types::{Point, Region},
        screen::Screen,
    },
    draw::Color,
};

use penrose_proc::stubbed_companion_trait;

pub mod atom;
pub mod event;
pub mod property;

pub use atom::{
    Atom, AtomIter, AUTO_FLOAT_WINDOW_TYPES, EWMH_SUPPORTED_ATOMS, UNMANAGED_WINDOW_TYPES,
};
pub use event::{
    ClientEventMask, ClientMessage, ClientMessageKind, ConfigureEvent, ExposeEvent, PointerChange,
    PropertyEvent, XEvent,
};
pub use property::{
    MapState, Prop, WindowAttributes, WindowClass, WindowState, WmHints, WmNormalHints,
    WmNormalHintsFlags,
};

/// An X resource ID
pub type Xid = u32;

const WM_NAME: &str = "penrose";

/// Enum to store the various ways that operations can fail in X traits
#[derive(thiserror::Error, Debug)]
pub enum XError {
    /// The underlying connection to the X server is closed
    #[error("The underlying connection to the X server is closed")]
    ConnectionClosed,

    /// Client data was malformed
    #[error("ClientMessage data must be 5 u32s: got {0}")]
    InvalidClientMessageData(usize),

    /// The requested property is not set for the given client
    #[error("The {0} property is not set for client {1}")]
    MissingProperty(String, Xid),

    /// A generic error type for use in user code when needing to construct
    /// a simple [XError].
    #[error("Unhandled error: {0}")]
    Raw(String),

    /// Parsing an [Atom][crate::core::xconnection::Atom] from a str failed.
    ///
    /// This happens when the atom name being requested is not a known atom.
    #[error(transparent)]
    Strum(#[from] strum::ParseError),

    /// An attempt was made to reference an atom that is not known to penrose
    #[error("{0} is not a known atom")]
    UnknownAtom(Xid),

    /// An attempt was made to reference a client that is not known to penrose
    #[error("{0} is not a known client")]
    UnknownClient(Xid),

    /*
     * Conversions from other penrose error types
     */
    /// Something went wrong using the [xcb][crate::xcb] module.
    ///
    /// See [XcbError][crate::xcb::XcbError] for variants.
    #[cfg(feature = "xcb")]
    #[error(transparent)]
    Xcb(#[from] crate::xcb::XcbError),

    /// Something went wrong using the [x11rb][crate::x11rb] module.
    ///
    /// See [X11rbError][crate::x11rb::X11rbError] for variants.
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rb(#[from] crate::x11rb::X11rbError),
}

/// Result type for errors raised by X traits
pub type Result<T> = std::result::Result<T, XError>;

/// On screen configuration options for X clients (not all are curently implemented)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClientConfig {
    /// The border width in pixels
    BorderPx(u32),
    /// Absolute size and position on the screen as a [Region]
    Position(Region),
    /// Mark this window as stacking on top of its peers
    StackAbove,
}

/// Attributes for an X11 client window (not all are curently implemented)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientAttr {
    /// Border color as an argb hex value
    BorderColor(u32),
    /// Set the pre-defined client event mask
    ClientEventMask,
    /// Set the pre-defined root event mask
    RootEventMask,
}

/// An [XEvent] parsed into a [KeyPress] if possible, otherwise the original `XEvent`
#[derive(Debug, Clone)]
pub enum KeyPressParseAttempt {
    /// The event was parasble as a [KeyPress]
    KeyPress(KeyPress),
    /// The event was not a [KeyPress]
    XEvent(XEvent),
}

/// Convert between string representations of X atoms and their IDs
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XAtomQuerier {
    /// Convert an X atom id to its human friendly name
    #[stub(Err(XError::Raw("mocked".into())))]
    fn atom_name(&self, atom: Xid) -> Result<String>;

    /// Fetch or intern an atom by name
    #[stub(Err(XError::Raw("mocked".into())))]
    fn atom_id(&self, name: &str) -> Result<Xid>;
}

/// State queries against the running X server
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XState: XAtomQuerier {
    /// The root window ID
    #[stub(42)]
    fn root(&self) -> Xid;

    /// Determine the currently connected [screens][Screen] and return their details
    #[stub(Ok(vec![]))]
    fn current_screens(&self) -> Result<Vec<Screen>>;

    /// Determine the current (x,y) position of the cursor relative to the root window.
    #[stub(Ok(Point::default()))]
    fn cursor_position(&self) -> Result<Point>;

    /// Warp the cursor to be within the specified window. If id == None then behaviour is
    /// definined by the implementor (e.g. warp cursor to active window, warp to center of screen)
    #[stub(Ok(()))]
    fn warp_cursor(&self, win_id: Option<Xid>, screen: &Screen) -> Result<()>;

    /// Return the current (x, y, w, h) dimensions of the requested window
    #[stub(Ok(Region::default()))]
    fn client_geometry(&self, id: Xid) -> Result<Region>;

    /// Run on startup/restart to determine already running windows that we need to track
    #[stub(Ok(vec![]))]
    fn active_clients(&self) -> Result<Vec<Xid>>;

    /// Return the client ID of the [crate::core::client::Client] that currently holds X focus
    #[stub(Ok(0))]
    fn focused_client(&self) -> Result<Xid>;
}

/// Sending and receiving X events
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XEventHandler {
    /// Flush pending actions to the X event loop
    #[stub(true)]
    fn flush(&self) -> bool;

    /// Wait for the next event from the X server and return it as an [XEvent]
    #[stub(Err(XError::Raw("mocked".into())))]
    fn wait_for_event(&self) -> Result<XEvent>;

    /// Send an X event to the target client
    ///
    /// The `msg` being sent can be composed by hand or, for known common message types, generated
    /// using the [build_client_event][1] method.
    ///
    /// [1]: XEventHandler::build_client_event
    #[stub(Err(XError::Raw("mocked".into())))]
    fn send_client_event(&self, msg: ClientMessage) -> Result<()>;

    /// Build the required event data for sending a known client event.
    #[stub(Err(XError::Raw("mocked".into())))]
    fn build_client_event(&self, kind: ClientMessageKind) -> Result<ClientMessage>;
}

/// Management of the visibility and lifecycle of X clients
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XClientHandler {
    /// Map a client to the display.
    #[stub(Ok(()))]
    fn map_client(&self, id: Xid) -> Result<()>;

    /// Unmap a client from the display.
    #[stub(Ok(()))]
    fn unmap_client(&self, id: Xid) -> Result<()>;

    /// Destroy and existing client.
    #[stub(Ok(()))]
    fn destroy_client(&self, id: Xid) -> Result<()>;

    /// Mark the given client as having focus
    #[stub(Ok(()))]
    fn focus_client(&self, id: Xid) -> Result<()>;

    /// Map a known penrose [Client] if it is not currently visible
    fn map_client_if_needed(&self, win: Option<&mut Client>) -> Result<()> {
        if let Some(c) = win {
            if !c.mapped {
                c.mapped = true;
                self.map_client(c.id())?;
            }
        }
        Ok(())
    }

    /// Unmap a known penrose [Client] if it is currently visible
    fn unmap_client_if_needed(&self, win: Option<&mut Client>) -> Result<()> {
        if let Some(c) = win {
            if c.mapped {
                c.mapped = false;
                self.unmap_client(c.id())?;
            }
        }
        Ok(())
    }
}

/// Querying and updating properties on X clients
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XClientProperties {
    /// Return the list of all properties set on the given client window
    ///
    /// Properties should be returned as their string name as would be used to intern the
    /// respective atom.
    #[stub(Ok(vec![]))]
    fn list_props(&self, id: Xid) -> Result<Vec<String>>;

    /// Query a property for a client by ID and name.
    ///
    /// Can fail if the property name is invalid or we get a malformed response from xcb.
    #[stub(Err(XError::Raw("mocked".into())))]
    fn get_prop(&self, id: Xid, name: &str) -> Result<Prop>;

    /// Delete an existing property from a client
    #[stub(Ok(()))]
    fn delete_prop(&self, id: Xid, name: &str) -> Result<()>;

    /// Change an existing property for a client
    #[stub(Ok(()))]
    fn change_prop(&self, id: Xid, name: &str, val: Prop) -> Result<()>;

    /*
     *  The following default implementations should used if possible.
     *
     *  Any custom implementations should take care to ensure that the state changes being made are
     *  equivaled to those implemented here.
     */

    /// Check to see if a given client window supports a particular protocol or not
    fn client_supports_protocol(&self, id: Xid, proto: &str) -> Result<bool> {
        match self.get_prop(id, Atom::WmProtocols.as_ref()) {
            Ok(Prop::Atom(protocols)) => Ok(protocols.iter().any(|p| p == proto)),
            Ok(p) => Err(XError::Raw(format!("Expected atoms, got {:?}", p))),
            Err(XError::MissingProperty(_, _)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Toggle the fullscreen state of the given client ID with the X server
    fn toggle_client_fullscreen(&self, id: Xid, client_is_fullscreen: bool) -> Result<()> {
        let data = if client_is_fullscreen {
            vec![]
        } else {
            vec![Atom::NetWmStateFullscreen.as_ref().to_string()]
        };

        self.change_prop(id, Atom::NetWmState.as_ref(), Prop::Atom(data))
    }

    /// Fetch a [client's][1] name proprty following ICCCM / EWMH standards
    ///
    /// [1]: crate::core::client::Client
    fn client_name(&self, id: Xid) -> Result<String> {
        match self.get_prop(id, Atom::NetWmName.as_ref()) {
            Ok(Prop::UTF8String(strs)) if !strs.is_empty() && !strs[0].is_empty() => {
                Ok(strs[0].clone())
            }

            _ => match self.get_prop(id, Atom::WmName.as_ref()) {
                Ok(Prop::UTF8String(strs)) if !strs.is_empty() => Ok(strs[0].clone()),
                Err(e) => Err(e),
                _ => Ok(String::new()),
            },
        }
    }
}

/// Modifying X client config and attributes
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XClientConfig {
    /// Configure the on screen appearance of a client window
    #[stub(Ok(()))]
    fn configure_client(&self, id: Xid, data: &[ClientConfig]) -> Result<()>;

    /// Set client attributes such as event masks, border color etc
    #[stub(Ok(()))]
    fn set_client_attributes(&self, id: Xid, data: &[ClientAttr]) -> Result<()>;

    /// Get the [WindowAttributes] for this client
    #[stub(Err(XError::Raw("mocked".into())))]
    fn get_window_attributes(&self, id: Xid) -> Result<WindowAttributes>;

    /*
     *  The following default implementations should used if possible.
     *
     *  Any custom implementations should take care to ensure that the state changes being made are
     *  equivaled to those implemented here.
     */

    /// Reposition the window identified by 'id' to the specifed region
    fn position_client(&self, id: Xid, r: Region, border: u32, stack_above: bool) -> Result<()> {
        let mut data = vec![ClientConfig::Position(r), ClientConfig::BorderPx(border)];
        if stack_above {
            data.push(ClientConfig::StackAbove);
        }
        self.configure_client(id, &data)
    }

    /// Raise the window to the top of the stack so it renders above peers
    fn raise_client(&self, id: Xid) -> Result<()> {
        self.configure_client(id, &[ClientConfig::StackAbove])
    }

    /// Change the border color for the given client
    fn set_client_border_color(&self, id: Xid, color: Color) -> Result<()> {
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.rgb_u32())])
    }
}

/// Keyboard input for created clients
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XKeyboardHandler {
    /// Attempt to grab control of all keyboard input
    #[stub(Ok(()))]
    fn grab_keyboard(&self) -> Result<()>;

    /// Attempt to release control of all keyboard inputs
    #[stub(Ok(()))]
    fn ungrab_keyboard(&self) -> Result<()>;

    /// Attempt to parse the next [XEvent] from an underlying connection as a [KeyPress] if there
    /// is one.
    ///
    /// Should return Ok(None) if no events are currently available.
    #[stub(Ok(None))]
    fn next_keypress(&self) -> Result<Option<KeyPressParseAttempt>>;

    /// Wait for the next [XEvent] from an underlying connection as a [KeyPress] and attempt to
    /// parse it as a [KeyPress].
    #[stub(Err(XError::Raw("mocked".into())))]
    fn next_keypress_blocking(&self) -> Result<KeyPressParseAttempt>;
}

/// A handle on a running X11 connection that we can use for issuing X requests.
///
/// XConn is intended as an abstraction layer to allow for communication with the underlying
/// display system (assumed to be X) using whatever mechanism the implementer wishes. In theory, it
/// should be possible to write an implementation that allows penrose to run on systems not using X
/// as the windowing system but X idioms and high level event types / client interations are
/// assumed.
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XConn:
    XState + XEventHandler + XClientHandler + XClientProperties + XClientConfig + Sized
{
    /// Hydrate this XConn to restore internal state following serde deserialization
    #[cfg(feature = "serde")]
    #[stub(Ok(()))]
    fn hydrate(&mut self) -> Result<()>;

    /// Initialise any state required before this connection can be used by the WindowManager.
    ///
    /// This must include checking to see if another window manager is running and return an error
    /// if there is, but other than that there are no other requirements.
    ///
    /// This method is called once during [WindowManager::init][1]
    ///
    /// [1]: crate::core::manager::WindowManager::init
    #[stub(Ok(()))]
    fn init(&self) -> Result<()>;

    /// An X id for a check window that will be used for holding EWMH window manager properties
    ///
    /// The creation of any resources required for this should be handled in `init` and the
    /// destruction of those resources should be handled in `cleanup`.
    #[stub(0)]
    fn check_window(&self) -> Xid;

    /// Perform any state cleanup required prior to shutting down the window manager
    #[stub(Ok(()))]
    fn cleanup(&self) -> Result<()>;

    /// Notify the X server that we are intercepting the user specified key bindings and prevent
    /// them being passed through to the underlying applications.
    ///
    /// This is what determines which key press events end up being sent through in the main event
    /// loop for the WindowManager.
    #[stub(Ok(()))]
    fn grab_keys(
        &self,
        key_bindings: &KeyBindings<Self>,
        mouse_bindings: &MouseBindings<Self>,
    ) -> Result<()>;

    /*
     *  The following default implementations should used if possible.
     *
     *  Any custom implementations should take care to ensure that the state changes being made are
     *  equivaled to those implemented here.
     */

    /// Mark the given client as newly created
    fn mark_new_client(&self, id: Xid) -> Result<()> {
        self.set_client_attributes(id, &[ClientAttr::ClientEventMask])
    }

    /// Set required EWMH properties to ensure compatability with external programs
    fn set_wm_properties(&self, workspaces: &[String]) -> Result<()> {
        let root = self.root();
        let check_win = self.check_window();
        for &win in &[check_win, root] {
            self.change_prop(
                win,
                Atom::NetSupportingWmCheck.as_ref(),
                Prop::Window(vec![check_win]),
            )?;

            self.change_prop(
                win,
                Atom::WmName.as_ref(),
                Prop::UTF8String(vec![WM_NAME.into()]),
            )?;
        }

        // EWMH support
        self.change_prop(
            root,
            Atom::NetSupported.as_ref(),
            Prop::Atom(
                EWMH_SUPPORTED_ATOMS
                    .iter()
                    .map(|a| a.as_ref().to_string())
                    .collect(),
            ),
        )?;
        self.update_desktops(workspaces)?;
        self.delete_prop(root, Atom::NetClientList.as_ref())?;
        self.delete_prop(root, Atom::NetClientListStacking.as_ref())
    }

    /// Update the root window properties with the current desktop details
    fn update_desktops(&self, workspaces: &[String]) -> Result<()> {
        let root = self.root();
        self.change_prop(
            root,
            Atom::NetNumberOfDesktops.as_ref(),
            Prop::Cardinal(workspaces.len() as u32),
        )?;
        self.change_prop(
            root,
            Atom::NetDesktopNames.as_ref(),
            Prop::UTF8String(workspaces.to_vec()),
        )
    }

    /// Update the root window properties with the current client details
    fn update_known_clients(&self, clients: &[Xid]) -> Result<()> {
        let root = self.root();
        self.change_prop(
            root,
            Atom::NetClientList.as_ref(),
            Prop::Window(clients.to_vec()),
        )?;
        self.change_prop(
            root,
            Atom::NetClientListStacking.as_ref(),
            Prop::Window(clients.to_vec()),
        )
    }

    /// Update which desktop is currently focused
    fn set_current_workspace(&self, wix: usize) -> Result<()> {
        self.change_prop(
            self.root(),
            Atom::NetCurrentDesktop.as_ref(),
            Prop::Cardinal(wix as u32),
        )
    }

    /// Set the WM_NAME prop of the root window
    fn set_root_window_name(&self, name: &str) -> Result<()> {
        self.change_prop(
            self.root(),
            Atom::WmName.as_ref(),
            Prop::UTF8String(vec![name.to_string()]),
        )
    }

    /// Update which desktop a client is currently on
    fn set_client_workspace(&self, id: Xid, wix: usize) -> Result<()> {
        self.change_prop(id, Atom::NetWmDesktop.as_ref(), Prop::Cardinal(wix as u32))
    }

    /// Determine whether the target client should be tiled or allowed to float
    fn client_should_float(&self, id: Xid, floating_classes: &[&str]) -> bool {
        if let Ok(Prop::UTF8String(strs)) = self.get_prop(id, Atom::WmClass.as_ref()) {
            if strs.iter().any(|c| floating_classes.contains(&c.as_ref())) {
                return true;
            }
        }

        let float_types: Vec<&str> = AUTO_FLOAT_WINDOW_TYPES.iter().map(|a| a.as_ref()).collect();
        if let Ok(Prop::Atom(atoms)) = self.get_prop(id, Atom::NetWmWindowType.as_ref()) {
            atoms.iter().any(|a| float_types.contains(&a.as_ref()))
        } else {
            false
        }
    }

    /// Check to see if this client is one that we should be handling or not
    #[tracing::instrument(level = "trace", skip(self))]
    fn is_managed_client(&self, id: Xid) -> bool {
        if self.get_prop(id, Atom::WmTransientFor.as_ref()).is_ok() {
            trace!("window is transient: don't manage");
            return false;
        }

        if let Ok(Prop::Atom(types)) = self.get_prop(id, Atom::NetWmWindowType.as_ref()) {
            let unmanaged_types: Vec<String> = UNMANAGED_WINDOW_TYPES
                .iter()
                .map(|t| t.as_ref().to_string())
                .collect();
            trace!(ty = ?types, "checking window type to see we should manage");
            return types.iter().all(|ty| !unmanaged_types.contains(ty));
        }

        trace!("unable to find type: defaulting to manage");
        return true;
    }

    /// The subset of active clients that are considered managed by penrose
    fn active_managed_clients(&self) -> Result<Vec<Xid>> {
        Ok(self
            .active_clients()?
            .into_iter()
            .filter(|&id| {
                let attrs_ok = self.get_window_attributes(id).map_or(true, |a| {
                    !a.override_redirect && a.map_state == MapState::Viewable
                });
                attrs_ok && self.is_managed_client(id)
            })
            .collect())
    }
}

#[cfg(test)]
pub use mock_conn::MockXConn;

#[cfg(test)]
mod mock_conn {
    use super::*;
    use std::{cell::Cell, fmt};

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct MockXConn {
        screens: Vec<Screen>,
        #[cfg_attr(feature = "serde", serde(skip))]
        events: Cell<Vec<XEvent>>,
        focused: Cell<Xid>,
        unmanaged_ids: Vec<Xid>,
    }

    impl fmt::Debug for MockXConn {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("MockXConn")
                .field("screens", &self.screens)
                .field("remaining_events", &self.remaining_events())
                .field("focused", &self.focused.get())
                .field("unmanaged_ids", &self.unmanaged_ids)
                .finish()
        }
    }

    impl MockXConn {
        /// Set up a new [MockXConn] with pre-defined [Screen]s and an event stream to pull from
        pub fn new(screens: Vec<Screen>, events: Vec<XEvent>, unmanaged_ids: Vec<Xid>) -> Self {
            MockXConn {
                screens,
                events: Cell::new(events),
                focused: Cell::new(0),
                unmanaged_ids,
            }
        }

        fn remaining_events(&self) -> Vec<XEvent> {
            let remaining = self.events.replace(vec![]);
            self.events.set(remaining.clone());
            remaining
        }
    }

    __impl_stub_xcon! {
        for MockXConn;

        atom_queries: {}
        client_properties: {
            fn mock_get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
                if name == Atom::WmName.as_ref() || name == Atom::NetWmName.as_ref() {
                    Ok(Prop::UTF8String(vec!["mock name".into()]))
                } else {
                    Err(XError::MissingProperty(name.into(), id))
                }
            }
        }
        client_handler: {
            fn mock_focus_client(&self, id: Xid) -> Result<()> {
                self.focused.replace(id);
                Ok(())
            }
        }
        client_config: {}
        event_handler: {
            fn mock_wait_for_event(&self) -> Result<XEvent> {
                let mut remaining = self.events.replace(vec![]);
                if remaining.is_empty() {
                    return Err(XError::ConnectionClosed)
                }
                let next = remaining.remove(0);
                self.events.set(remaining);
                Ok(next)
            }
        }
        state: {
            fn mock_current_screens(&self) -> Result<Vec<Screen>> {
                Ok(self.screens.clone())
            }

            fn mock_focused_client(&self) -> Result<Xid> {
                Ok(self.focused.get())
            }
        }
        conn: {
            fn mock_is_managed_client(&self, id: Xid) -> bool {
                !self.unmanaged_ids.contains(&id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    struct WmNameXConn {
        wm_name: bool,
        net_wm_name: bool,
        empty_net_wm_name: bool,
    }

    impl StubXClientProperties for WmNameXConn {
        fn mock_get_prop(&self, id: Xid, name: &str) -> Result<Prop> {
            match Atom::from_str(name)? {
                Atom::WmName if self.wm_name => Ok(Prop::UTF8String(vec!["wm_name".into()])),
                Atom::WmName if self.net_wm_name && self.empty_net_wm_name => {
                    Ok(Prop::UTF8String(vec!["".into()]))
                }
                Atom::NetWmName if self.net_wm_name => {
                    Ok(Prop::UTF8String(vec!["net_wm_name".into()]))
                }
                Atom::NetWmName if self.empty_net_wm_name => Ok(Prop::UTF8String(vec!["".into()])),
                _ => Err(XError::MissingProperty(name.into(), id)),
            }
        }
    }

    test_cases! {
        window_name;
        args: (wm_name: bool, net_wm_name: bool, empty_net_wm_name: bool, expected: &str);

        case: wm_name_only => (true, false, false, "wm_name");
        case: net_wm_name_only => (false, true, false, "net_wm_name");
        case: both_prefers_net => (true, true, false, "net_wm_name");
        case: net_wm_name_empty => (true, false, true, "wm_name");

        body: {
            let conn = WmNameXConn {
                wm_name,
                net_wm_name,
                empty_net_wm_name,
            };
            assert_eq!(&conn.client_name(42).unwrap(), expected);
        }
    }
}
