//! ANSI escapes printing

use super::{
    color::Color,
    command::TerminalRequest,
    style::{Style, Stylish},
};
use std::{
    fmt::{Display, Formatter},
    io,
};

/// Controls which features of ANSI output to enable
///
/// The color_* options are not exclusive. If you enable all,
/// we print all the kinds of color sequences. You'd do that
/// to be most compatible without having to detect terminal
/// capabilities.
#[derive(Debug, Clone, Copy)]
pub struct AnsiConfig {
    /// print color in 3 bits (8 colors ANSI palette)
    pub color_3b: bool,
    /// print color in 4 bits (16 colors ANSI palette)
    pub color_4b: bool,
    /// print color in 8 bits (256 colors ANSI palette)
    pub color_8b: bool,
    /// print color in 24 bits (true color RGB)
    pub color_24b: bool,
    /// Add CR (\r) to LF (\n) - useful in raw mode
    pub add_cr_to_lf: bool,
}

/// Takes care of printing to ANSI terminal
///
/// It will escape ESC to prevent abuse.
/// It will print colors for style.
/// It will add CR to LF for text if necessary.
#[derive(Debug, Default, Clone)]
pub struct AnsiPrint<T>(pub T);
impl<T> TerminalRequest for AnsiPrint<T>
where
    AnsiPrint<T>: Display,
    T: std::fmt::Debug,
{
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        write.write_all(self.to_string().as_bytes()).map(|()| true)
    }
}
impl<T> AnsiPrint<T> {
    /// Configures default ANSI print configuration.
    pub fn default(self) -> AnsiPrintConfigured<T> {
        AnsiPrintConfigured(self.0, AnsiConfig::default())
    }
    /// Configures raw mode for ANSI printing.
    pub fn raw(self, raw: bool) -> AnsiPrintConfigured<T> {
        self.default().raw(raw)
    }

    pub(crate) fn compatible_color(self) -> AnsiPrintConfigured<T> {
        self.default().compatible_color()
    }

    pub(crate) fn true_color(self) -> AnsiPrintConfigured<T> {
        self.default().true_color()
    }
}
/// Takes care of printing to ANSI terminal
///
/// It will escape ESC to prevent abuse.
/// It will print colors for style.
/// It will add CR to LF for text if necessary.
#[derive(Debug, Default, Clone)]
pub struct AnsiPrintConfigured<T>(pub T, pub AnsiConfig);
impl<T> TerminalRequest for AnsiPrintConfigured<T>
where
    AnsiPrintConfigured<T>: Display,
    T: std::fmt::Debug,
{
    fn apply(&self, mut write: impl io::Write) -> io::Result<bool> {
        write.write_all(self.to_string().as_bytes()).map(|()| true)
    }
}
impl<T> AnsiPrintConfigured<T> {
    fn raw(mut self, raw: bool) -> Self {
        self.1.add_cr_to_lf = !raw;
        self
    }
    fn compatible_color(mut self) -> Self {
        self.1.color_4b = true;
        self.1.color_8b = true;
        self.1.color_24b = true;
        self
    }

    fn true_color(mut self) -> Self {
        self.1.color_24b = true;
        self
    }
}

