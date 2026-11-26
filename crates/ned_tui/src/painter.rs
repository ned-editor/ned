use super::Screen;
use crate::{output::ansi::ScreenStyle, prelude::*, unicode::{Graphemes, grapheme_width, str_graphemes}};

/// The TUI Painter :)
///
/// It has a style and scope and it paints text on the screen
/// or fills it with background. It can also copy symbols from
/// one screen to another.
///
#[derive(Debug, Clone)]
pub struct Painter {
    scope: Window,
    style: Style,
}
impl Default for Painter {
    /// Default style, unlimited scope
    fn default() -> Self {
        Self {
            scope: point(u16::MAX, u16::MAX).window(),
            style: Default::default(),
        }
    }
}
impl Painter {
    /// Creates a painter with style and clipping scope.
    pub fn new(style: impl Into<Style>, scope: Window) -> Self {
        Self {
            scope,
            style: style.into(),
        }
    }
    /// Set new scope
    pub fn with_scope(&self, scope: Window) -> Painter {
        Self::new(self.style, scope)
    }
    /// Ignore the current style and use the given one
    pub fn with_new_style(&self, style: Style) -> Painter {
        Self::new(style, self.scope)
    }
    /// Returns active clipping window scope.
    pub fn scope(&self) -> Window {
        self.scope
    }
    /// Copy symbols to the screen respecting scope
    pub fn copy<'a>(
        &self,
        symbols: impl Iterator<Item = (Point, &'a ScreenStyle, &'a str)>,
        screen: &mut Screen,
    ) {
        for (pos, style, symbol) in symbols {
            let pos = pos + self.scope.position();
            screen.set(pos, *style, symbol);
        }
    }
    /// Render the given text to the screen with a column offset.
    ///
    /// The offset allows to continue a line where the previous painting stopped.
    /// The offest is zero-based relative to the painter scope.
    ///
    /// With the explicit_carriage_return set to true, new line (\n) will not imply carriage return (\r).
    /// Set it to true if you fave pre-formatted text where \n should not return carriage.
    /// False suitable for most texts in the nix world and no harm in \r\n world.
    /// Set it to false if you want text to continue at the beginning after new line.
    ///
    /// It handles the text in a special way:
    /// - it works with symbols-graphemes rather than chars
    /// - each grapheme will occupy it's own spot on the screen (emoji welcome)
    /// - \r moves the cursor to the start of the scope line
    /// - \n moves the cursor to the next line in scope
    /// - if end of scope is reached, wraps around to next line
    /// - it skips symbols that are out of scope
    /// - it returns the actual window scope the text was rendered to
    ///   which allows the next thing to lay next by
    pub fn paint(
        &self,
        text: &str,
        screen: &mut Screen,
        offset: u16,
        explicit_carriage_return: bool,
    ) -> Window {
        let scope = self.scope.trim(screen.size());

        if scope.position().is_in(self.scope) {
            // start with negative resulting scope
            let mut left = scope.col.saturating_add(scope.width);
            let mut top = scope.row.saturating_add(scope.height);
            let mut right = scope.col;
            let mut bottom = scope.row;
            for (at, symbol) in text_symbols(&text, scope.size(), offset, !explicit_carriage_return)
            {
                let at = at + self.scope.position();
                if at.is_in(self.scope) {
                    let mut style = *self.style();
                    // preserve back color if not set
                    if style.back_color.is_none() {
                        if let Some((cur_style, _)) = screen.get(at) {
                            style = style.back(cur_style.back_color());
                        }
                    }

                    // only draw what's in scope
                    screen.set(at, &style, symbol);

                    right = u16::max(right, at.col);
                    bottom = u16::max(bottom, at.row);
                    left = u16::min(left, at.col);
                    top = u16::min(top, at.row);
                }
            }
            if left <= right && top <= bottom {
                window(point(left, top), point(right - left + 1, bottom - top + 1))
            } else {
                scope.trim(point(0, 0))
            }
        } else {
            // ignore text beyond our scope altogether
            //trace!("Text out of scope {:?} - {:?}", scope.position(), text);
            scope.trim(point(0, 0))
        }
    }
    /// Fill the scope in the screen with background color
    pub fn fill(&self, screen: &mut Screen) {
        for row in self.scope.row..self.scope.row + self.scope.height {
            for col in self.scope.col..self.scope.col + self.scope.width {
                screen.set(point(col, row), &self.style, " ")
            }
        }
    }
}

