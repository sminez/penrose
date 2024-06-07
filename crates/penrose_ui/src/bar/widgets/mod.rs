//! Self rendering building blocks for text based UI elements
use crate::{Context, Result, TextStyle};
use penrose::{
    core::State,
    pure::geometry::Rect,
    x::{XConn, XEvent},
    Color, Xid,
};
use std::{
    fmt,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tracing::trace;

pub mod debug;
mod simple;
mod sys;
mod workspaces;

pub use simple::{ActiveWindowName, CurrentLayout, RootWindowName};
pub use sys::{amixer_volume, battery_summary, current_date_and_time, wifi_network};
pub use workspaces::Workspaces;

/// A status bar widget that can be rendered using a [Context]
pub trait Widget<X>
where
    X: XConn,
{
    /// Render the current state of the widget to the status bar window.
    fn draw(
        &mut self,
        ctx: &mut Context<'_>,
        screen: usize,
        screen_has_focus: bool,
        w: u32,
        h: u32,
    ) -> Result<()>;

    /// Current required width and height for this widget due to its content
    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)>;

    /// Does this widget currently require re-rendering? (should be reset to false when 'draw' is called)
    fn require_draw(&self) -> bool;

    /// If true, this widget will expand to fill remaining available space after layout has been
    /// computed. If multiple greedy widgets are present in a given StatusBar then the available
    /// space will be split evenly between all widgets.
    fn is_greedy(&self) -> bool;

    #[allow(unused_variables)]
    /// A startup hook to be run in order to initialise this Widget
    fn on_startup(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// An event hook to be run in order to update this Widget
    fn on_event(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// A refresh hook to be run in order to update this Widget
    fn on_refresh(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// A manage hook to be run in order to update this Widget
    fn on_new_client(&mut self, id: Xid, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }
}

/// A simple piece of static text with an optional background color.
///
/// Can be used as a simple static element in a status bar or as an inner element for rendering
/// more complex text based widgets.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    txt: String,
    fg: Color,
    bg: Option<Color>,
    padding: (u32, u32),
    is_greedy: bool,
    right_justified: bool,
    extent: Option<(u32, u32)>,
    require_draw: bool,
}

impl Text {
    /// Construct a new [Text]
    pub fn new(
        txt: impl Into<String>,
        style: TextStyle,
        is_greedy: bool,
        right_justified: bool,
    ) -> Self {
        Self {
            txt: txt.into(),
            fg: style.fg,
            bg: style.bg,
            padding: style.padding,
            is_greedy,
            right_justified,
            extent: None,
            require_draw: true,
        }
    }

    /// Borrow the current contents of the widget.
    pub fn get_text(&self) -> &String {
        &self.txt
    }

    /// Mutably borrow the current contents of the widget.
    pub fn get_text_mut(&mut self) -> &mut String {
        &mut self.txt
    }

    /// Set the rendered text and trigger a redraw
    pub fn set_text(&mut self, txt: impl Into<String>) {
        let new_text = txt.into();
        if self.txt != new_text {
            self.txt = new_text;
            self.extent = None;
            self.require_draw = true;
        }
    }
}

impl<X: XConn> Widget<X> for Text {
    fn draw(&mut self, ctx: &mut Context<'_>, _: usize, _: bool, w: u32, h: u32) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.fill_rect(Rect::new(0, 0, w, h), color)?;
        }

        let (ew, eh) = <Self as Widget<X>>::current_extent(self, ctx, h)?;
        let offset = w as i32 - ew as i32;
        let right_justify = self.right_justified && self.is_greedy && offset > 0;
        if right_justify {
            ctx.translate(offset, 0);
            ctx.draw_text(&self.txt, h - eh, self.padding, self.fg)?;
            ctx.translate(-offset, 0);
        } else {
            ctx.draw_text(&self.txt, h - eh, self.padding, self.fg)?;
        }

        self.require_draw = false;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, _h: u32) -> Result<(u32, u32)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let (l, r) = self.padding;
                let (w, h) = ctx.text_extent(&self.txt)?;
                let extent = (w + l + r, h);
                self.extent = Some(extent);

                Ok(extent)
            }
        }
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn is_greedy(&self) -> bool {
        self.is_greedy
    }
}

