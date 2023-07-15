//! System monitor widgets
use crate::bar::widgets::{RefreshText, TextStyle};
use penrose::util::{spawn_for_output, spawn_for_output_with_args};
use std::fs;

/// Display the current charge level and status of a named battery.
///
/// If the given battery name is not found on this system, this widget will
/// render as an empty string.
pub fn battery_summary(bat: &'static str, style: TextStyle) -> RefreshText {
    RefreshText::new(style, move || battery_text(bat).unwrap_or_default())
}

fn battery_text(bat: &str) -> Option<String> {
    let status = read_sys_file(bat, "status")?;
    let energy_now: u32 = read_sys_file(bat, "energy_now")?.parse().ok()?;
    let energy_full: u32 = read_sys_file(bat, "energy_full")?.parse().ok()?;

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

/// Display the current date and time in YYYY-MM-DD HH:MM format
///
/// This widget shells out to the `date` tool to generate its output
pub fn current_date_and_time(style: TextStyle) -> RefreshText {
    RefreshText::new(style, || {
        spawn_for_output_with_args("date", &["+%F %R"])
            .unwrap_or_default()
            .trim()
            .to_string()
    })
}

/// Display the ESSID currently connected to and the signal quality as
/// a percentage.
pub fn wifi_network(style: TextStyle) -> RefreshText {
    RefreshText::new(style, move || wifi_text().unwrap_or_default())
}

fn wifi_text() -> Option<String> {
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

/// Display the current volume level as reported by `amixer`
pub fn amixer_volume(channel: &'static str, style: TextStyle) -> RefreshText {
    RefreshText::new(style, move || amixer_text(channel).unwrap_or_default())
}

// Parse the current volume as a percentage from amixer.
//
// Expected output format:
//   $ amixer sget Master
//     Simple mixer control 'Master',0
//       Capabilities: pvolume pvolume-joined pswitch pswitch-joined
//       Playback channels: Mono
//       Limits: Playback 0 - 127
//       Mono: Playback 0 [0%] [-63.50dB] [on]
fn amixer_text(channel: &str) -> Option<String> {
    let raw = spawn_for_output(format!("amixer sget {channel}")).ok()?;

    let vol = raw
        .lines()
        .last()?
        .split_whitespace()
        .nth(3)?
        .replace(|c| "[]%".contains(c), "");

    Some(format!(" {vol}%"))
}
