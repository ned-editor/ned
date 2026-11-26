//! ANSI composable styling structures and helpers

use super::color::Color;
use std::{
    borrow::Cow,
    fmt::Display,
    ops::{Deref, DerefMut},
};

/// Wrap displayable types with style for the terminal.
/// It is usually used indirectly through the trait helper [`Styled`] and [`Stylist`]:
///
/// ```rust
/// use termit::prelude::Stylist;
/// use termit::prelude::Pretty;
/// use termit::prelude::Color;
/// let pretty = "hello".front(Color::white());
/// assert!(matches!(pretty, Pretty{..}));
/// // Pretty itself can be styled:
/// pretty.bold(true);
/// // And thanks to deref, you can still access the str
/// assert_eq!(pretty.chars().next(), Some('h'));
/// ```
///
/// [`Pretty`] is also a widget so you can use it in your UI.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Pretty<T> {
    content: T,
    style: Style,
}
impl<T> Pretty<T> {
    /// Create something pretty, then use the [`Stylist`] helper.
    pub fn new(content: T) -> Self {
        Self {
            content,
            style: Style::DEFAULT,
        }
    }
    /// Unwraps the inner content and style.
    pub fn into_inner(self) -> (T, Style) {
        (self.content, self.style)
    }
}

/// The composable style for your terminal output.
///
/// You probably do not need to work it directly and
/// instead use the [`Stylist`] helper.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// The foreground text color.
    pub front_color: Option<Color>,
    /// The background fill color.
    pub back_color: Option<Color>,
    /// The underline / frame color.
    pub frame_color: Option<Color>,
    /// Bold text modifier.
    pub bold: Option<bool>,
    /// Italic text modifier.
    pub italic: Option<bool>,
    /// Strike-through text modifier.
    pub crossed_out: Option<bool>,
    /// Underline text modifier.
    pub underlined: Option<bool>,
    /// Blinking text modifier.
    pub blink: Option<bool>,
}
impl Style {
    /// static empty style
    pub const DEFAULT: Style = Style {
        front_color: None,
        back_color: None,
        frame_color: None,
        bold: None,
        italic: None,
        underlined: None,
        blink: None,
        crossed_out: None,
    };
}
/// Things that have a style and we can modify it
///
/// Implement this if you hold a style inside and want to be `Styled`
pub trait Stylish {
    /// Mutable reference to internal style.
    fn style_mut(&mut self) -> &mut Style;
    /// Reference to internal style.
    fn style(&self) -> &Style;
}
/// Convenience to modify style on things that have one (or may have one)
///
/// You probably wouldn't implement this directly. Instead, implement `Styled`
pub trait Stylist {
    /// The target type being styled.
    type Cute;
    /// Sets foreground color.
    fn front(self, color: impl Into<Option<Color>>) -> Self::Cute;
    /// Sets background color.
    fn back(self, color: impl Into<Option<Color>>) -> Self::Cute;
    /// Sets frame / underline color.
    fn frame(self, color: impl Into<Option<Color>>) -> Self::Cute;
    /// Toggles underline.
    fn underlined(self, underlined: impl Into<Option<bool>>) -> Self::Cute;
    /// Toggles strike-through.
    fn crossed_out(self, crossed: bool) -> Self::Cute;
    /// Toggles bold modifier.
    fn bold(self, bold: impl Into<Option<bool>>) -> Self::Cute;
    /// Toggles italic modifier.
    fn italic(self, italic: impl Into<Option<bool>>) -> Self::Cute;
    /// Toggles blink modifier.
    fn blink(self, blink: impl Into<Option<bool>>) -> Self::Cute;
    /// Applies additional style attributes.
    fn apply(self, additional: impl AsRef<Style>) -> Self::Cute;
    /// Mutates style using closure.
    fn with_style(self, what: impl FnOnce(&mut Style)) -> Self::Cute;
}
/// Things that would benefit from touching up with extra style
///
/// Implement this if you'd like to be styled.
/// Either you hold the style within, so implement `Stylish` and your `type Cute=Self`.
/// Or you can wrap yourself in `Pretty` and your `type Cute=Pretty<Self>`
pub trait Styled: Sized {
    /// The cute styled target type.
    type Cute: Stylish;
    /// Wraps into a styled entity.
    fn pretty(self) -> Self::Cute;
}

