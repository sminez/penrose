//! A penrose Draw backed implementation of dmenu
#![warn(
    broken_intra_doc_links,
    clippy::all,
    missing_debug_implementations,
    future_incompatible,
    missing_docs,
    rust_2018_idioms
)]

#[macro_use]
extern crate log;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use penrose::{
    core::{
        bindings::KeyPress,
        data_types::{Region, WinType},
        xconnection::{Atom, ExposeEvent, KeyPressParseAttempt, Prop, XEvent, Xid},
    },
    draw::{
        widget::{InputBox, LinesWithSelection, Text},
        Color, DrawContext, DrawError, KeyPressDraw, KeyboardControlled, Result, TextStyle, Widget,
    },
};

use std::convert::TryInto;

const PAD_PX: f64 = 2.0;

/// The result of attempting to match against user input in a call to
/// [PMenu::get_selection_from_input]
#[derive(Debug, Clone)]
pub enum PMenuMatch {
    /// The selected line along its line number (0 indexed)
    Line(usize, String),
    /// Nothing matched and this was the user's input when they hit Return
    UserInput(String),
    /// The user exited out of matching or had nothing typed
    NoMatch,
}

/// Config for running a [PMenu] match
#[derive(Debug, Clone)]
pub struct PMenuConfig {
    /// Should line numbers be displayed to the user?
    ///
    /// Default: false
    pub show_line_numbers: bool,
    /// Should matches be sorted by their ranked relevance compared to the current input?
    ///
    /// Default: true
    pub sort_by_relevance: bool,
    /// Background color for the rendered window
    ///
    /// Default: #111111
    pub bg_color: Color,
    /// Foreground color for text
    ///
    /// Default: #ebdbb2
    pub fg_color: Color,
    /// Selected line background color
    ///
    /// Default #504945
    pub selected_color: Color,
    /// Default font to use for rendering text
    ///
    /// Default: monospace
    pub font: String,
    /// Font point size
    ///
    /// Default: 12
    pub point_size: i32,
    /// Number of lines to display at a time
    ///
    /// Default: 10
    pub n_lines: usize,
    /// Minimum width of the spawned window as a percentage of the screen size
    ///
    /// Default: 0.5
    pub min_width_perc: f64,
}

impl Default for PMenuConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            sort_by_relevance: true,
            bg_color: "#1d2021".try_into().unwrap(),
            fg_color: "#ebdbb2".try_into().unwrap(),
            selected_color: "#504945".try_into().unwrap(),
            font: "monospace".into(),
            point_size: 12,
            n_lines: 10,
            min_width_perc: 0.5,
        }
    }
}

/// Utility struct for obtaining input from the user
#[derive(Debug)]
pub struct PMenu<D>
where
    D: KeyPressDraw,
{
    drw: D,
    id: Option<Xid>,
    bg: Color,
    ac: Color,
    prompt: Text,
    patt: InputBox,
    txt: LinesWithSelection,
    w: f64,
    h: f64,
    show_line_numbers: bool,
    sort_by_relevance: bool,
    min_width_perc: f64,
}

