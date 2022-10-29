//! An abstraciton layer for talking to an underlying X server.
use crate::{
    bindings::{KeyCode, KeyPress, MouseState},
    // core::{client::Client, screen::Screen},
    geometry::{Point, Rect},
    stack_set::Screen,
    Xid,
};
use penrose_proc::stubbed_companion_trait;
use tracing::trace;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod atom;
pub mod event;
pub mod property;

pub use atom::*;
pub use event::*;
pub use property::*;

const WM_NAME: &str = "penrose";

/// Enum to store the various ways that operations can fail in X traits
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The underlying connection to the X server is closed
    #[error("The underlying connection to the X server is closed")]
    ConnectionClosed,

    /// Client data was malformed
    #[error("Invalid client message format: {0} (expected 8, 16 or 32)")]
    InvalidClientMessageData(u8),

    /// Wm(Normal)Hints received from the X server were invalid
    #[error("Invalid window hints property: {0}")]
    InvalidHints(String),

    /// The requested property is not set for the given client
    #[error("The {0} property is not set for client {1}")]
    MissingProperty(String, Xid),

    /// A generic error type for use in user code when needing to construct
    /// a simple [Error].
    #[error("{0}")]
    Raw(String),

    /// Parsing an [Atom][crate::core::xconnection::Atom] from a str failed.
    ///
    /// This happens when the atom name being requested is not a known atom.
    #[error("{0}")]
    Strum(#[from] strum::ParseError),

    /// An attempt was made to reference an atom that is not known to penrose
    #[error("{0} is not a known atom")]
    UnknownAtom(Xid),

    /// An attempt was made to reference a client that is not known to penrose
    #[error("{0} is not a known client")]
    UnknownClient(Xid),
}

/// Result type for errors raised by X traits
pub type Result<T> = std::result::Result<T, Error>;

/// A window type to be specified when creating a new window in the X server
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WinType {
    /// A simple hidden stub window for facilitating other API calls
    CheckWin,
    /// A window that receives input only (not queryable)
    InputOnly,
    /// A regular window. The [Atom] passed should be a
    /// valid _NET_WM_WINDOW_TYPE (this is not enforced)
    InputOutput(Atom),
}

/// On screen configuration options for X clients (not all are curently implemented)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClientConfig {
    /// The border width in pixels
    BorderPx(u32),
    /// Absolute size and position on the screen as a [Rect]
    Position(Rect),
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    #[stub(Err(Error::Raw("mocked".into())))]
    fn atom_name(&self, atom: Xid) -> Result<String>;

    /// Fetch or intern an atom by name
    #[stub(Err(Error::Raw("mocked".into())))]
    fn atom_id(&self, name: &str) -> Result<Xid>;
}

/// State queries against the running X server
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XState: XAtomQuerier {
    /// The root window ID
    #[stub(Xid(42))]
    fn root(&self) -> Xid;

    /// Determine the currently connected [screens][Screen] and return their details
    #[stub(Ok(vec![]))]
    fn current_screens(&self) -> Result<Vec<Screen<Xid>>>;

    /// Determine the current (x,y) position of the cursor relative to the root window.
    #[stub(Ok(Point::default()))]
    fn cursor_position(&self) -> Result<Point>;

    /// Warp the cursor to be within the specified window. If id == None then behaviour is
    /// definined by the implementor (e.g. warp cursor to active window, warp to center of screen)
    #[stub(Ok(()))]
    fn warp_cursor(&self, win_id: Option<Xid>, screen: &Screen<Xid>) -> Result<()>;

    /// Return the current (x, y, w, h) dimensions of the requested window
    #[stub(Ok(Rect::default()))]
    fn client_geometry(&self, id: Xid) -> Result<Rect>;

    /// Run on startup/restart to determine already running windows that we need to track
    #[stub(Ok(vec![]))]
    fn active_clients(&self) -> Result<Vec<Xid>>;

    /// Return the client ID of the [crate::core::client::Client] that currently holds X focus
    #[stub(Ok(Xid(0)))]
    fn focused_client(&self) -> Result<Xid>;
}

/// Sending and receiving X events
#[stubbed_companion_trait(doc_hidden = "true")]
pub trait XEventHandler {
    /// Flush pending actions to the X event loop
    #[stub(true)]
    fn flush(&self) -> bool;

    /// Wait for the next event from the X server and return it as an [XEvent]
    #[stub(Err(Error::Raw("mocked".into())))]
    fn wait_for_event(&self) -> Result<XEvent>;

    /// Send an X event to the target client
    ///
    /// The `msg` being sent can be composed by hand or, for known common message types, generated
    /// using the [build_client_event][1] method.
    ///
    /// [1]: XEventHandler::build_client_event
    #[stub(Err(Error::Raw("mocked".into())))]
    fn send_client_event(&self, msg: ClientMessage) -> Result<()>;

    /// Build the required event data for sending a known client event.
    #[stub(Err(Error::Raw("mocked".into())))]
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

    /// Destroy an existing client.
    #[stub(Ok(()))]
    fn destroy_client(&self, id: Xid) -> Result<()>;