impl Stylish for Painter {
    fn style(&self) -> &Style {
        &self.style
    }
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}
impl Stylish for &mut Painter {
    fn style(&self) -> &Style {
        &self.style
    }
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}
impl Styled for Painter {
    type Cute = Self;

    fn pretty(self) -> Self::Cute {
        self
    }
}
impl Styled for &Painter {
    type Cute = Painter;

    fn pretty(self) -> Self::Cute {
        self.clone()
    }
}
impl Styled for &mut Painter {
    type Cute = Self;

    fn pretty(self) -> Self::Cute {
        self
    }
}

struct TextSymbols<'a> {
    return_carriage_on_new_line: bool,
    size: Point,
    graphemes: Graphemes<'a>,
    pos: Point,
    wrapped: bool,
    wipe: usize,
}

impl<'a> Iterator for TextSymbols<'a> {
    type Item = (Point, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        if self.wipe != 0 {
            const WIPER: &str = "                                                              ";
            let wiper = &WIPER[0..self.wipe];
            self.wipe = 0;
            return Some((self.pos, wiper));
        }

        while let (Some(grapheme), true) = (self.graphemes.next(), self.size.contains(&self.pos)) {
            match grapheme {
                "\r\n" | "\n\r" => {
                    if !self.wrapped {
                        self.pos.col = 0;
                        self.pos.row += 1;
                    }
                    self.wrapped = false;
                }
                "\r" => {
                    self.pos.col = 0;
                    self.wrapped = false;
                }
                "\n" => {
                    if self.return_carriage_on_new_line {
                        self.pos.col = 0;
                    }
                    if !self.wrapped {
                        self.pos.row += 1;
                    }
                    self.wrapped = false;
                }
                grapheme => {
                    let (display_width, char_width) = display_width_char(grapheme);
                    self.wipe = char_width.saturating_sub(1);
                    self.wrapped =
                        self.pos.col.saturating_add(display_width as u16) >= self.size.col;

                    if let Some(idx) = self.size.idx_at_point(self.pos) {
                        let stroke = (self.pos, grapheme);

                        // set next position
                        if let Some(next) = self.size.point_at_idx(idx + display_width as u32) {
                            self.pos = next;
                        } else {
                            // beyond scope so next loop will be None
                            self.pos += point(1, 1)
                        }

                        return Some(stroke);
                    } else {
                        // ignore graphemes not in scope
                        // but quit early if over the bottom
                        if self.pos.row > self.size.row {
                            return None;
                        }
                    }
                }
            }
        }
        None
    }
}

/// Credit: https://github.com/tombruijn @ https://github.com/lintje/lintje/blob/501aab06e19008e787237438a69ac961f38bb4b7/src/utils.rs#L22-L71
/// Calculate the render width of a single Unicode character. Unicode characters may consist of
/// multiple String characters, which is why the function argument takes a string.
fn display_width_char(string: &str) -> (usize, usize) {
    let width = grapheme_width(string);
    (width, width)
}

fn text_symbols<'a>(
    text: &'a impl AsRef<str>,
    size: Point,
    offset: u16,
    cr_on_lf: bool,
) -> TextSymbols<'a> {
    let content = text.as_ref();
    let graphemes = str_graphemes(content);
    let pos = if size.col == 0 {
        point(0, 0)
    } else {
        point(offset % size.col, offset / size.col)
    };
    TextSymbols {
        return_carriage_on_new_line: cr_on_lf,
        pos,
        size,
        graphemes,
        wrapped: false,
        wipe: 0,
    }
}

