//! Document diagnostic gutter indicators, inline underlines, and end-of-line messages.

use ned_core::{diagnostic, diagnostic_severity};
use ned_tui::output::color::Color;
use ned_tui::output::style::Style;

/// Computes the LSP diagnostic indicator and style for a given line index.
#[must_use]
pub fn get_diagnostic_indicator(
    diags: &[diagnostic],
    line_idx: usize,
    gutter_style: Style,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> (&'static str, Style) {
    let mut highest_severity: Option<diagnostic_severity> = None;

    for d in diags {
        if d.range.start.line as usize == line_idx {
            match d.severity {
                Some(diagnostic_severity::error) => {
                    return ("●", error_style);
                }
                Some(diagnostic_severity::warning) => {
                    highest_severity = Some(diagnostic_severity::warning);
                }
                Some(diagnostic_severity::information) => {
                    if highest_severity.is_none()
                        || highest_severity == Some(diagnostic_severity::hint)
                    {
                        highest_severity = Some(diagnostic_severity::information);
                    }
                }
                Some(diagnostic_severity::hint) => {
                    if highest_severity.is_none() {
                        highest_severity = Some(diagnostic_severity::hint);
                    }
                }
                None => {}
            }
        }
    }

    match highest_severity {
        Some(diagnostic_severity::error) => ("●", error_style),
        Some(diagnostic_severity::warning) => ("●", warning_style),
        Some(diagnostic_severity::information) => ("●", info_style),
        Some(diagnostic_severity::hint) => ("●", hint_style),
        None => (" ", gutter_style),
    }
}

/// Returns the underline color for a character inside a diagnostic range, prioritizing errors.
#[must_use]
pub fn diag_underline_color(
    diags: &[diagnostic],
    line_idx: usize,
    col_idx: usize,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> Option<Color> {
    let l = line_idx as u32;
    let c = col_idx as u32;
    let mut best: Option<(u8, Style)> = None;

    for d in diags {
        let s = &d.range.start;
        let e = &d.range.end;
        let in_range = if s.line == e.line {
            l == s.line
                && c >= s.character
                && (c < e.character || (s.character == e.character && c == s.character))
        } else {
            (l == s.line && c >= s.character)
                || (l > s.line && l < e.line)
                || (l == e.line && c < e.character)
        };

        if in_range {
            let (prio, st) = match d.severity {
                Some(diagnostic_severity::error) => (4, error_style),
                Some(diagnostic_severity::warning) => (3, warning_style),
                Some(diagnostic_severity::information) => (2, info_style),
                Some(diagnostic_severity::hint) | None => (1, hint_style),
            };
            if best.is_none_or(|(bp, _)| prio > bp) {
                best = Some((prio, st));
            }
        }
    }

    best.and_then(|(_, st)| st.front_color)
}

/// Returns the highest-severity diagnostic message and style for inline end-of-line display.
#[must_use]
pub fn get_line_inline_diagnostic(
    diags: &[diagnostic],
    line_idx: usize,
    error_style: Style,
    warning_style: Style,
    info_style: Style,
    hint_style: Style,
) -> Option<(&str, Style)> {
    let mut best: Option<(u8, &str, Style)> = None;

    for d in diags {
        if d.range.start.line as usize == line_idx {
            let (prio, st) = match d.severity {
                Some(diagnostic_severity::error) => (4, error_style),
                Some(diagnostic_severity::warning) => (3, warning_style),
                Some(diagnostic_severity::information) => (2, info_style),
                Some(diagnostic_severity::hint) | None => (1, hint_style),
            };
            if best.is_none_or(|(bp, _, _)| prio > bp) {
                best = Some((prio, d.message.as_str(), st));
            }
        }
    }

    best.map(|(_, msg, st)| (msg, st))
}