    /// Forcably kill an existing client.
    #[stub(Ok(()))]
    fn kill_client(&self, id: Xid) -> Result<()>;

    /// Mark the given client as having focus
    #[stub(Ok(()))]
    fn focus_client(&self, id: Xid) -> Result<()>;

    // /// Map a known penrose [Client] if it is not currently visible
    // fn map_client_if_needed(&self, win: Option<&mut Client>) -> Result<()> {
    //     if let Some(c) = win {
    //         if !c.mapped {
    //             c.mapped = true;
    //             self.map_client(c.id())?;
    //         }
    //     }
    //     Ok(())
    // }

    // /// Unmap a known penrose [Client] if it is currently visible
    // fn unmap_client_if_needed(&self, win: Option<&mut Client>) -> Result<()> {
    //     if let Some(c) = win {
    //         if c.mapped {
    //             c.mapped = false;
    //             self.unmap_client(c.id())?;
    //         }
    //     }
    //     Ok(())
    // }
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
    #[stub(Err(Error::Raw("mocked".into())))]
    fn get_prop(&self, id: Xid, name: &str) -> Result<Prop>;

    /// Delete an existing property from a client
    #[stub(Ok(()))]
    fn delete_prop(&self, id: Xid, name: &str) -> Result<()>;

    /// Change an existing property for a client
    #[stub(Ok(()))]
    fn change_prop(&self, id: Xid, name: &str, val: Prop) -> Result<()>;

    /// Update a client's `WM_STATE` property to the given value.
    ///
    /// See the [ICCCM docs][1] for more information on what each value means for the client.
    ///
    /// [1]: https://tronche.com/gui/x/icccm/sec-4.html#s-4.1.3.1
    #[stub(Ok(()))]
    fn set_client_state(&self, id: Xid, wm_state: WindowState) -> Result<()>;

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
            Ok(p) => Err(Error::Raw(format!("Expected atoms, got {:?}", p))),
            Err(Error::MissingProperty(_, _)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Check to see if a given client accepts input focus
    fn client_accepts_focus(&self, id: Xid) -> bool {
        match self.get_prop(id, Atom::WmHints.as_ref()) {
            Ok(Prop::WmHints(WmHints { accepts_input, .. })) => accepts_input,
            _ => true,
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

    /// Determine whether the target client should be tiled or allowed to float
    fn client_should_float(&self, id: Xid, floating_classes: &[&str]) -> bool {
        if let Ok(prop) = self.get_prop(id, Atom::WmTransientFor.as_ref()) {
            trace!(?prop, "window is transient: setting to floating state");
            return true;
        }

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
    #[stub(Err(Error::Raw("mocked".into())))]
    fn get_window_attributes(&self, id: Xid) -> Result<WindowAttributes>;

    /*
     *  The following default implementations should used if possible.
     *
     *  Any custom implementations should take care to ensure that the state changes being made are
     *  equivaled to those implemented here.
     */

    /// Reposition the window identified by 'id' to the specifed region
    fn position_client(&self, id: Xid, r: Rect, border: u32, stack_above: bool) -> Result<()> {
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
    fn set_client_border_color(&self, id: Xid, color: u32) -> Result<()> {
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color)])
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
    #[stub(Err(Error::Raw("mocked".into())))]
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
    #[stub(Xid(0))]
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
    fn grab_keys(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()>;

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

    // /// Check to see if this client is one that we should be handling or not
    // #[tracing::instrument(level = "trace", skip(self))]
    // fn is_managed_client(&self, c: &Client) -> bool {
    //     let unmanaged_types: Vec<String> = UNMANAGED_WINDOW_TYPES
    //         .iter()
    //         .map(|t| t.as_ref().to_string())
    //         .collect();
    //     trace!(ty = ?c.wm_type, "checking window type to see we should manage");
    //     return c.wm_type.iter().all(|ty| !unmanaged_types.contains(ty));
    // }

    // /// The subset of active clients that are considered managed by penrose
    // fn active_managed_clients(&self, floating_classes: &[&str]) -> Result<Vec<Client>> {
    //     Ok(self
    //         .active_clients()?
    //         .into_iter()
    //         .filter_map(|id| {
    //             let attrs_ok = self.get_window_attributes(id).map_or(true, |a| {
    //                 !a.override_redirect
    //                     && a.window_class == WindowClass::InputOutput
    //                     && a.map_state == MapState::Viewable
    //             });
    //             if attrs_ok {
    //                 trace!(%id, "parsing existing client");
    //                 let wix = match self.get_prop(id, Atom::NetWmDesktop.as_ref()) {
    //                     Ok(Prop::Cardinal(wix)) => wix,
    //                     _ => 0, // Drop unknown clients onto ws 0 as we know that is always there
    //                 };

    //                 let c = Client::new(self, id, wix as usize, floating_classes);
    //                 if self.is_managed_client(&c) {
    //                     return Some(c);
    //                 }
    //             }
    //             None
    //         })
    //         .collect())
    // }
}
