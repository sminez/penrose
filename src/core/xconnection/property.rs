//! Data types for working with X window properties
use crate::{
    core::{
        data_types::{Point, Region},
        xconnection::Xid,
    },
    PenroseError, Result,
};

/// Know property types that should be returnable by XConn impls when they check
/// window properties.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Prop {
    /// One or more X Atoms
    Atom(Vec<String>),
    /// Raw bytes for when the prop type is non-standard
    Bytes(Vec<u32>),
    /// A cardinal number
    Cardinal(u32),
    /// UTF-8 encoded string data
    UTF8String(Vec<String>),
    /// An X window IDs
    Window(Vec<Xid>),
    /// The WmHints properties for this window
    WmHints(WmHints),
    /// The WmNormalHints properties for this window
    WmNormalHints(WmNormalHints),
}

bitflags! {
    /// Possible flags that can be set in a WmHints client property
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Default)]
    pub struct WmHintsFlags: u32 {
        /// Input hint is set
        const INPUT_HINT         = 0b0000000001;
        /// State hint is set
        const STATE_HINT         = 0b0000000010;
        /// Icon pixmap hint is set
        const ICON_PIXMAP_HINT   = 0b0000000100;
        /// Icon window hint is set
        const ICON_WINDOW_HINT   = 0b0000001000;
        /// Icon position hint is set
        const ICON_POSITION_HINT = 0b0000010000;
        /// Icon mask hint is set
        const ICON_MASK_HINT     = 0b0000100000;
        /// Window group hint is set
        const WINDOW_GROUP_HINT  = 0b0001000000;
        // unused                  0b0010000000;
        /// Urgency hint is set
        const URGENCY_HINT       = 0b0100000000;
    }
}

bitflags! {
    /// Possible flags that can be set in a WmNormalHints client property
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Default)]
    pub struct WmNormalHintsFlags: u32 {
        /// User-specified x, y
        const U_POSITION    = 0b0000000001;
        /// User-specified width, height
        const U_SIZE        = 0b0000000010;
        /// Program-specified position
        const P_POSITION    = 0b0000000100;
        /// Program-specified size
        const P_SIZE        = 0b0000001000;
        /// Program-specified minimum size
        const P_MIN_SIZE    = 0b0000010000;
        /// Program-specified maximum size
        const P_MAX_SIZE    = 0b0000100000;
        /// Program-specified resize increments
        const P_RESIZE_INC  = 0b0001000000;
        /// Program-specified min and max aspect ratios
        const P_ASPECT      = 0b0010000000;
        /// Program-specified base size
        const P_BASE_SIZE   = 0b0100000000;
        /// Program-specified window gravity
        const P_WIN_GRAVITY = 0b1000000000;
    }
}

/// Possible valid values for setting the `WM_STATE` property on a client.
///
/// See the [ICCCM docs][1] for more information.
///
/// [1]: https://tronche.com/gui/x/icccm/sec-4.html#s-4.1.3.1
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum WindowState {
    /// Window is not visible
    Withdrawn,
    /// Window is visible
    Normal,
    /// Window is iconified
    Iconic,
}

/// The mapping states a window can be in
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum MapState {
    /// The window is unmapped
    Unmapped,
    /// The window is never viewable
    UnViewable,
    /// The window is currently viewable
    Viewable,
}

/// The input class for a window
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum WindowClass {
    /// Class is copied from parent window
    CopyFromParent,
    /// Window can be displayed
    InputOutput,
    /// Window can only be used for queries
    InputOnly,
}

/// Client requested hints about information other than window geometry.
///
/// See the ICCCM [spec][1] for further details.
///
/// [1]: https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#wm_hints_property
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WmHints {
    pub(crate) flags: WmHintsFlags,
    pub(crate) accepts_input: bool,
    pub(crate) initial_state: WindowState,
    pub(crate) icon_pixmap: u32,
    pub(crate) icon_win: Xid,
    pub(crate) icon_position: Point,
    pub(crate) icon_mask: u32,
    pub(crate) window_group: u32,
}

impl WmHints {
    /// Create a new instance from component parts
    pub fn new(
        flags: WmHintsFlags,
        accepts_input: bool,
        initial_state: WindowState,
        icon_pixmap: u32,
        icon_win: Xid,
        icon_position: Point,
        icon_mask: u32,
        window_group: u32,
    ) -> Self {
        Self {
            flags,
            accepts_input,
            initial_state,
            icon_pixmap,
            icon_win,
            icon_position,
            icon_mask,
            window_group,
        }
    }

