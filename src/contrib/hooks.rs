//! Additional common hooks that can be used out of the box with minimal config.
use crate::core::{
    client::Client, helpers::spawn, hooks::Hook, manager::WindowManager, ring::Selector,
    xconnection::XConn,
};
use std::collections::HashMap;

/**
 * Automatically set the X root window WM_NAME property to be the WM_NAME of the
 * active window.
 *
 * If WM_NAME is not set for the active window, then a default value of 'n/a' is set instead. This
 * is intended for use with external programs such as Polybar as a way of exposing state.
 * NOTE: currently, WM_NAME is read when the window is first mapped only.
 */
#[derive(Clone, Copy, Debug)]
pub struct ActiveClientAsRootName {}

impl ActiveClientAsRootName {
    /// Construct a pre-boxed instance of the ActiveClientAsRootName hook
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl<X: XConn> Hook<X> for ActiveClientAsRootName {
    fn new_client(&mut self, wm: &mut WindowManager<X>, c: &mut Client) {
        wm.set_root_window_name(c.wm_name());
    }
}

/**
 * Automatically set the X root window WM_NAME property to be the current layout symbol for the
 * active workspace.
 *
 * This is intended for use with external programs such as Polybar as a way of exposing state.
 */
#[derive(Clone, Copy, Debug)]
pub struct LayoutSymbolAsRootName {}

impl LayoutSymbolAsRootName {
    /// Construct a pre-boxed instance of the LayoutSymbolAsRootName hook
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl<X: XConn> Hook<X> for LayoutSymbolAsRootName {
    fn layout_change(&mut self, wm: &mut WindowManager<X>, _: usize, _: usize) {
        wm.set_root_window_name(wm.current_layout_symbol());
    }
}

/**
 * Whenever a focus moves to the workspace 'name' and the workspace is empty,
 * set a specific layout and spawn a set of default clients.
 *
 * The layout is set first and then clients are spawned in the order they are defined using the
 * penrose::core::helpers::spawn function. This means that the final client will have focus and the
 * the clients will be arranged based on the order they are spawned.
 */
#[derive(Clone, Debug)]
pub struct DefaultWorkspace {
    defaults: Vec<String>,
    layout: String,
    name: String,
}

impl DefaultWorkspace {
    /// Create a new DefaultWorkspace that is pre-boxed for adding to your workspace hooks
    pub fn new(
        name: impl Into<String>,
        layout: impl Into<String>,
        defaults: Vec<impl Into<String>>,
    ) -> Box<Self> {
        Box::new(Self {
            name: name.into(),
            layout: layout.into(),
            defaults: defaults.into_iter().map(|d| d.into()).collect(),
        })
    }
}

impl<X: XConn> Hook<X> for DefaultWorkspace {
    fn workspace_change(&mut self, wm: &mut WindowManager<X>, _: usize, new: usize) {
        if let Some(ws) = wm.workspace_mut(&Selector::Index(new)) {
            if ws.name() == self.name && ws.is_empty() {
                // can fail if the layout symbol is wrong
                ws.try_set_layout(&self.layout);
                self.defaults.iter().for_each(spawn);
            }
        }
    }
}

/**
 * Automatically remove empty workspaces when they lose focus.
 *
 * Workspaces with names in 'protected' will not be auto-removed when empty so that you can
 * maintain a set of default workspaces that are always available. This hook is most useful when
 * combined with `DefaultWorkspace` to provide a set of ephemeral workspace configurations that can
 * be created on demand.
 */
#[derive(Clone, Debug)]
pub struct RemoveEmptyWorkspaces {
    protected: Vec<String>,
}

impl RemoveEmptyWorkspaces {
    /// Create a new RemoveEmptyWorkspaces that is pre-boxed for adding to your workspace hooks.
    pub fn new(protected: Vec<impl Into<String>>) -> Box<Self> {
        Box::new(Self {
            protected: protected.into_iter().map(|d| d.into()).collect(),
        })
    }
}

impl<X: XConn> Hook<X> for RemoveEmptyWorkspaces {
    fn workspace_change(&mut self, wm: &mut WindowManager<X>, old: usize, _: usize) {
        let sel = Selector::Index(old);
        if let Some(ws) = wm.workspace(&sel) {
            if !self.protected.iter().any(|p| p == ws.name()) && ws.is_empty() {
                wm.remove_workspace(&sel);
            }
        };
    }
}

/// An individual workspace mapping for ClientSpawnRules
#[derive(Clone, Debug)]
pub enum SpawnRule<'a> {
    /// Target a client by WM_CLASS
    ClassName(&'a str, usize),
    /// Target a client by WM_NAME
    WMName(&'a str, usize),
}

/**
 * Move clients with a matching WM_NAME to a target workspace when they are spawned.
 *
 * The Strings used to identify the clients that should be moved are their WM_NAME
 * and WM_CLASS X11 properties.
 * ```
 * # #[macro_use] extern crate penrose; fn main() {
 * use penrose::contrib::hooks::{SpawnRule, ClientSpawnRules};
 *
 * let my_hook = ClientSpawnRules::new(vec![
 *     SpawnRule::ClassName("xterm-256color" , 3),
 *     SpawnRule::WMName("Firefox Developer Edition" , 7),
 * ]);
 * # }
 */
#[derive(Clone, Debug)]
pub struct ClientSpawnRules {
    class_rules: HashMap<String, usize>,
    name_rules: HashMap<String, usize>,
}

impl ClientSpawnRules {
    /// Create a new ClientSpawnRules that is pre-boxed for adding to your workspace hooks.
    pub fn new(rules: Vec<SpawnRule<'_>>) -> Box<Self> {
        let mut class_rules = HashMap::new();
        let mut name_rules = HashMap::new();

        for rule in rules.into_iter() {
            match rule {
                SpawnRule::ClassName(s, i) => class_rules.insert(s.into(), i),
                SpawnRule::WMName(s, i) => name_rules.insert(s.into(), i),
            };
        }

        Box::new(Self {
            class_rules,
            name_rules,
        })
    }
}

impl<X: XConn> Hook<X> for ClientSpawnRules {
    /// This sets the client workspace to the desired value which is then picked up and
    /// trigers the spawn on that workspace in WindowManager.handle_map_request
    fn new_client(&mut self, _: &mut WindowManager<X>, c: &mut Client) {
        if let Some(wix) = self.class_rules.get(c.wm_class()) {
            c.set_workspace(*wix);
        } else if let Some(wix) = self.name_rules.get(c.wm_name()) {
            c.set_workspace(*wix);
        }
    }
}
