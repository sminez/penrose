//! A simple wrapper around notify-send to allow for generating notifications
use crate::{common::helpers::spawn_with_args, Error, ErrorHandler, Result};
use std::fmt;
use tracing::error;

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
    /// Critical / high priority
    Critical,
}

impl fmt::Display for NotifyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::Critical => "critical",
        };

        write!(f, "{}", s)
    }
}

/// Notification configuration
#[derive(Debug)]
pub struct NotifyConfig {
    /// The urgency level of the generated notification
    ///
    /// default: Normal
    level: NotifyLevel,
    /// Duration in milliseconds that the notification should be displayed for
    ///
    /// default: 5000
    duration: usize,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            level: NotifyLevel::Normal,
            duration: 5000,
        }
    }
}

/// Send a notification using the `notify-send` command line program
///
/// # Example
/// ```no_run
/// # use penrose::{contrib::extensions::notify_send::*};
/// # fn example() -> penrose::Result<()> {
/// notify_send("My Notification", "hello from penrose!", NotifyConfig::default())?;
///
/// // equivalent to the following on the command line:
/// // $ notify-send 'My Notification' 'hello from penrose!' -u normal -t 5000
/// # Ok(())
/// # }
/// ```
pub fn notify_send(
    title: impl Into<String>,
    body: impl Into<String>,
    config: NotifyConfig,
) -> Result<()> {
    spawn_with_args(
        "notify-send",
        &[
            "-u",
            &config.level.to_string(),
            "-t",
            &config.duration.to_string(),
            &title.into(),
            &body.into(),
        ],
    )
}

/// A simple error handler that uses 'notify-send' to display a dialog window with the error
/// message.
pub fn notify_send_error_handler() -> ErrorHandler {
    Box::new(|e: Error| {
        if notify_send(
            "Unhandled Error",
            e.to_string(),
            NotifyConfig {
                level: NotifyLevel::Critical,
                duration: 10000,
            },
        )
        .is_err()
        {
            error!("Unable to display error via notify-send. Error was: {}", e);
        }
    })
}