/// The default is on the safe side
impl Default for AnsiConfig {
    fn default() -> Self {
        Self::compatible_color()
    }
}
impl AnsiConfig {
    /// full control
    #[must_use]
    pub fn new(
        color_3b: bool,
        color_4b: bool,
        color_8b: bool,
        color_24b: bool,
        add_cr_to_lf: bool,
    ) -> Self {
        Self {
            color_3b,
            color_4b,
            color_8b,
            color_24b,
            add_cr_to_lf,
        }
    }
    /// only print RGB and DO add CR to LF
    #[must_use]
    pub const fn true_color() -> Self {
        Self {
            color_3b: false,
            color_4b: false,
            color_8b: false,
            color_24b: true,
            add_cr_to_lf: true,
        }
    }
    /// only print RGB and do NOT add CR to LF
    #[must_use]
    pub const fn raw_true_color() -> Self {
        Self {
            color_3b: false,
            color_4b: false,
            color_8b: false,
            color_24b: true,
            add_cr_to_lf: false,
        }
    }
    /// print color in 4b, 8b and true color, and DO add CR to LF
    #[must_use]
    pub const fn compatible_color() -> Self {
        Self {
            color_3b: false,
            color_4b: true,
            color_8b: true,
            color_24b: true,
            add_cr_to_lf: true,
        }
    }
    /// print color in 4b, 8b and true color, and do NOT add CR to LF
    #[must_use]
    pub const fn raw_compatible_color() -> Self {
        Self {
            color_3b: false,
            color_4b: true,
            color_8b: true,
            color_24b: true,
            add_cr_to_lf: false,
        }
    }
    /// no color support, but DO add CR to LF
    #[must_use]
    pub const fn no_color() -> Self {
        Self {
            color_3b: false,
            color_4b: false,
            color_8b: false,
            color_24b: false,
            add_cr_to_lf: true,
        }
    }
    /// no color support and do NOT add CR to LF
    #[must_use]
    pub const fn raw_no_color() -> Self {
        Self {
            color_3b: false,
            color_4b: false,
            color_8b: false,
            color_24b: false,
            add_cr_to_lf: false,
        }
    }
}
impl<T> Display for AnsiPrint<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        AnsiPrintConfigured(self.0.as_ref(), AnsiConfig::default()).fmt(f)
    }
}
impl Display for AnsiPrint<Style> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        AnsiPrintConfigured(&self.0, AnsiConfig::default()).fmt(f)
    }
}
/// Print normal text, escaping escape sequences to protect from abuse
/// and adding '\r' on '\n' if in raw mode
impl<T> Display for AnsiPrintConfigured<T>
where
    T: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self(content, config) = self;
        for line in content.as_ref().split_inclusive(LF) {
            for part in line.split_inclusive(ESC) {
                if part.ends_with(ESC) {
                    // add CR to LF
                    write!(f, "{part}{ESC}")?;
                } else {
                    write!(f, "{part}")?;
                }
            }
            if config.add_cr_to_lf && line.ends_with(LF) && !line.ends_with(CRLF) {
                // add carriage return after line feed
                write!(f, "{CR}")?;
            }
        }
        Ok(())
    }
}

/// Set the given style, always resetting first
impl Display for AnsiPrintConfigured<Style> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self(style, config) = self;
        AnsiPrintConfigured(ScreenStyle::from(style), *config).fmt(f)
    }
}

/// Set the given style, always resetting first
impl Display for AnsiPrintConfigured<&'_ Style> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self(style, config) = self;
        AnsiPrintConfigured(ScreenStyle::from(*style), *config).fmt(f)
    }
}
/// Set the given style, always resetting first
impl std::fmt::Display for AnsiPrintConfigured<ScreenStyle> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(style, config) = self;
        AnsiPrintConfigured(style, *config).fmt(f)
    }
}
/// Set the given style, always resetting first
impl std::fmt::Display for AnsiPrintConfigured<&'_ ScreenStyle> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(style, config) = self;
        // reset attrs not to mix with previous style
        write!(f, "{CSI}m")?;
        if 1 == 1 & (style.flags >> ScreenStyle::FRONT) {
            let mut printed = false;
            if config.color_3b {
                printed = true;
                let code = ansi_color_code_4b(style.front.to_3b(), false);
                write!(f, "{CSI}1;{code}m")?;
            }
            if config.color_4b {
                printed = true;
                let code = ansi_color_code_4b(style.front.to_4b(), false);
                write!(f, "{CSI}1;{code}m")?;
            }
            if config.color_8b {
                printed = true;
                let ansi = style.front.to_8b();
                write!(f, "{CSI}38;5;{ansi}m")?;
            }
            if config.color_24b && (!style.front.is_ansi() || !printed) {
                let (r, g, b) = style.front.to_24b();
                write!(f, "{CSI}38;2;{r};{g};{b}m")?;
            }
        }
        if 1 == 1 & (style.flags >> ScreenStyle::BACK) {
            let mut printed = false;
            if config.color_3b {
                printed = true;
                let code = ansi_color_code_4b(style.back.to_3b(), true);
                write!(f, "{CSI}1;{code}m")?;
            }
            if config.color_4b {
                printed = true;
                let code = ansi_color_code_4b(style.back.to_4b(), true);
                write!(f, "{CSI}1;{code}m")?;
            }
            if config.color_8b {
                printed = true;
                let ansi = style.back.to_8b();
                write!(f, "{CSI}48;5;{ansi}m")?;
            }
            if config.color_24b && (!style.back.is_ansi() || !printed) {
                let (r, g, b) = style.back.to_24b();
                write!(f, "{CSI}48;2;{r};{g};{b}m")?;
            }
        }
        if 1 == 1 & (style.flags >> ScreenStyle::UNDER) {
            write!(f, "{CSI}4m")?;
            if style.under != style.front {
                let mut printed = false;
                if config.color_8b {
                    printed = true;
                    let ansi = style.under.to_8b();
                    write!(f, "{CSI}58;5;{ansi}m")?;
                }
                if config.color_24b && (!style.under.is_ansi() || !printed) {
                    let (r, g, b) = style.under.to_24b();
                    write!(f, "{CSI}58;2;{r};{g};{b}m")?;
                }
            }
        }
        if 1 == 1 & (style.flags >> ScreenStyle::BOLD) {
            write!(f, "{CSI}1m")?;
        }
        if 1 == 1 & (style.flags >> ScreenStyle::ITALIC) {
            write!(f, "{CSI}3m")?;
        }
        if 1 == 1 & (style.flags >> ScreenStyle::BLINK) {
            write!(f, "{CSI}5m")?;
        }
        Ok(())
    }
}

