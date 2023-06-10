//! A simple wrapper for suckless' [dmenu][1] tool for providing quick text based menus
//!
//! See [`DMenuKind`] for dmenu type support options.
use crate::{Color, Error, Result};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

/// The result of attempting to match against user input in a [DMenu]
#[derive(Debug, Clone)]
pub enum MenuMatch {
    /// The selected line along its line number (0 indexed)
    Line(usize, String),
    /// Nothing matched and this was the user's input when they hit Return
    UserInput(String),
    /// The user exited out of matching or had nothing typed
    NoMatch,
}

/// Two different derivatives of dmenu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMenuKind {
    /// Suckless's version of dmenu
    ///
    /// [1]: https://tools.suckless.org/dmenu/
    Suckless,
    /// Newer `dmenu-rs`
    ///
    /// [1]: https://github.com/Shizcow/dmenu-rs
    Rust,
}

/// Custom configuration options for [`DMenu`].
///
/// # Example
/// ```no_run
/// # use penrose::extensions::util::dmenu::*;
/// let dc = DMenuConfig {
///     show_line_numbers: true,
///     show_on_bottom: true,
///     password_input: true,
///     ignore_case: false,
///     n_lines: 0,
///     custom_font: Some("JetBrains Nerd Font Mono".to_owned()),
///     kind: DMenuKind::Rust,
///     custom_prompt: Some(" ".to_owned()),
///     ..DMenuConfig::default()
/// };
/// ```
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct DMenuConfig {
    /// Should line numbers be displayed to the user?
    ///
    /// Default: false
    pub show_line_numbers: bool,

    /// Show dmenu at the bottom the the screen.
    ///
    /// Default: false
    pub show_on_bottom: bool,

    /// Should dmenu treat the input as a password and render characters as '*'?
    ///
    /// NOTE: This requires the [Password][1] patch in order to work.
    /// or in the case of dmenu-rs it requires the password plugin.
    ///
    /// Default: false
    ///
    /// [1]: https://tools.suckless.org/dmenu/patches/password/
    /// [1]: https://github.com/Shizcow/dmenu-rs/tree/master/src/plugins
    pub password_input: bool,

    /// Should dmenu ignore case in the user input when matching?
    ///
    /// Default: false
    pub ignore_case: bool,

    /// Background color for the rendered window
    ///
    /// Default: #282828
    pub bg_color: Color,

    /// Foreground color for text
    ///
    /// Default: #ebdbb2
    pub fg_color: Color,

    /// Selected line background color
    ///
    /// Default: #458588
    pub selected_color: Color,

    /// Number of lines to display at a time.
    ///
    /// Setting n_lines=0 will result in the choices being displayed horizontally
    /// instead of vertically.
    ///
    /// Default: 10
    pub n_lines: u8,

    /// Allow the user to load a custom font
    ///
    /// Default: None
    pub custom_font: Option<String>,

    /// Specify to kind of dmenu to use
    ///
    /// Default: Suckless
    pub kind: DMenuKind,

    /// Optional prompt customization.
    ///
    /// Default: None
    pub custom_prompt: Option<String>,
}

impl Default for DMenuConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            show_on_bottom: false,
            password_input: false,
            ignore_case: false,
            bg_color: 0x282828ff.into(),
            fg_color: 0xebdbb2ff.into(),
            selected_color: 0x458588ff.into(),
            n_lines: 10,
            custom_font: None,
            kind: DMenuKind::Suckless,
            custom_prompt: None,
        }
    }
}

impl DMenuConfig {
    /// Construct a default config with a custom prompt
    pub fn with_prompt(prompt: &str) -> Self {
        Self {
            custom_prompt: Some(prompt.to_string()),
            ..Default::default()
        }
    }

    /// Build the dmenu flags
    fn flags(&self, screen_index: usize) -> Vec<String> {
        let Self {
            show_on_bottom,
            password_input,
            ignore_case,
            bg_color,
            fg_color,
            selected_color,
            n_lines,
            custom_font,
            kind,
            custom_prompt,
            ..
        } = self;

        // Only some command line options require the "--" for the rust version.
        let prefix = match kind {
            DMenuKind::Suckless => "-",
            DMenuKind::Rust => "--",
        };

        let mut flags = vec!["-m".to_owned(), screen_index.to_string()];

        flags.push(format!("{prefix}nb"));
        flags.push(bg_color.as_rgb_hex_string());

        flags.push(format!("{prefix}nf"));
        flags.push(fg_color.as_rgb_hex_string());

        flags.push(format!("{prefix}sb"));
        flags.push(selected_color.as_rgb_hex_string());

        if *n_lines > 0 {
            flags.push("-l".to_owned());
            flags.push(n_lines.to_string());
        }

        if *show_on_bottom {
            flags.push("-b".to_owned());
        }

        if *password_input {
            flags.push("-P".to_owned());
        }

        if *ignore_case {
            flags.push("-i".to_owned());
        }

        if let Some(font) = custom_font {
            flags.push(format!("{prefix}fn"));
            flags.push(font.to_owned());
        }

        if let Some(prompt) = custom_prompt {
            flags.push("-p".to_owned());
            flags.push(prompt.to_owned());
        }

        flags
    }
}

/// A wrapper around the suckless [dmenu][1] program for creating dynamic menus
/// in penrose.
#[derive(Debug, Clone)]
pub struct DMenu {
    /// Holds the custom dmenu configuration for this instance.
    config: DMenuConfig,
    /// The screen index this instance of dmenu will show up on
    screen_index: usize,
}