#[test]
fn test_text_symbols() {
    let symbols = text_symbols(&"12\r\n4567", point(3, 2), 0, false).collect::<Vec<_>>();
    assert_eq!(symbols[0].1, "1");
    assert_eq!(symbols[1].1, "2");
    assert_eq!(symbols[2].1, "4");
    assert_eq!(symbols[3].1, "5");
    assert_eq!(symbols[4].1, "6");
    assert_eq!(symbols.len(), 5);
}
#[test]
fn test_text_symbols_offset() {
    let symbols = text_symbols(&"12\r\n4567", point(3, 2), 1, false).collect::<Vec<_>>();
    assert_eq!(symbols[0], (point(1, 0), "1"));
    assert_eq!(symbols[1], (point(2, 0), "2"));
    assert_eq!(symbols[2], (point(0, 1), "4"));
    assert_eq!(symbols[3], (point(1, 1), "5"));
    assert_eq!(symbols[4], (point(2, 1), "6"));
    assert_eq!(symbols.len(), 5);
}
#[test]
fn test_text_symbols_fit() {
    let symbols = text_symbols(&"123\r\n4567", point(3, 2), 0, false).collect::<Vec<_>>();
    assert_eq!(symbols[0].1, "1");
    assert_eq!(symbols[1].1, "2");
    assert_eq!(symbols[2].1, "3");
    assert_eq!(symbols[3].1, "4");
    assert_eq!(symbols[4].1, "5");
    assert_eq!(symbols[5].1, "6");
    assert_eq!(symbols.len(), 6);
}

#[test]
fn test_text_size() {
    let mut big_screen = Screen::new(point(80, 80));
    let big_scope = big_screen.size().window();
    let small_style = Style::DEFAULT;
    let painter = Painter::new(small_style, big_scope);

    let scope = painter.paint("12\n3", &mut big_screen, 0, true);
    assert_eq!(scope, point(3, 2).window());

    let scope = painter.paint("12\r3", &mut big_screen, 0, true);
    assert_eq!(scope, point(2, 1).window());

    let scope = painter.paint("12\r\n3", &mut big_screen, 0, true);
    assert_eq!(scope, point(2, 2).window());
}

#[test]
fn display_width() {
    // Emoji occupy two columns, whether they get there through an emoji
    // presentation selector, a zero width joiner sequence or a skin tone
    // modifier. `unicode_width` reports that width directly since 0.1.11, so
    // the second element of the pair no longer exceeds the first; earlier
    // releases summed the components and these expectations read (1, 1),
    // (2, 4) and (2, 8) respectively.
    assert_eq!("❤️".len(), 6);
    // U+2764 is narrow on its own, U+FE0F requests emoji presentation.
    assert_eq!(display_width_char("\u{2764}"), (1, 1));
    assert_eq!(display_width_char("❤️"), (2, 2));
    assert_eq!(display_width_char("✅"), (2, 2));
    // Zero width joiner sequences.
    assert_eq!(display_width_char("\u{1F469}\u{200D}\u{1F52C}"), (2, 2));
    assert_eq!(
        display_width_char("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F467}"),
        (2, 2)
    );
    // Skin tone modifier.
    assert_eq!(display_width_char("\u{1F44B}\u{1F3FD}"), (2, 2));
    // The modifiers carry no width by themselves.
    assert_eq!(display_width_char("\u{200D}"), (0, 0));
    assert_eq!(display_width_char("\u{FE0F}"), (0, 0));
    // Tab is rendered rather than measured.
    assert_eq!(display_width_char("\t"), (4, 4));
}

#[test]
fn graphemeis() {
    assert_eq!("❤️".graphemes(true).count(), 1);
    assert_eq!("✅".graphemes(true).count(), 1);
    assert_eq!("\u{1F469}\u{200D}\u{1F52C}".graphemes(true).count(), 1);
}