impl<D> PMenu<D>
where
    D: KeyPressDraw,
{
    /// Construct a new [PMenu] with the given config.
    pub fn new(mut drw: D, config: PMenuConfig) -> Result<Self> {
        if !(0.0..=1.0).contains(&config.min_width_perc) {
            return Err(DrawError::Raw(format!(
                "min_width_perc must be in the range 0.0..1.0: {}",
                config.min_width_perc
            )));
        }

        drw.register_font(&config.font);

        let default_style = TextStyle {
            font: config.font.clone(),
            point_size: config.point_size,
            fg: config.fg_color,
            bg: Some(config.bg_color),
            padding: (PAD_PX, PAD_PX),
        };

        let inverted_style = TextStyle {
            fg: config.fg_color,
            bg: Some(config.selected_color),
            ..default_style.clone()
        };

        Ok(Self {
            drw,
            bg: config.bg_color,
            ac: config.selected_color,
            txt: LinesWithSelection::new(
                config.font,
                config.point_size,
                PAD_PX,
                config.bg_color,
                config.fg_color,
                config.selected_color,
                config.fg_color,
                config.n_lines,
                false,
            ),
            patt: InputBox::new(&default_style, false, true),
            prompt: Text::new("", &inverted_style, false, true),
            w: 0.0,
            h: 0.0,
            id: None,
            show_line_numbers: config.show_line_numbers,
            sort_by_relevance: config.sort_by_relevance,
            min_width_perc: config.min_width_perc,
        })
    }

    fn init_window(&mut self, screen_index: usize) -> Result<()> {
        debug!("getting screen size");
        let screen_region = *self
            .drw
            .screen_sizes()?
            .get(screen_index)
            .ok_or_else(|| DrawError::Raw("screen_index out of range".into()))?;

        let (_, _, sw, sh) = screen_region.values();

        let mut ctx = self.drw.temp_context(sw, sh)?;
        let (prompt_w, prompt_h) = self.prompt.current_extent(&mut ctx, 1.0)?;

        let (input_w, input_h) = self.txt.current_extent(&mut ctx, 1.0)?;

        self.w = (prompt_w + input_w + PAD_PX).max((sw as f64) * self.min_width_perc);
        self.h = prompt_h + input_h + PAD_PX * 4.0;

        let id = self.drw.new_window(
            WinType::InputOutput(Atom::NetWindowTypeDialog),
            Region::new(0, 0, self.w as u32, self.h as u32)
                .centered_in(&screen_region)
                .unwrap(),
            true,
        )?;

        let prop = Prop::UTF8String(vec!["penrose-menu".into()]);
        for a in &[Atom::NetWmName, Atom::WmName, Atom::WmClass] {
            self.drw.change_prop(id, a.as_ref(), prop.clone())?;
        }

        self.drw.flush(id)?;
        self.id = Some(id);

        Ok(())
    }

    fn redraw(&mut self, with_prompt: bool) -> Result<()> {
        let id = self.id.unwrap();
        let mut ctx = self.drw.context_for(id)?;

        ctx.clear();
        ctx.color(&self.bg);
        ctx.rectangle(0.0, 0.0, self.w, self.h);

        let (w, h) = if with_prompt {
            let (w, h) = self.prompt.current_extent(&mut ctx, self.h)?;
            ctx.color(&self.ac);
            ctx.rectangle(0.0, 0.0, w + PAD_PX, h + PAD_PX);
            (w, h)
        } else {
            let (_, h) = self.patt.current_extent(&mut ctx, self.h)?;
            (0.0, h)
        };

        ctx.translate(PAD_PX, PAD_PX);

        if with_prompt {
            self.prompt.draw(&mut ctx, 0, false, w, h)?;
            ctx.translate(w, 0.0);
        }

        self.patt.draw(&mut ctx, 0, false, self.w - w, h)?;
        ctx.translate(0.0, h);
        self.txt.draw(&mut ctx, 0, true, self.w - w, h)?;
        self.drw.flush(id)?;

        Ok(())
    }

    /// Set the maximum number of lines from the input that will be displayed.
    ///
    /// Defaults to 10
    pub fn set_n_lines(&mut self, n_lines: usize) {
        self.txt.set_n_lines(n_lines);
    }

    /// Spawn a temporary window using the embedded [KeyPressDraw] impl and fetch input from the user.
    ///
    /// ## NOTE
    /// This method will block the current thread while it runs.
    ///
    /// # Example
    /// ```
    /// # use penrose::draw::{Result, KeyPressDraw};
    /// # use penrose_menu::{PMenu, PMenuMatch};
    /// # fn example<T: KeyPressDraw>(mut pmenu: PMenu<T>) -> Result<()> {
    /// let lines = vec!["foo", "bar", "baz"];
    ///
    /// match pmenu.get_selection_from_input(Some(">>> "), lines, 0)? {
    ///     PMenuMatch::Line(i, s) => println!("matched {} on line {}", s, i),
    ///     PMenuMatch::UserInput(s) => println!("user input: {}", s),
    ///     PMenuMatch::NoMatch => println!("no match"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_selection_from_input(
        &mut self,
        prompt: Option<impl Into<String>>,
        input: Vec<impl Into<String>>,
        screen_index: usize,
    ) -> Result<PMenuMatch> {
        let input: Vec<String> = input.into_iter().map(|s| s.into()).collect();

        let with_prompt = if let Some(p) = prompt {
            self.prompt.set_text(p);
            true
        } else {
            false
        };

        self.txt.set_input(input.clone())?;
        self.init_window(screen_index)?;

        self.drw.grab_keyboard()?;
        let selection = self.get_selection_inner(input, with_prompt);
        self.drw.ungrab_keyboard()?;

        self.drw.destroy_client(self.id.unwrap())?;
        self.id = None;

        selection
    }

    fn get_selection_inner(&mut self, input: Vec<String>, with_prompt: bool) -> Result<PMenuMatch> {
        let display_lines = if self.show_line_numbers {
            input
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:<3} {}", i, line))
                .collect()
        } else {
            input
        };

        self.txt.set_input(display_lines.clone())?;
        self.drw.map_client(self.id.unwrap())?;
        self.redraw(with_prompt)?;

        let mut matches: Vec<(usize, &String)> = display_lines.iter().enumerate().collect();
        let matcher = SkimMatcherV2::default();

        loop {
            debug!("waiting for keypress");
            match self.drw.next_keypress_blocking()? {
                KeyPressParseAttempt::XEvent(XEvent::Expose(ExposeEvent { id, count, .. })) => {
                    debug!("got expose event");
                    if Some(id) == self.id && count == 0 {
                        self.redraw(with_prompt)?;
                    }
                }

                KeyPressParseAttempt::KeyPress(k) => {
                    debug!("got keypress event");
                    match k {
                        KeyPress::Return if self.txt.selected_index() < matches.len() => {
                            let ix = self.txt.selected_index();
                            let (_, raw) = matches[ix];
                            let s = if self.show_line_numbers {
                                raw.split_at(4).1.to_string()
                            } else {
                                raw.to_string()
                            };
                            return Ok(PMenuMatch::Line(ix, s));
                        }

                        KeyPress::Return => {
                            let patt = self.patt.get_text();
                            return if patt.is_empty() {
                                Ok(PMenuMatch::NoMatch)
                            } else {
                                Ok(PMenuMatch::UserInput(patt.clone()))
                            };
                        }

                        KeyPress::Escape => {
                            return Ok(PMenuMatch::NoMatch);
                        }

                        KeyPress::Backspace | KeyPress::Utf8(_) => {
                            self.patt.handle_keypress(k)?;

                            let mut scored = display_lines
                                .iter()
                                .enumerate()
                                .flat_map(|(i, line)| {
                                    matcher
                                        .fuzzy_match(line, self.patt.get_text())
                                        .map(|score| (score, (i, line)))
                                })
                                .collect::<Vec<_>>();

                            if self.sort_by_relevance {
                                scored.sort_by_key(|(score, _)| -*score);
                            }

                            matches = scored.into_iter().map(|(_, data)| data).collect();
                            let lines = matches.iter().map(|(_, line)| line.to_string()).collect();
                            self.txt.set_input(lines)?;
                        }

                        KeyPress::Up | KeyPress::Down => {
                            self.txt.handle_keypress(k)?;
                        }

                        _ => continue,
                    };

                    self.redraw(with_prompt)?;
                }

                _ => (),
            }
        }
    }
}