impl<T> Stylish for Pretty<T> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
    fn style(&self) -> &Style {
        &self.style
    }
}
impl Stylish for Style {
    fn style_mut(&mut self) -> &mut Style {
        self
    }
    fn style(&self) -> &Style {
        self
    }
}

impl Styled for Style {
    type Cute = Self;

    fn pretty(self) -> Self::Cute {
        self
    }
}
impl<T> Styled for Pretty<T> {
    type Cute = Self;

    fn pretty(self) -> Self::Cute {
        self
    }
}

impl<T> Stylist for T
where
    T: Styled,
{
    type Cute = T::Cute;
    fn front(self, color: impl Into<Option<Color>>) -> Self::Cute {
        self.with_style(|style| style.front_color = color.into())
    }
    fn back(self, color: impl Into<Option<Color>>) -> Self::Cute {
        self.with_style(|style| style.back_color = color.into())
    }
    fn underlined(self, underlined: impl Into<Option<bool>>) -> Self::Cute {
        self.with_style(|style| style.underlined = underlined.into())
    }
    fn frame(self, color: impl Into<Option<Color>>) -> Self::Cute {
        self.with_style(|style| style.frame_color = color.into())
    }
    fn crossed_out(self, crossed: bool) -> Self::Cute {
        self.with_style(|style| style.crossed_out = crossed.into())
    }
    fn bold(self, bold: impl Into<Option<bool>>) -> Self::Cute {
        self.with_style(|style| style.bold = bold.into())
    }
    fn italic(self, italic: impl Into<Option<bool>>) -> Self::Cute {
        self.with_style(|style| style.italic = italic.into())
    }
    fn blink(self, blink: impl Into<Option<bool>>) -> Self::Cute {
        self.with_style(|style| style.blink = blink.into())
    }
    fn apply(self, additional: impl AsRef<Style>) -> Self::Cute {
        self.with_style(|style| *style += additional)
    }
    fn with_style(self, what: impl FnOnce(&mut Style)) -> Self::Cute {
        let mut pretty = self.pretty();
        what(pretty.style_mut());
        pretty
    }
}

impl<T> Deref for Pretty<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}
impl<T> DerefMut for Pretty<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

macro_rules! with_default_style {
    ($type:ty) => {
        impl Styled for $type {
            type Cute = Pretty<Self>;
            fn pretty(self) -> Pretty<Self> {
                Pretty {
                    content: self,
                    style: Style::DEFAULT,
                }
            }
        }
    };
}

