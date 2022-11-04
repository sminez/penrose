//! Utility extensions for use in the penrose window manager
use crate::{
    pure::RelativePosition,
    util::{spawn, spawn_for_output, spawn_with_args},
    Error, Result,
};

pub mod debug;
pub mod dmenu;

/// Detect the current monitor set up and arrange the monitors if needed using [xrandr][1].
///
/// NOTE
/// - Primary monitor will be set to `primary`
/// - Monitor resolution is set using the --auto flag in xrandr
/// - Only supports one and two monitor setups.
///
/// [1]: https://wiki.archlinux.org/index.php/Xrandr
pub fn update_monitors_via_xrandr(
    primary: &str,
    secondary: &str,
    position: RelativePosition,
) -> Result<()> {
    let raw = spawn_for_output("xrandr")?;
    let status = raw
        .lines()
        .find(|line| line.starts_with(secondary))
        .ok_or_else(|| {
            Error::Custom("unable to find secondary monitor in xrandr output".to_owned())
        })?
        .split(' ')
        .nth(1)
        .ok_or_else(|| Error::Custom("unexpected xrandr output".to_owned()))?;

    let pos = match position {
        RelativePosition::Left => "--left-of",
        RelativePosition::Right => "--right-of",
        RelativePosition::Above => "--above",
        RelativePosition::Below => "--below",
    };

    // force the primary monitor
    spawn(format!("xrandr --output {} --primary --auto", primary))?;

    // Updated the secondary monitor
    match status {
        "disconnected" => spawn(format!("xrandr --output {secondary} --off")),

        "connected" => spawn(format!(
            "xrandr --output {secondary} --auto {pos} {primary}",
        )),

        _ => Ok(()),
    }
}

/// A notification level when calling notify-send
///
/// The effect of this on the generated notification will depend on the notification
/// daemon that you are using.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NotifyLevel {
    /// Low priority
    Low,
    /// Normal priority
    Normal,
    /// Critical priority
    Critical,
}

/// Send a notification using the `notify-send` command line program
pub fn notify_send(title: impl AsRef<str>, body: impl AsRef<str>) -> Result<()> {
    notify_send_custom(title, body, NotifyLevel::Normal, 5000)
}

/// Send a notification using the `notify-send` command line program
///
/// Duration is in ms.
pub fn notify_send_custom(
    title: impl AsRef<str>,
    body: impl AsRef<str>,
    level: NotifyLevel,
    duration: usize,
) -> Result<()> {
    let level = match level {
        NotifyLevel::Low => "low",
        NotifyLevel::Normal => "normal",
        NotifyLevel::Critical => "critical",
    };

    spawn_with_args(
        "notify-send",
        &[
            "-u",
            level,
            "-t",
            &duration.to_string(),
            title.as_ref(),
            body.as_ref(),
        ],
    )
}
