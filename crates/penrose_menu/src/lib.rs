//! A penrose Draw backed implementation of dmenu
#![warn(
    broken_intra_doc_links,
    clippy::all,
    missing_debug_implementations,
    future_incompatible,
    missing_docs,
    rust_2018_idioms
)]

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use penrose::{
    core::{
        bindings::KeyPress,
        data_types::{Region, WinId, WinType},
        xconnection::Atom,
    },
    draw::{
        widget::{InputBox, LinesWithSelection, Text},
        Color, DrawContext, DrawError, KeyPressDraw, KeyPressResult, KeyboardControlled, Result,
        TextStyle, Widget,
    },
};

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
    /// Default font to use for rendering text
    ///
    /// Default: monospace
    pub font: String,
    /// Font point size
    ///
    /// Default: 12
    pub point_size: i32,
    /// Maximum width of the spawned window as a percentage of the screen size
    ///
    /// Default: 0.8
    pub max_width_perc: f64,
    /// Maximum height of the spawned window as a percentage of the screen size
    ///
    /// Default: 0.8
    pub max_height_perc: f64,
}

impl Default for PMenuConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            sort_by_relevance: true,
            bg_color: 0x282828ff.into(),
            fg_color: 0xebdbb2ff.into(),
            selected_color: 0x458588ff.into(),
            font: "monospace".into(),
            point_size: 12,
            max_width_perc: 0.8,
            max_height_perc: 0.8,
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
    id: Option<WinId>,
    bg: Color,
    prompt: Text,
    patt: InputBox,
    txt: LinesWithSelection,
    w: f64,
    h: f64,
}