with_default_style!(Box<dyn Display>);
with_default_style!(&str);
with_default_style!(String);
with_default_style!(&String);
with_default_style!(&mut String);
with_default_style!(Cow<'_, &'_ str>);
with_default_style!(std::sync::Arc<String>);
with_default_style!(std::rc::Rc<String>);
with_default_style!(std::cell::Cell<String>);
with_default_style!(std::sync::Arc<&'_ str>);
with_default_style!(std::rc::Rc<&'_ str>);
with_default_style!(std::cell::Cell<&'_ str>);
with_default_style!(std::sync::Arc<Cow<'_, &'_ str>>);
with_default_style!(std::rc::Rc<Cow<'_, &'_ str>>);
with_default_style!(std::cell::Cell<Cow<'_, &'_ str>>);
with_default_style!(std::fmt::Arguments<'_>);

/// Combine styles - LHS is the default, RHS overrides
impl<T> std::ops::Add<T> for Style
where
    T: AsRef<Style>,
{
    type Output = Style;

    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        Self {
            front_color: rhs.front_color.or(self.front_color),
            back_color: rhs.back_color.or(self.back_color),
            frame_color: rhs.frame_color.or(self.frame_color),
            crossed_out: rhs.crossed_out.or(self.crossed_out),
            bold: rhs.bold.or(self.bold),
            italic: rhs.italic.or(self.italic),
            underlined: rhs.underlined.or(self.underlined),
            blink: rhs.blink.or(self.blink),
        }
    }
}
/// Combine styles - LHS is the default, RHS overrides
impl AsRef<Style> for Style {
    fn as_ref(&self) -> &Style {
        self
    }
}
/// Combine styles - LHS is the default, RHS overrides
impl<T> std::ops::Add<T> for &Style
where
    T: AsRef<Style>,
{
    type Output = Style;

    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        *self + rhs
    }
}
/// Combine styles - LHS is the default, RHS overrides
impl<T> std::ops::Add<T> for &mut Style
where
    T: AsRef<Style>,
{
    type Output = Style;

    fn add(self, rhs: T) -> Self::Output {
        let rhs = rhs.as_ref();
        *self + rhs
    }
}
/// Combine styles - LHS is the default, RHS overrides
impl<T> std::ops::AddAssign<T> for Style
where
    T: AsRef<Style>,
{
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs.as_ref()
    }
}

/// A styled string slice or owned string.
pub type StylishStringy<'a> = Pretty<Cow<'a, str>>;
impl<'a> From<Pretty<&'a str>> for StylishStringy<'a> {
    fn from(content: Pretty<&'a str>) -> Self {
        StylishStringy {
            style: content.style,
            content: Cow::Borrowed(content.content),
        }
    }
}
impl<'a> From<Pretty<String>> for StylishStringy<'a> {
    fn from(content: Pretty<String>) -> Self {
        StylishStringy {
            style: content.style,
            content: Cow::Owned(content.content),
        }
    }
}
impl<'a> From<&'a str> for StylishStringy<'a> {
    fn from(content: &'a str) -> Self {
        let style = Style::default();
        StylishStringy {
            style,
            content: Cow::Borrowed(content),
        }
    }
}
impl<'a> From<String> for StylishStringy<'a> {
    fn from(content: String) -> Self {
        let style = Style::default();
        StylishStringy {
            style,
            content: Cow::Owned(content),
        }
    }
}

#[test]
fn test_str() {
    assert_eq!(
        "nice".front(Color::blue(true)),
        Pretty {
            content: "nice",
            style: Style {
                front_color: Some(Color::blue(true)),
                back_color: None,
                frame_color: None,
                crossed_out: None,
                bold: None,
                italic: None,
                blink: None,
                underlined: None
            }
        }
    );
}

#[test]
fn test_pretty() {
    assert_eq!(
        Pretty::new("content").front(Color::blue(true)),
        Pretty {
            content: "content",
            style: Style {
                front_color: Some(Color::blue(true)),
                back_color: None,
                frame_color: None,
                crossed_out: None,
                bold: None,
                italic: None,
                blink: None,
                underlined: None
            }
        }
    );
}

#[test]
fn test_mut_string() {
    let mut content = "content".to_owned();
    let mut content2 = "content".to_owned();

    let content_mut = &mut content;
    assert_eq!(
        content_mut.front(Color::blue(true)).back(Color::black()),
        Pretty {
            content: &mut content2,
            style: Style {
                front_color: Some(Color::blue(true)),
                back_color: Some(Color::ansi(0)),
                frame_color: None,
                crossed_out: None,
                bold: None,
                italic: None,
                blink: None,
                underlined: None
            }
        }
    );
}

#[test]
fn test_style_copy() {
    assert_eq!(
        Style::DEFAULT.front(Color::blue(true)),
        Style {
            front_color: Some(Color::blue(true)),
            back_color: None,
            frame_color: None,
            crossed_out: None,
            bold: None,
            italic: None,
            blink: None,
            underlined: None
        }
    );
}
