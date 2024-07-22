//! System monitor widgets and utility functions

/// Helper functions for obtaining system information for use in status bar widgets
pub mod helpers {
    use penrose::util::{spawn_for_output, spawn_for_output_with_args};
    use std::{fs, path::PathBuf};

    /// This finds the first battery (BAT) file it finds; so far only
    /// confirmed working on Linux.
    pub fn battery_file_search() -> Option<String> {
        let battery_paths = vec![
            // Linux
            "/sys/class/power_supply",
            // OpenBSD
            "/var/run/apm",
            // FreeBSD and DragonFlyBSD
            "/dev",
            // illumos
            "/dev/battery",
        ];

        battery_paths
            .into_iter()
            .filter_map(|base_path| {
                let base_path = PathBuf::from(base_path);
                if base_path.exists() && base_path.is_dir() {
                    fs::read_dir(base_path).ok()
                } else {
                    None
                }
            })
            .flat_map(|read_dir| read_dir.filter_map(Result::ok))
            .filter_map(|entry| {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_str()?;
                if file_name_str.starts_with("BAT") && file_name_str[3..].parse::<u32>().is_ok() {
                    Some(file_name_str.to_string())
                } else {
                    None
                }
            })
            .next()
    }

    /// Fetch the requested battery's charge as a percentage of its total along with an indicator
    /// of whether it is charging or discharging.
    ///
    /// This will return `None` if it is unable to read or parse the required system files for the
    /// requested battery.
    pub fn battery_text(bat: &str) -> Option<String> {
        let status = read_sys_file(bat, "status")?;
        let energy_now: u32 = read_sys_file(bat, "charge_now")?.parse().ok()?;
        let energy_full: u32 = read_sys_file(bat, "charge_full")?.parse().ok()?;

        let charge = energy_now * 100 / energy_full;

        let icon = if status == "Charging" {
            ""
        } else if charge >= 90 || status == "Full" {
            ""
        } else if charge >= 70 {
            ""
        } else if charge >= 50 {
            ""
        } else if charge >= 20 {
            ""
        } else {
            ""
        };

        Some(format!("{icon} {charge}%"))
    }

    fn read_sys_file(bat: &str, fname: &str) -> Option<String> {
        fs::read_to_string(format!("/sys/class/power_supply/{bat}/{fname}"))
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Fetch the current date and time in `YYYY-MM-DD HH:MM` format using the `date` command line
    /// program.
    ///
    /// Will return `None` if there are errors in calling `date`.
    pub fn date_text() -> Option<String> {
        Some(
            spawn_for_output_with_args("date", &["+%F %R"])
                .ok()?
                .trim()
                .to_string(),
        )
    }

    /// Fetch the active ESSID and associated signal quality for the active wifi network.
    ///
    /// Makes use of the `iwgetid` command line program and will return `None` if there are errors
    /// in calling it or reading required system files to determine the signal quality.
    pub fn wifi_text() -> Option<String> {
        let (interface, essid) = interface_and_essid()?;
        let signal = signal_quality(&interface)?;

        Some(format!("<{essid} {signal}%>"))
    }

    // Read the interface name and essid via iwgetid.
    //   Output format is '$interface    ESSID:"$essid"'
    fn interface_and_essid() -> Option<(String, String)> {
        let raw = spawn_for_output("iwgetid").ok()?;
        let mut iter = raw.split(':');

        // Not using split_whitespace here as the essid may contain whitespace
        let interface = iter.next()?.split_whitespace().next()?.to_owned();
        let essid = iter.next()?.split('"').nth(1)?.to_string();

        Some((interface, essid))
    }

    // Parsing the format described here: https://hewlettpackard.github.io/wireless-tools/Linux.Wireless.Extensions.html
    fn signal_quality(interface: &str) -> Option<String> {
        let raw = fs::read_to_string("/proc/net/wireless").ok()?;

        for line in raw.lines() {
            if line.starts_with(interface) {
                return Some(
                    line.split_whitespace()
                        .nth(2)?
                        .strip_suffix('.')?
                        .to_owned(),
                );
            }
        }

        None
    }

    /// Parse the current volume as a percentage from amixer.
    ///
    /// Expected output format:
    ///   $ amixer sget Master
    ///     Simple mixer control 'Master',0
    ///       Capabilities: pvolume pvolume-joined pswitch pswitch-joined
    ///       Playback channels: Mono
    ///       Limits: Playback 0 - 127
    ///       Mono: Playback 0 [0%] [-63.50dB] [on]
    pub fn amixer_text(channel: &str) -> Option<String> {
        let raw = spawn_for_output(format!("amixer sget {channel}")).ok()?;

        let vol = raw
            .lines()
            .last()?
            .split_whitespace()
            .find(|s| s.ends_with("%]"))?
            .replace(|c| "[]%".contains(c), "");

        Some(format!(" {vol}%"))
    }
}

/// System information widgets provided as [RefreshText] widgets.
///
/// These will update themselves every time that the window manager refreshes its internal state.
/// To update on a specified interval instead, see the [interval] module instead.
pub mod refresh {
    use crate::bar::widgets::{sys::helpers, RefreshText, TextStyle};