pub(crate) fn ansi_color_code_4b(mut c: u8, background: bool) -> u8 {
    if c >= 8 {
        c += 52;
    }
    if background {
        c += 10;
    }
    c += 30;
    assert!(
        (30..38).contains(&c)
            || (40..48).contains(&c)
            || (90..98).contains(&c)
            || (100..108).contains(&c),
        "correct ansi escape code param"
    );
    c
}
/// Carriage return and line feed.
pub const CRLF: &str = "\r\n";
/// Line feed character.
pub const LF: char = '\n';
/// Carriage return character.
pub const CR: char = '\r';
/// Escape character.
pub const ESC: char = '\x1b';
/// Control Sequence Introducer string.
pub const CSI: &str = "\x1b[";

/// Terminal screen cell style representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenStyle {
    flags: u8,
    front: Color,
    back: Color,
    under: Color,
}
impl ScreenStyle {
    /// Default screen style (grey on black).
    pub const DEFAULT: ScreenStyle = ScreenStyle {
        flags: 0,
        front: Color::grey(true),
        back: Color::black(),
        under: Color::grey(true),
    };
    /// Returns foreground color if set.
    #[must_use]
    pub fn front_color(&self) -> Option<Color> {
        if 1 == 1 & (self.flags >> Self::FRONT) {
            Some(self.front)
        } else {
            None
        }
    }
    /// Returns background color if set.
    #[must_use]
    pub fn back_color(&self) -> Option<Color> {
        if 1 == 1 & (self.flags >> Self::BACK) {
            Some(self.back)
        } else {
            None
        }
    }
}
impl ScreenStyle {
    const FRONT: u8 = 0;
    const BACK: u8 = 1;
    const UNDER: u8 = 2;
    const BOLD: u8 = 3;
    const ITALIC: u8 = 4;
    const BLINK: u8 = 5;
}

impl<T> From<&T> for ScreenStyle
where
    T: Stylish,
{
    fn from(stylish: &T) -> Self {
        let style = stylish.style();
        let front = style.front_color.unwrap_or(Color::grey(true));
        let back = style.back_color.unwrap_or(Color::black());
        let under = style.frame_color.unwrap_or(front);
        let flags = (u8::from(style.frame_color.is_some() && style.underlined.unwrap_or_default())
            << Self::UNDER)
            | (u8::from(style.front_color.is_some()) << Self::FRONT)
            | (u8::from(style.back_color.is_some()) << Self::BACK)
            | (u8::from(style.bold.unwrap_or_default()) << Self::BOLD)
            | (u8::from(style.italic.unwrap_or_default()) << Self::ITALIC)
            | (u8::from(style.blink.unwrap_or_default()) << Self::BLINK);
        Self {
            flags,
            front,
            back,
            under,
        }
    }
}

#[test]
fn print_new_lines() {
    assert_eq!(
        AnsiPrintConfigured("a\n b\r\n c", AnsiConfig::no_color()).to_string(),
        "a\n\r b\r\n c"
    );
}

#[test]
fn print_esc_esc() {
    assert_eq!(
        AnsiPrintConfigured("a \x1b[0m b", AnsiConfig::default()).to_string(),
        "a \x1b\x1b[0m b"
    );
}

#[test]
fn print_compatible_colors() {
    use crate::output::style::Stylist;
    assert_eq!(
        AnsiPrintConfigured(
            crate::output::style::Style::DEFAULT
                .front(crate::output::color::Color::rgb(1, 99, 222)),
            AnsiConfig::compatible_color()
        )
        .to_string(),
        "\x1b[m\x1b[1;96m\x1b[38;5;26m\x1b[38;2;1;99;222m"
    );
}
