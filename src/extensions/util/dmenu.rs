//! A simple wrapper for suckless' [dmenu][1] tool for providing quick text based menus
//!
//! [1]: https://tools.suckless.org/dmenu/
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

/// Config for running a [DMenu] selection
#[derive(Debug, Copy, Clone)]
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
}

impl Default for DMenuConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            password_input: false,
            ignore_case: false,
            bg_color: 0x282828ff.into(),
            fg_color: 0xebdbb2ff.into(),
            selected_color: 0x458588ff.into(),
            n_lines: 10,
        }
    }
}

impl DMenuConfig {
    fn flags(&self, prompt: &str, screen_index: usize) -> Vec<String> {
        let &DMenuConfig {
            password_input,
            ignore_case,
            bg_color,
            fg_color,
            selected_color,
            n_lines,
            ..
        } = self;

        let mut flags = vec![
            "-nb".to_string(),
            bg_color.as_rgb_hex_string(),
            "-nf".to_string(),
            fg_color.as_rgb_hex_string(),
            "-sb".to_string(),
            selected_color.as_rgb_hex_string(),
            "-m".to_string(),
            screen_index.to_string(),
        ];

        if n_lines > 0 {
            flags.extend_from_slice(&["-l".to_string(), n_lines.to_string()]);
        }

        if password_input {
            flags.push("-P".to_string());
        }

        if ignore_case {
            flags.push("-i".to_string());
        }

        if !prompt.is_empty() {
            flags.extend_from_slice(&["-p".to_string(), prompt.to_string()]);
        }

        flags
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
    /// # use penrose::extensions::util::dmenu::*;
    /// let lines = vec!["some", "choices", "to", "pick", "from"];
    /// let menu = DMenu::new(">>>", lines, DMenuConfig::default());
    ///
    /// let screen_index = 0;
    ///
    /// match menu.run(screen_index).unwrap() {
    ///     MenuMatch::Line(i, s) => println!("matched '{}' on line '{}'", s, i),
    ///     MenuMatch::UserInput(s) => println!("user input: '{}'", s),
    ///     MenuMatch::NoMatch => println!("no match"),
    /// }
    /// ```
    pub fn run(&self, screen_index: usize) -> Result<MenuMatch> {
        let raw = self.raw_user_choice_from_dmenu(screen_index)?;
        let choice = raw.trim();

        if choice.is_empty() {
            return Ok(MenuMatch::NoMatch);
        }

        let res = self
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
            );

        Ok(res)
    }

    fn choices_as_input_bytes(&self) -> Vec<u8> {
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

        choices.as_bytes().to_vec()
    }

    fn raw_user_choice_from_dmenu(&self, screen_index: usize) -> Result<String> {
        let args = self.config.flags(&self.prompt, screen_index);
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

            stdin.write_all(&self.choices_as_input_bytes())?;
        }

        let mut raw = String::new();
        proc.stdout
            .ok_or_else(|| Error::Custom("failed to spawn dmenu".to_owned()))?
            .read_to_string(&mut raw)?;

        Ok(raw)
    }
}
