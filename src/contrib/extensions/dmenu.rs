//! A simple wrapper for suckless' [dmenu][1] tool for providing quick text based menus
//!
//! [1]: https://tools.suckless.org/dmenu/
use crate::{draw::Color, PenroseError, Result};

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

/// Config for running a [DMenu] selection
#[derive(Debug, Clone)]
pub struct DMenuConfig {
    /// Should line numbers be displayed to the user?
    ///
    /// Default: false
    pub show_line_numbers: bool,
    /// Should dmenu treat the input as a password and render characters as '*'?
    ///
    /// # NOTE
    /// This requires the [Password][1] patch in order to work.
    ///
    /// Default: false
    ///
    /// [1]: https://tools.suckless.org/dmenu/patches/password/
    pub password_input: bool,
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
    /// Default #458588
    pub selected_color: Color,
    /// Number of lines to display at a time
    ///
    /// Default: 10
    pub n_lines: usize,
}

impl Default for DMenuConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            password_input: false,
            bg_color: 0x282828ff.into(),
            fg_color: 0xebdbb2ff.into(),
            selected_color: 0x458588ff.into(),
            n_lines: 10,
        }
    }
}

impl DMenuConfig {
    fn flags(&self, prompt: &str, screen_index: usize) -> Vec<String> {
        let mut s = format!(
            "-l {} -nb {} -nf {} -sb {} -m {}",
            self.n_lines,
            self.bg_color.as_rgb_hex_string(),
            self.fg_color.as_rgb_hex_string(),
            self.selected_color.as_rgb_hex_string(),
            screen_index,
        );

        if self.password_input {
            s.push_str(" -P");
        }

        if !prompt.is_empty() {
            s.push_str(&format!(" -p {}", prompt));
        }

        s.split_whitespace().map(|s| s.into()).collect()
    }
}

/// A wrapper around the suckless [dmenu][1] program for creating dynamic menus
/// in penrose.
///
/// [1]: https://tools.suckless.org/dmenu/
#[derive(Debug, Clone)]
pub struct DMenu {
    config: DMenuConfig,
    prompt: String,
    choices: Vec<String>,
}

impl DMenu {
    /// Create a new [DMenu] command which can be triggered and re-used by calling the `run` method
    pub fn new(
        prompt: impl Into<String>,
        choices: Vec<impl Into<String>>,
        config: DMenuConfig,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            choices: choices.into_iter().map(|s| s.into()).collect(),
            config,
        }
    }

    /// Run this [DMenu] command and return the selected choice.
    ///
    /// # Example
    /// ```no_run
    /// # use penrose::{contrib::extensions::dmenu::*, WindowManager, XcbConnection};
    /// # fn example(manager: &mut WindowManager<XcbConnection>) -> penrose::Result<()> {
    /// let lines = vec!["some", "choices", "to", "pick", "from"];
    /// let menu = DMenu::new(">>>", lines, DMenuConfig::default());
    ///
    /// let screen_index = manager.active_screen_index();
    ///
    /// match menu.run(screen_index)? {
    ///     MenuMatch::Line(i, s) => println!("matched '{}' on line '{}'", s, i),
    ///     MenuMatch::UserInput(s) => println!("user input: '{}'", s),
    ///     MenuMatch::NoMatch => println!("no match"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn run(&self, screen_index: usize) -> Result<MenuMatch> {
        let args = self.config.flags(&self.prompt, screen_index);
        let choices = if self.config.show_line_numbers {
            self.choices
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{:<3} {}", i, s))
                .collect::<Vec<String>>()
                .join("\n")
        } else {
            self.choices.join("\n")
        };

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
                .ok_or_else(|| perror!("unable to open stdin"))?;
            stdin.write_all(choices.as_bytes())?;
        }

        let mut raw = String::new();
        proc.stdout
            .ok_or_else(|| PenroseError::SpawnProc("failed to spawn dmenu".into()))?
            .read_to_string(&mut raw)?;
        let choice = raw.trim();

        if choice.is_empty() {
            return Ok(MenuMatch::NoMatch);
        }

        Ok(self
            .choices
            .iter()
            .enumerate()
            .find(|(i, s)| {
                if self.config.show_line_numbers {
                    format!("{:<3} {}", i, s) == choice
                } else {
                    *s == choice
                }
            })
            .map_or_else(
                || MenuMatch::UserInput(choice.to_string()),
                |(i, _)| MenuMatch::Line(i, self.choices[i].to_string()),
            ))
    }
}