impl<D> PMenu<D>
where
    D: KeyPressDraw,
{
    /// Construct a new [PMenu] with the given config.
    ///
    /// # Example
    /// ```no_run
    /// use penrose::xcb::XcbDraw;
    /// use penrose_menu::{PMenu, PMenuConfig};
    ///
    /// let mut pmenu = match XcbDraw::new() {
    ///     Ok(drw) => PMenu::new(drw, PMenuConfig::default()),
    ///     Err(e) => panic!("unable to initialise Draw: {}", e),
    /// };
    /// ```
    pub fn new(mut drw: D, config: PMenuConfig) -> Result<Self> {
        drw.register_font(&config.font);

        let default_style = TextStyle {
            font: config.font.clone(),
            point_size: config.point_size,
            fg: config.fg_color,
            bg: Some(config.bg_color),
            padding: (1.0, 1.0),
        };

        Ok(Self {
            drw,
            bg: config.bg_color,
            txt: LinesWithSelection::new(
                config.font,
                config.point_size,
                3.0,
                config.bg_color,
                config.fg_color,
                config.selected_color,
                config.fg_color,
                false,
            ),
            patt: InputBox::new(&default_style, false, true),
            prompt: Text::new("", &default_style, false, true),
            w: 0.0,
            h: 0.0,
            id: None,
        })
    }

    fn init_window(&mut self, screen_index: usize, w_max: f64, h_max: f64) -> Result<()> {
        if !(0.0..=1.0).contains(&w_max) || !(0.0..=1.0).contains(&h_max) {
            return Err(DrawError::Raw(format!(
                "w_max and h_max must be in the range 0.0..1.0: w_max={}, h_max={}",
                w_max, h_max
            )));
        }

        let screen_region = *self
            .drw
            .screen_sizes()?
            .get(screen_index)
            .ok_or_else(|| DrawError::Raw("screen_index out of range".into()))?;

        let (_, _, sw, sh) = screen_region.values();
        let mut ctx = self.drw.temp_context(sw, sh)?;
        let (prompt_w, prompt_h) = self.prompt.current_extent(&mut ctx, 1.0)?;
        let (input_w, input_h) = self.txt.current_extent(&mut ctx, 1.0)?;

        // TODO: work out why extent still isn't right
        self.w = (prompt_w + input_w) * 1.1;
        self.h = (prompt_h + input_h) * 1.1;

        let (_, _, w, h) = screen_region
            .scale_w(w_max)
            .scale_h(h_max)
            .centered_in(&screen_region)
            .unwrap() // We know we are bounded by screen_region
            .values();

        let w_max = w as f64;
        let h_max = h as f64;

        if self.w > w_max || self.h > h_max {
            self.w = w_max;
            self.h = h_max;
        }

        let id = self.drw.new_window(
            WinType::InputOutput(Atom::NetWindowTypeDialog),
            Region::new(0, 0, self.w as u32, self.h as u32)
                .centered_in(&screen_region)
                .unwrap(),
            true,
        )?;

        self.drw.flush(id);
        self.id = Some(id);

        Ok(())
    }

    fn redraw(&mut self) -> Result<()> {
        let id = self.id.unwrap();
        let mut ctx = self.drw.context_for(id)?;

        ctx.clear();
        ctx.color(&self.bg);
        ctx.rectangle(0.0, 0.0, self.w, self.h);

        let (w, h) = self.prompt.current_extent(&mut ctx, self.h)?;
        self.prompt.draw(&mut ctx, 0, false, w, h)?;
        ctx.translate(w, 0.0);

        self.patt.draw(&mut ctx, 0, false, w, h)?;
        ctx.translate(0.0, h);

        self.txt.draw(&mut ctx, 0, true, w, h)?;

        ctx.flush();
        self.drw.flush(id);
        Ok(())
    }

    /// Spawn a temporary window using the embedded [KeyPressDraw] impl and fethc input from the user.
    ///
    /// # Example
    /// ```
    /// # use penrose::draw::{Result, KeyPressDraw};
    /// # use penrose_menu::{PMenu, PMenuMatch};
    /// # fn example<T: KeyPressDraw>(mut pmenu: PMenu<T>) -> Result<()> {
    /// let lines = vec!["foo", "bar", "baz"];
    ///
    /// match pmenu.get_selection_from_input(">>> ", lines, 10, 0, 0.8, 0.8)? {
    ///     PMenuMatch::Line(i, s) => println!("matched {} on line {}", s, i),
    ///     PMenuMatch::UserInput(s) => println!("user input: {}", s),
    ///     PMenuMatch::NoMatch => println!("no match"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_selection_from_input(
        &mut self,
        prompt: impl Into<String>,
        input: Vec<impl Into<String>>,
        max_lines: usize,
        screen_index: usize,
        w_max: f64,
        h_max: f64,
    ) -> Result<PMenuMatch> {
        let input: Vec<String> = input.into_iter().map(|s| s.into()).collect();
        self.prompt.set_text(prompt);
        self.txt.set_input(input.clone())?;
        self.txt.set_max_lines(if max_lines < input.len() {
            max_lines
        } else {
            input.len()
        });

        self.init_window(screen_index, w_max, h_max)?;
        let selection = self.get_selection_inner(input);
        self.drw.destroy_window(self.id.unwrap());
        self.id = None;

        selection
    }

    fn get_selection_inner(&mut self, input: Vec<String>) -> Result<PMenuMatch> {
        let mut matches: Vec<(usize, &String)> = input.iter().enumerate().collect();
        let matcher = SkimMatcherV2::default();

        loop {
            if let KeyPressResult::KeyPress(k) = self.drw.next_keypress() {
                match k {
                    KeyPress::Return if self.txt.selected_index() < matches.len() => {
                        let m = matches[self.txt.selected_index()];
                        return Ok(PMenuMatch::Line(m.0, m.1.clone()));
                    }

                    KeyPress::Escape | KeyPress::Return => {
                        let patt = self.patt.get_text();
                        return if patt.is_empty() {
                            Ok(PMenuMatch::NoMatch)
                        } else {
                            Ok(PMenuMatch::UserInput(patt.clone()))
                        };
                    }

                    KeyPress::Backspace | KeyPress::Utf8(_) => {
                        self.patt.handle_keypress(k)?;

                        let mut scored = input
                            .iter()
                            .enumerate()
                            .flat_map(|(i, line)| {
                                matcher
                                    .fuzzy_match(line, self.patt.get_text())
                                    .map(|score| (score, (i, line)))
                            })
                            .collect::<Vec<_>>();

                        scored.sort_by_key(|(score, _)| -*score);
                        matches = scored.into_iter().map(|(_, data)| data).collect();
                        let lines = matches.iter().map(|(_, line)| line.to_string()).collect();
                        self.txt.set_input(lines)?;
                    }

                    KeyPress::Up | KeyPress::Down => {
                        self.txt.handle_keypress(k)?;
                    }

                    _ => continue,
                };
            }

            self.redraw()?;
        }
    }
}