impl DMenu {
    /// Create a new [`DMenu`] command which can be triggered and reused by calling
    /// the `run` method for a basic dmenu prompt, or the `build_menu`
    /// for more advanced selection menus.
    pub fn new(config: &DMenuConfig, screen_index: usize) -> Self {
        Self {
            config: config.to_owned(),
            screen_index,
        }
    }

    /// Used for launching regular old [`DMenu`] with no menu matching
    /// via the `dmenu_run` wrapper script.
    pub fn run(&self) -> Result<()> {
        let args = self.config.flags(self.screen_index);
        let spawned_process = Command::new("dmenu_run").args(args).spawn();

        match spawned_process {
            Ok(mut process) => match process.wait() {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    /// Run this [`DMenu`] command and return the selected choice.
    ///
    /// # Example
    /// ```no_run
    /// # use penrose::extensions::util::dmenu::*;
    /// let screen_index = 0;
    /// let dmenu = DMenu::new(&DMenuConfig::default(), screen_index);
    ///
    /// let choices = vec!["some", "choices", "to", "pick", "from"];
    ///
    /// match dmenu.build_menu(choices).unwrap() {
    ///     MenuMatch::Line(i, s) => println!("matched '{}' on line '{}'", s, i),
    ///     MenuMatch::UserInput(s) => println!("user input: '{}'", s),
    ///     MenuMatch::NoMatch => println!("no match"),
    /// }
    /// ```
    // #[allow(clippy::pattern_type_mismatch)]
    pub fn build_menu(&self, param_choices: Vec<impl Into<String>>) -> Result<MenuMatch> {
        let choices: Vec<String> = param_choices
            .into_iter()
            .map(std::convert::Into::into)
            .collect();
        let raw = self.raw_user_choice_from_dmenu(&choices)?;
        let choice = raw.trim();

        if choice.is_empty() {
            return Ok(MenuMatch::NoMatch);
        }

        let res = choices
            .iter()
            .enumerate()
            .find(|(i, s)| {
                if self.config.show_line_numbers {
                    format!("{i:<3} {s}") == choice
                } else {
                    *s == choice
                }
            })
            .map_or_else(
                || MenuMatch::UserInput(choice.to_owned()),
                |(i, _)| {
                    MenuMatch::Line(
                        i,
                        choices.get(i).expect("Indexing choices panicked").clone(),
                    )
                },
            );

        Ok(res)
    }

    /// Get a vector of choices as bytes
    fn choices_as_input_bytes(&self, choices: &[String]) -> Vec<u8> {
        if self.config.show_line_numbers {
            choices
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{i:<3} {s}"))
                .collect::<Vec<String>>()
                .join("\n")
                .as_bytes()
                .to_vec()
        } else {
            choices.join("\n").as_bytes().to_vec()
        }
    }

    /// Launch a shell process with all arguments to dmenu
    fn raw_user_choice_from_dmenu(&self, choices: &[String]) -> Result<String> {
        let args = self.config.flags(self.screen_index);
        let mut proc = Command::new("dmenu")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(args)
            .spawn()?;

        {
            // Taking stdin here and dropping it when the block scope ends to close it and
            // let dmenu determine the end of input
            let mut stdin = proc
                .stdin
                .take()
                .ok_or_else(|| Error::Custom("unable to open stdin".to_owned()))?;

            stdin.write_all(&self.choices_as_input_bytes(choices))?;
        }

        let mut raw = String::new();
        proc.stdout
            .ok_or_else(|| Error::Custom("failed to spawn dmenu".to_owned()))?
            .read_to_string(&mut raw)?;

        Ok(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::{DMenuConfig, DMenuKind};

    /// Flags [ nb, nf, sb, and nf] need to be modified for the different
    /// versions of dmenu. Classic Suckless dmenu uses a single dash "-", dmenu-rs
    /// uses the more modern cli style of double dashes "--".
    /// This test depends on the order the flags are loaded into the array, so if the order
    /// is changed in the flags function, these tests will fail. There is a better way, but
    /// this works for now.
    #[test]
    fn dmenu_suckless_config_test() {
        let dc = DMenuConfig {
            custom_font: Some("mono".to_owned()),
            ..DMenuConfig::default()
        };

        // Should default to suckless c-style dmenu
        assert_eq!(dc.kind, DMenuKind::Suckless);
        let flags = dc.flags(0);

        for (i, flag) in flags.into_iter().enumerate() {
            if i == 2 {
                assert_eq!(flag, "-nb".to_owned());
            }
            if i == 4 {
                assert_eq!(flag, "-nf".to_owned());
            }
            if i == 6 {
                assert_eq!(flag, "-sb".to_owned());
            }
            if i == 10 {
                assert_eq!(flag, "-fn".to_owned());
            }
        }
    }

    /// Flags [ nb, nf, sb, and nf] need to be modified for the different
    /// versions of dmenu. Classic Suckless dmenu uses a single dash "-", dmenu-rs
    /// uses the more modern cli style of double dashes "--".
    /// This test depends on the order the flags are loaded into the array, so if the order
    /// is changed in the flags function, these tests will fail. There is a better way, but
    /// this works for now.
    #[test]
    fn dmenu_rs_config_test() {
        let dc = DMenuConfig {
            custom_font: Some("mono".to_owned()),
            kind: DMenuKind::Rust,
            ..DMenuConfig::default()
        };

        assert_eq!(dc.kind, DMenuKind::Rust);
        let flags = dc.flags(0);

        for (i, flag) in flags.into_iter().enumerate() {
            if i == 2 {
                assert_eq!(flag, "--nb".to_owned());
            }
            if i == 4 {
                assert_eq!(flag, "--nf".to_owned());
            }
            if i == 6 {
                assert_eq!(flag, "--sb".to_owned());
            }
            if i == 10 {
                assert_eq!(flag, "--fn".to_owned());
            }
        }
    }
}
