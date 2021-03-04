//! Metadata around X clients and manipulating them
use crate::core::xconnection::{Atom, Prop, WmHints, WmNormalHints, XClientProperties, Xid};

/**
 * Meta-data around a client window that we are handling.
 *
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Client {
    pub(crate) id: Xid,
    pub(crate) workspace: usize,
    pub(crate) wm_name: String,
    pub(crate) wm_class: Vec<String>, // should always be two elements but that's not enforced?
    pub(crate) wm_type: Vec<String>,  // Can't use Atom as it could be something arbitrary
    pub(crate) wm_protocols: Vec<String>, // Can't use Atom as it could be something arbitrary
    pub(crate) wm_hints: Option<WmHints>,
    pub(crate) wm_normal_hints: Option<WmNormalHints>,
    // state flags
    pub(crate) accepts_focus: bool,
    pub(crate) floating: bool,
    pub(crate) fullscreen: bool,
    pub(crate) mapped: bool,
    pub(crate) urgent: bool,
    pub(crate) wm_managed: bool,
}

impl Client {
    /// Track a new client window on a specific workspace
    ///
    /// This uses the provided [`XClientProperties`] to query state from the X server about the
    /// client and cache that for later use. If any of the requests fail then we set defaults
    /// rather than erroring as we always need to be able to track clients when they are mapped.
    pub(crate) fn new<X>(conn: &X, id: Xid, workspace: usize, floating_classes: &[&str]) -> Self
    where
        X: XClientProperties,
    {
        // TODO: do we want error logging around setting defaults here?
        //       the xcb impl probably needs to catch BadAtom as "missing"?
        let floating = conn.client_should_float(id, floating_classes);
        let accepts_focus = conn.client_accepts_focus(id);
        let wm_name = conn.client_name(id).unwrap_or("unknown".into());

        let wm_class = match conn.get_prop(id, Atom::WmClass.as_ref()) {
            Ok(Prop::UTF8String(strs)) => strs,
            _ => vec![],
        };
        let wm_type = match conn.get_prop(id, Atom::NetWmWindowType.as_ref()) {
            Ok(Prop::Atom(atoms)) => atoms,
            _ => vec![Atom::NetWindowTypeNormal.as_ref().to_string()],
        };
        let wm_hints = match conn.get_prop(id, Atom::WmHints.as_ref()) {
            Ok(Prop::WmHints(hints)) => Some(hints),
            _ => None,
        };
        let wm_normal_hints = match conn.get_prop(id, Atom::WmNormalHints.as_ref()) {
            Ok(Prop::WmNormalHints(hints)) => Some(hints),
            _ => None,
        };
        let wm_protocols = match conn.get_prop(id, Atom::WmProtocols.as_ref()) {
            Ok(Prop::Atom(protocols)) => protocols,
            _ => vec![],
        };

        Self {
            id,
            workspace,
            wm_name,
            wm_class,
            wm_type,
            wm_protocols,
            wm_hints,
            wm_normal_hints,
            floating,
            accepts_focus,
            fullscreen: false,
            mapped: false,
            urgent: false,
            wm_managed: true,
        }
    }

    /// The X window ID of this client
    pub fn id(&self) -> Xid {
        self.id
    }

    /// The WM_CLASS property of this client
    pub fn wm_class(&self) -> &str {
        match self.wm_class.get(0) {
            Some(class) => class,
            None => "unknown",
        }
    }

    /// The WM_NAME property of this client
    pub fn wm_name(&self) -> &str {
        &self.wm_name
    }

    /// Whether or not this client is currently fullscreen
    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    /// The current workspace index that this client is showing on
    pub fn workspace(&self) -> usize {
        self.workspace
    }

    /// Mark this window as being on a new workspace
    pub fn set_workspace(&mut self, workspace: usize) {
        self.workspace = workspace
    }

    /// Set the floating state of this client
    pub fn set_floating(&mut self, floating: bool) {
        self.floating = floating
    }

    pub(crate) fn set_name(&mut self, name: impl Into<String>) {
        self.wm_name = name.into()
    }

    /// The WM_CLASS of the window that this Client is tracking
    pub fn class(&self) -> &str {
        match self.wm_class.get(0) {
            Some(class) => class,
            None => "unknown",
        }
    }

    /// Mark this client as not being managed by the WindowManager directly
    pub fn externally_managed(&mut self) {
        self.wm_managed = false;
    }

    /// Mark this client as being managed by the WindowManager directly
    pub fn internally_managed(&mut self) {
        self.wm_managed = true;
    }
}