/// A simple widget that does not care about window manager state.
///
/// On each refresh, the provided `get_text` function is called and the output is
/// stored in a [Text] widget. Whenever the output changes, this widget will trigger
/// a re-render of the status bar.
///
///
/// ### A note on blocking code
///
/// Be aware that the `get_text` function you provide will be run on _every_ refresh
/// of the internal window manager state, meaning that slow running functions will
/// very quickly make your window manager sluggish and unresponsive! If you need to
/// run logic that is slow or may take a variable amount of time (such as pulling
/// data in over the network) then you will likely want to make use of the
/// [`IntervalText`] struct instead.
///
/// # Example
/// ```no_run
/// use penrose::{util::spawn_for_output_with_args, Color};
/// use penrose_ui::{bar::widgets::RefreshText, core::TextStyle};
///
/// // Use the pacman package manager to get a count of how many packages are
/// // currently installed on the system.
/// fn my_get_text() -> String {
///     let n_packages = spawn_for_output_with_args("sh", &["-c", "pacman -Q | wc -l"])
///         .unwrap_or_default()
///         .trim()
///         .to_string();
///
///     format!("#pacman packages: {n_packages}")
/// }
///
/// let style = TextStyle {
///     fg: 0xebdbb2ff.into(),
///     bg: Some(0x282828ff.into()),
///     padding: (2, 2),
/// };
///
/// let my_widget = RefreshText::new(style, my_get_text);
/// ```
pub struct RefreshText {
    inner: Text,
    get_text: Box<dyn Fn() -> String>,
}

impl fmt::Debug for RefreshText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefreshText")
            .field("inner", &self.inner)
            .finish()
    }
}

impl RefreshText {
    /// Construct a new [`RefreshText`] using the specified styling and a function for
    /// generating the widget contents.
    pub fn new<F>(style: TextStyle, get_text: F) -> Self
    where
        F: Fn() -> String + 'static,
    {
        Self {
            inner: Text::new("", style, false, false),
            get_text: Box::new(get_text),
        }
    }
}

impl<X: XConn> Widget<X> for RefreshText {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        Widget::<X>::current_extent(&mut self.inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        Widget::<X>::is_greedy(&self.inner)
    }

    fn require_draw(&self) -> bool {
        Widget::<X>::require_draw(&self.inner)
    }

    fn on_refresh(&mut self, _: &mut State<X>, _: &X) -> Result<()> {
        let txt = (self.get_text)();

        self.inner.set_text(txt);

        Ok(())
    }
}

/// A simple widget that does not care about window manager state and refreshes on a
/// specified interval.
///
/// On the requested interval, the provided `get_text` function is called and the output is
/// stored in a [`Text`] widget. Whenever the output changes, this widget will trigger
/// a re-render of the status bar.
///
/// Unlike [`RefreshText`], your `get_text` function will only be run on the schedule you
/// specify rather than every time the internal window manager state refreshes. This is
/// useful for code that is slow to run such as network requests.
///
/// # Example
/// ```no_run
/// use penrose::{util::spawn_for_output_with_args, Color};
/// use penrose_ui::{bar::widgets::IntervalText, core::TextStyle};
/// use std::time::Duration;
///
/// // Make a curl request to wttr.in to fetch the current weather information
/// // for our location.
/// fn my_get_text() -> String {
///     spawn_for_output_with_args("curl", &["-s", "http://wttr.in?format=3"])
///         .unwrap_or_default()
///         .trim()
///         .to_string()
/// }
///
/// let style = TextStyle {
///     fg: 0xebdbb2ff.into(),
///     bg: Some(0x282828ff.into()),
///     padding: (2, 2),
/// };
///
///
/// let my_widget = IntervalText::new(
///     style,
///     my_get_text,
///     Duration::from_secs(60 * 5)
/// );
/// ```
#[derive(Debug)]
pub struct IntervalText {
    inner: Arc<Mutex<Text>>,
}

impl IntervalText {
    /// Construct a new [`IntervalText`] using the specified styling and a function for
    /// generating the widget contents. The function for updating the widget contents
    /// will be run in its own thread on the interval provided.
    pub fn new<F>(style: TextStyle, get_text: F, interval: Duration) -> Self
    where
        F: Fn() -> String + 'static + Send,
    {
        let inner = Arc::new(Mutex::new(Text::new("", style, false, false)));
        let txt = Arc::clone(&inner);

        thread::spawn(move || loop {
            trace!("updating text for IntervalText widget");
            let s = (get_text)();

            {
                let mut t = match txt.lock() {
                    Ok(inner) => inner,
                    Err(poisoned) => poisoned.into_inner(),
                };
                t.set_text(s);
            }

            thread::sleep(interval);
        });

        Self { inner }
    }
}

impl<X: XConn> Widget<X> for IntervalText {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::draw(&mut *inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::current_extent(&mut *inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::is_greedy(&*inner)
    }

    fn require_draw(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::require_draw(&*inner)
    }
}