    /// Try to construct a [WmHints] instance from raw bytes.
    ///
    /// This method expects a slice of 9 u32s corresponding to the C struct layout shown below.
    ///
    /// ```C
    /// typedef struct {
    ///     long flags;          /* marks which fields in this structure are defined */
    ///     Bool input;          /* does this application rely on the window manager to
    ///                             get keyboard input? */
    ///     int initial_state;   /* see below */
    ///     Pixmap icon_pixmap;  /* pixmap to be used as icon */
    ///     Window icon_window;  /* window to be used as icon */
    ///     int icon_x, icon_y;  /* initial position of icon */
    ///     Pixmap icon_mask;    /* pixmap to be used as mask for icon_pixmap */
    ///     XID window_group;    /* id of related window group */
    ///     /* this structure may be extended in the future */
    /// } XWMHints;
    /// ```
    pub fn try_from_bytes(raw: &[u32]) -> Result<Self> {
        if raw.len() != 9 {
            return Err(PenroseError::InvalidHints(format!(
                "raw bytes should be [u32; 9] for WmHints, got [u32; {}]",
                raw.len()
            )));
        }

        let flags = WmHintsFlags::from_bits(raw[0]).unwrap();
        let accepts_input = !flags.contains(WmHintsFlags::INPUT_HINT) || raw[1] > 0;
        let initial_state = match (flags.contains(WmHintsFlags::STATE_HINT), raw[2]) {
            (true, 0) => WindowState::Withdrawn,
            (true, 1) | (false, _) => WindowState::Normal,
            (true, 2) => WindowState::Iconic,
            _ => {
                return Err(PenroseError::InvalidHints(format!(
                    "initial state flag should be 0, 1, 2: got {}",
                    raw[2]
                )))
            }
        };

        Ok(Self {
            flags,
            accepts_input,
            initial_state,
            icon_pixmap: raw[3],
            icon_win: raw[4],
            icon_position: Point::new(raw[5], raw[6]),
            icon_mask: raw[7],
            window_group: raw[8],
        })
    }
}

/// Client requested hints about window geometry.
///
/// See the ICCCM [spec][1] for further details or the [Xlib manual][2] for more details of the
/// data fromat but note that Penrose does not honour the following hints:
///   - gravity
///   - increment
///   - aspect ratio
///
/// [1]: https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#wm_normal_hints_property
/// [2]: https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/wm-normal-hints.html
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WmNormalHints {
    pub(crate) flags: WmNormalHintsFlags,
    pub(crate) base: Option<Region>,
    pub(crate) min: Option<Region>,
    pub(crate) max: Option<Region>,
    pub(crate) user_specified: Option<Region>,
}

impl WmNormalHints {
    /// Create a new instance from component parts
    pub fn new(
        flags: WmNormalHintsFlags,
        base: Option<Region>,
        min: Option<Region>,
        max: Option<Region>,
        user_specified: Option<Region>,
    ) -> Self {
        Self {
            flags,
            base,
            min,
            max,
            user_specified,
        }
    }

    /// Try to construct a [WmNormalHints] instance from raw bytes.
    ///
    /// This method expects a slice of 18 u32s corresponding to the C struct layout shown below.
    ///
    /// ```C
    /// typedef struct {
    ///     long flags;                /* marks which fields in this structure are defined */
    ///     int x, y;                  /* Obsolete */
    ///     int width, height;         /* Obsolete */
    ///     int min_width, min_height;
    ///     int max_width, max_height;
    ///     int width_inc, height_inc;
    ///     struct {
    ///            int x;              /* numerator */
    ///            int y;              /* denominator */
    ///     } min_aspect, max_aspect;
    ///     int base_width, base_height;
    ///     int win_gravity;
    ///     /* this structure may be extended in the future */
    /// } XSizeHints;
    /// ```
    pub fn try_from_bytes(raw: &[u32]) -> Result<Self> {
        if raw.len() != 18 {
            return Err(PenroseError::InvalidHints(format!(
                "raw bytes should be [u32; 18] for WmNormalHints, got [u32; {}]",
                raw.len()
            )));
        }

        let flags = WmNormalHintsFlags::from_bits(raw[0]).unwrap();

        // These properties are marked as obsolete but some clients still set them
        // so it they are useful as fallbacks
        let (x, y) = (raw[1], raw[2]);
        let (user_w, user_h) = (raw[3], raw[4]);

        let (min_w, min_h) = (raw[5], raw[6]);
        let (max_w, max_h) = (raw[7], raw[8]);
        let (base_w, base_h) = (raw[15], raw[16]);

        // ignoring increment, aspect ratio, gravity as they are not used in
        // the main WindowManager logic

        let if_set = |x, y, w, h| {
            if w > 0 && h > 0 {
                Some(Region::new(x, y, w, h))
            } else {
                None
            }
        };

        Ok(Self {
            flags,
            base: if_set(x, y, base_w, base_h),
            min: if_set(x, y, min_w, min_h),
            max: if_set(x, y, max_w, max_h),
            user_specified: if_set(x, y, user_w, user_h),
        })
    }
}

/// Window Attributes honoured by penose.
///
/// Only a small subset of window attributes are checked and honoured by penrose. This list may be
/// extended in future.
///
/// ```C
/// typedef struct xcb_get_window_attributes_reply_t {
///     uint8_t        response_type;
///     uint8_t        backing_store;
///     uint16_t       sequence;
///     uint32_t       length;
///     xcb_visualid_t visual;
///     uint16_t       _class;
///     uint8_t        bit_gravity;
///     uint8_t        win_gravity;
///     uint32_t       backing_planes;
///     uint32_t       backing_pixel;
///     uint8_t        save_under;
///     uint8_t        map_is_installed;
///     uint8_t        map_state;
///     uint8_t        override_redirect;
///     xcb_colormap_t colormap;
///     uint32_t       all_event_masks;
///     uint32_t       your_event_mask;
///     uint16_t       do_not_propagate_mask;
///     uint8_t        pad0[2];
/// } xcb_get_window_attributes_reply_t;
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WindowAttributes {
    pub(crate) override_redirect: bool,
    pub(crate) map_state: MapState,
    pub(crate) window_class: WindowClass,
}

impl WindowAttributes {
    /// Create a new instance from component parts
    pub fn new(override_redirect: bool, map_state: MapState, window_class: WindowClass) -> Self {
        Self {
            override_redirect,
            map_state,
            window_class,
        }
    }
}
