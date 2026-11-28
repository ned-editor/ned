//! fallback terminal size discovery using environment variables and tput.

#![allow(dead_code)]

use crate::prelude::{point, Point};

/// discovers terminal dimensions without native syscalls.
#[must_use]
pub fn get_terminal_size_no_sys() -> Option<Point> {
    get_terminal_size_from_env()
        .or_else(get_terminal_size_from_tput)
        .or(Some(point(80, 24)))
}

/// reads LINES and COLUMNS environment variables.
#[must_use]
pub fn get_terminal_size_from_env() -> Option<Point> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .and_then(|c| {
            std::env::var("LINES")
                .ok()
                .and_then(|l| l.parse().ok())
                .map(|l| point(c, l))
        })
}

/// returns the size of the screen as determined by tput.
#[must_use]
pub fn get_terminal_size_from_tput() -> Option<Point> {
    match (tput_value("cols"), tput_value("lines")) {
        (Some(w), Some(h)) => Some((w, h).into()),
        _ => None,
    }
}

/// Parses a numeric value from `tput` command output.
fn tput_value(arg: &str) -> Option<u16> {
    let output = std::process::Command::new("tput").arg(arg).output().ok()?;
    let value = output
        .stdout
        .into_iter()
        .filter_map(|b| char::from(b).to_digit(10))
        .fold(0, |v, n| v * 10 + u16::try_from(n).expect("digit fits in u16"));

    if value > 0 {
        Some(value)
    } else {
        None
    }
}
