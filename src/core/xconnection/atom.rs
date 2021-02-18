//! Data types for working with X atoms
use strum::*;

/// A Penrose internal representation of X atoms.
///
/// Atom names are shared between all X11 API libraries so this enum allows us to get a little bit
/// of type safety around their use. Implementors of [XConn][1] should accept any variant of [Atom]
/// that they are passed by client code.
///
/// [1]: crate::core::xconnection::XConn
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(AsRefStr, EnumString, EnumIter, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Atom {
    /// ATOM
    #[strum(serialize = "ATOM")]
    Atom,
    /// ATOM_WINDOW
    #[strum(serialize = "ATOM_WINDOW")]
    Window,
    /// ATOM_CARDINAL
    #[strum(serialize = "ATOM_CARDINAL")]
    Cardinal,
    /// MANAGER
    #[strum(serialize = "MANAGER")]
    Manager,
    /// UTF8_STRING
    #[strum(serialize = "UTF8_STRING")]
    UTF8String,
    /// WM_CLASS
    #[strum(serialize = "WM_CLASS")]
    WmClass,
    /// WM_DELETE_WINDOW
    #[strum(serialize = "WM_DELETE_WINDOW")]
    WmDeleteWindow,
    /// WM_HINTS
    #[strum(serialize = "WM_HINTS")]
    WmHints,
    /// WM_NORMAL_HINTS
    #[strum(serialize = "WM_NORMAL_HINTS")]
    WmNormalHints,
    /// WM_PROTOCOLS
    #[strum(serialize = "WM_PROTOCOLS")]
    WmProtocols,
    /// WM_STATE
    #[strum(serialize = "WM_STATE")]
    WmState,
    /// WM_NAME
    #[strum(serialize = "WM_NAME")]
    WmName,
    /// WM_TRANSIENT_FOR
    #[strum(serialize = "WM_TRANSIENT_FOR")]
    WmTransientFor,
    /// WM_TAKE_FOCUS
    #[strum(serialize = "WM_TAKE_FOCUS")]
    WmTakeFocus,
    /// _NET_ACTIVE_WINDOW
    #[strum(serialize = "_NET_ACTIVE_WINDOW")]
    NetActiveWindow,
    /// _NET_CLIENT_LIST
    #[strum(serialize = "_NET_CLIENT_LIST")]
    NetClientList,
    /// _NET_CLIENT_LIST
    #[strum(serialize = "_NET_CLIENT_LIST_STACKING")]
    NetClientListStacking,
    /// _NET_CURRENT_DESKTOP
    #[strum(serialize = "_NET_CURRENT_DESKTOP")]
    NetCurrentDesktop,
    /// _NET_DESKTOP_NAMES
    #[strum(serialize = "_NET_DESKTOP_NAMES")]
    NetDesktopNames,
    /// _NET_NUMBER_OF_DESKTOPS
    #[strum(serialize = "_NET_NUMBER_OF_DESKTOPS")]
    NetNumberOfDesktops,
    /// _NET_SUPPORTED
    #[strum(serialize = "_NET_SUPPORTED")]
    NetSupported,
    /// _NET_SUPPORTING_WM_CHECK
    #[strum(serialize = "_NET_SUPPORTING_WM_CHECK")]
    NetSupportingWmCheck,
    /// _NET_SYSTEM_TRAY_OPCODE
    #[strum(serialize = "_NET_SYSTEM_TRAY_OPCODE")]
    NetSystemTrayOpcode,
    /// _NET_SYSTEM_TRAY_ORIENTATION
    #[strum(serialize = "_NET_SYSTEM_TRAY_ORIENTATION")]
    NetSystemTrayOrientation,
    /// _NET_SYSTEM_TRAY_ORIENTATION_HORZ
    #[strum(serialize = "_NET_SYSTEM_TRAY_ORIENTATION_HORZ")]
    NetSystemTrayOrientationHorz,
    /// _NET_SYSTEM_TRAY_S0
    #[strum(serialize = "_NET_SYSTEM_TRAY_S0")]
    NetSystemTrayS0,
    /// _NET_WM_DESKTOP
    #[strum(serialize = "_NET_WM_DESKTOP")]
    NetWmDesktop,
    /// _NET_WM_NAME
    #[strum(serialize = "_NET_WM_NAME")]
    NetWmName,
    /// _NET_WM_STATE
    #[strum(serialize = "_NET_WM_STATE")]
    NetWmState,
    /// _NET_WM_STATE_FULLSCREEN
    #[strum(serialize = "_NET_WM_STATE_FULLSCREEN")]
    NetWmStateFullscreen,
    /// _NET_WM_WINDOW_TYPE
    #[strum(serialize = "_NET_WM_WINDOW_TYPE")]
    NetWmWindowType,
    /// _XEMBED
    #[strum(serialize = "_XEMBED")]
    XEmbed,
    /// _XEMBED_INFO
    #[strum(serialize = "_XEMBED_INFO")]
    XEmbedInfo,

    // Window Types
    /// _NET_WM_WINDOW_TYPE_DESKTOP
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DESKTOP")]
    NetWindowTypeDesktop,
    /// _NET_WM_WINDOW_TYPE_DOCK
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DOCK")]
    NetWindowTypeDock,
    /// _NET_WM_WINDOW_TYPE_TOOLBAR
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_TOOLBAR")]
    NetWindowTypeToolbar,
    /// _NET_WM_WINDOW_TYPE_MENU
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_MENU")]
    NetWindowTypeMenu,
    /// _NET_WM_WINDOW_TYPE_UTILITY
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_UTILITY")]
    NetWindowTypeUtility,
    /// _NET_WM_WINDOW_TYPE_SPLASH
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_SPLASH")]
    NetWindowTypeSplash,
    /// _NET_WM_WINDOW_TYPE_DIALOG
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DIALOG")]
    NetWindowTypeDialog,
    /// _NET_WM_WINDOW_TYPE_DROPDOWN_MENU
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU")]
    NetWindowTypeDropdownMenu,
    /// _NET_WM_WINDOW_TYPE_POPUP_MENU
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_POPUP_MENU")]
    NetWindowTypePopupMenu,
    /// _NET_WM_WINDOW_TYPE_NOTIFICATION
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_NOTIFICATION")]
    NetWindowTypeNotification,
    /// _NET_WM_WINDOW_TYPE_COMBO
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_COMBO")]
    NetWindowTypeCombo,
    /// _NET_WM_WINDOW_TYPE_DND
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DND")]
    NetWindowTypeDnd,
    /// _NET_WM_WINDOW_TYPE_NORMAL
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_NORMAL")]
    NetWindowTypeNormal,
}

/// Clients with one of these window types will be auto floated
pub const AUTO_FLOAT_WINDOW_TYPES: &[Atom] = &[
    Atom::NetWindowTypeCombo,
    Atom::NetWindowTypeDesktop,
    Atom::NetWindowTypeDialog,
    Atom::NetWindowTypeDock,
    Atom::NetWindowTypeDropdownMenu,
    Atom::NetWindowTypeMenu,
    Atom::NetWindowTypeNotification,
    Atom::NetWindowTypePopupMenu,
    Atom::NetWindowTypeSplash,
    Atom::NetWindowTypeToolbar,
    Atom::NetWindowTypeUtility,
];

/// Windows with a type in this array will not be managed by penrose
pub const UNMANAGED_WINDOW_TYPES: &[Atom] = &[
    Atom::NetWindowTypeDock,
    Atom::NetWindowTypeNotification,
    Atom::NetWindowTypeToolbar,
    Atom::NetWindowTypeUtility,
];

/// Currently supported EWMH atoms
pub const EWMH_SUPPORTED_ATOMS: &[Atom] = &[
    Atom::NetActiveWindow,
    Atom::NetClientList,
    Atom::NetClientListStacking,
    Atom::NetCurrentDesktop,
    Atom::NetDesktopNames,
    Atom::NetNumberOfDesktops,
    Atom::NetSupported,
    Atom::NetSupportingWmCheck,
    // Atom::NetSystemTrayS0,
    // Atom::NetSystemTrayOpcode,
    // Atom::NetSystemTrayOrientationHorz,
    Atom::NetWmName,
    Atom::NetWmState,
    Atom::NetWmStateFullscreen,
    Atom::NetWmWindowType,
];