    /// Display the current charge level and status of a named battery.
    ///
    /// If the given battery name is not found on this system, this widget will
    /// render as an empty string.
    pub fn battery_summary(bat: &'static str, style: TextStyle) -> RefreshText {
        RefreshText::new(style, move || {
            helpers::battery_text(bat).unwrap_or_default()
        })
    }

    /// Display the current date and time in YYYY-MM-DD HH:MM format
    ///
    /// This widget shells out to the `date` tool to generate its output
    pub fn current_date_and_time(style: TextStyle) -> RefreshText {
        RefreshText::new(style, || helpers::date_text().unwrap_or_default())
    }

    /// Display the ESSID currently connected to and the signal quality as
    /// a percentage.
    pub fn wifi_network(style: TextStyle) -> RefreshText {
        RefreshText::new(style, move || helpers::wifi_text().unwrap_or_default())
    }

    /// Display the current volume level as reported by `amixer`
    pub fn amixer_volume(channel: &'static str, style: TextStyle) -> RefreshText {
        RefreshText::new(style, move || {
            helpers::amixer_text(channel).unwrap_or_default()
        })
    }
}

/// System information widgets provided as [IntervalText] widgets.
///
/// These will update themselves based on the interval provided. To update when the window manager
/// refreshes its internal state, see the [refresh] module instead.
pub mod interval {
    use crate::bar::widgets::{sys::helpers, IntervalText, TextStyle};
    use std::time::Duration;

    /// Display the current charge level and status of a named battery.
    ///
    /// If the given battery name is not found on this system, this widget will
    /// render as an empty string.
    pub fn battery_summary(
        bat: &'static str,
        style: TextStyle,
        interval: Duration,
    ) -> IntervalText {
        IntervalText::new(style, move || helpers::battery_text(bat), interval)
    }

    /// Display the current date and time in YYYY-MM-DD HH:MM format
    ///
    /// This widget shells out to the `date` tool to generate its output
    pub fn current_date_and_time(style: TextStyle, interval: Duration) -> IntervalText {
        IntervalText::new(style, helpers::date_text, interval)
    }

    /// Display the ESSID currently connected to and the signal quality as
    /// a percentage.
    pub fn wifi_network(style: TextStyle, interval: Duration) -> IntervalText {
        IntervalText::new(style, helpers::wifi_text, interval)
    }

    /// Display the current volume level as reported by `amixer`
    pub fn amixer_volume(
        channel: &'static str,
        style: TextStyle,
        interval: Duration,
    ) -> IntervalText {
        IntervalText::new(style, move || helpers::amixer_text(channel), interval)
    }
}
