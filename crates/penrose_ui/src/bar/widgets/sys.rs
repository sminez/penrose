//! System monitor widgets
use crate::bar::widgets::{RefreshText, TextStyle};
use std::fs;

/// Display the current charge level and status of a named battery.
///
/// If the given battery name is not found on this system, this widget will
/// render as an empty string.
pub fn battery_summary(bat: &'static str, style: &TextStyle) -> RefreshText {
    RefreshText::new(style, move || get_battery_text(bat).unwrap_or_default())
}

fn get_battery_text(bat: &str) -> Option<String> {
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
