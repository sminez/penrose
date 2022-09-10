//! User facing configuration of the penrose [WindowManager][crate::core::manager::WindowManager].
use crate::{
    core::layout::{side_stack, Layout, LayoutConf},
    draw::{self, Color},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

crate::__with_builder_and_getters! {
    /// The main user facing configuration details.
    ///
    /// See [ConfigBuilder] for details of what can be overwritten.
    ///
    /// # Example
    /// ```
    /// use penrose::{Config, draw::Color};
    /// use std::convert::TryFrom;
    ///
    /// let config = Config::default();
    ///
    /// assert_eq!(config.border_px(), &2);
    /// assert_eq!(config.focused_border(), &Color::try_from("#cc241d").unwrap());
    /// ```
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Clone, Debug, PartialEq)]
    Config;

    /// Builder struct for generating user [Config]
    ///
    /// # Example
    /// ```
    /// use penrose::core::{config::Config, layout::{LayoutConf, Layout, side_stack, monocle}};
    ///
    /// fn my_layouts() -> Vec<Layout> {
    ///     let mono_conf = LayoutConf {
    ///         follow_focus: true,
    ///         gapless: true,
    ///         ..Default::default()
    ///     };
    ///     let n_main = 1;
    ///     let ratio = 0.6;
    ///
    ///     vec![
    ///         Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
    ///         Layout::new("[mono]", mono_conf, monocle, n_main, ratio),
    ///     ]
    /// }
    ///
    /// let mut config_builder = Config::default().builder();
    /// let config = config_builder
    ///     .floating_classes(vec!["rofi", "dmenu", "dunst", "pinentry-gtk-2"])
    ///     .layouts(my_layouts())
    ///     .border_px(4)
    ///     .focused_border("#ebdbb2")
    ///     .unwrap()
    ///     .build()
    ///     .expect("failed to build config");
    /// ```
    #[derive(Debug)]
    ConfigBuilder;

    /// the initial available workspaces.
    ///
    /// # Constraints
    /// You must provide at least one workspace per screen
    VecImplInto workspaces: String; => vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"];

    /// the window classes that will always be considered floating
    VecImplInto floating_classes: String; => vec!["dmenu", "dunst"];

    /// the [Layout] functions to be used by each [Workspace][crate::core::workspace::Workspace]
    ///
    /// # Constraints
    /// You must provide at least one layout function
    Concrete layouts: Vec<Layout>; =>
        vec![
            Layout::new("[side]", LayoutConf::default(), side_stack, 1, 0.6),
            Layout::floating("[----]"),
        ];

    /// the focused border color as a hex literal
    ImplTry draw::Error; focused_border: Color; => "#cc241d";
    /// the unfocused border color as a hex literal
    ImplTry draw::Error; unfocused_border: Color; => "#3c3836";
    /// the border width of each window in pixels
    Concrete border_px: u32; => 2;
    /// the gap between tiled windows in pixels
    Concrete gap_px: u32; => 5;
    /// the percentage of the screen to grow the main region by when incrementing
    Concrete main_ratio_step: f32; => 0.05;
    /// whether or not space should be reserved for a status bar
    Concrete show_bar: bool; => true;
    /// whether or not the reserved space for a status bar is at the top of the sceen
    Concrete top_bar: bool; => true;
    /// the height of the space to be reserved for a status bar in pixels
    Concrete bar_height: u32; => 18;
}

impl Config {
    /// Create a range from 1 -> n_workspaces for use in keybindings
    pub fn ws_range(&self) -> std::ops::Range<usize> {
        1..(self.workspaces.len() + 1)
    }
}

impl ConfigBuilder {
    fn validate(&self) -> std::result::Result<(), String> {
        if self.inner.workspaces.is_empty() {
            return Err("Must supply at least one workspace name".into());
        }

        if self.inner.layouts.is_empty() {
            return Err("Must supply at least one layout function".into());
        }

        if !(0.0..=1.0).contains(&self.inner.main_ratio_step) {
            return Err("main_ratio_step must be in the range 0.0 -> 1.0".into());
        }

        Ok(())
    }
}
