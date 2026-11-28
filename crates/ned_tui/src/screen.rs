use crate::{
    geometry::{point, Point},
    output::{
        ansi::{AnsiPrint, ScreenStyle},
        command::{ClearScreen, MoveCursorBy, MoveCursorTo, ResetStyles, TerminalCommander},
    },
    prelude::Window,
};

use crate::log::{trace, warn};
use std::{cell::RefCell, io};

/// Terminal screen buffer
///
/// It helps prevent unnecessary trafic to the terminal:
/// - collect all the painting in one cycle, then print the result
/// - print only the diff between last and new cycle
///
/// You probably won't use it directly and instead use the [`crate::Painter`].
/// If you do, however, it should be fairly straightforward.
#[derive(Debug, Default)]
pub struct Screen {
    width: u16,
    height: u16,
    /// Whether to wipe the screen completely on print.
    pub wipe: bool,
    /// Whether truecolor (24-bit RGB) is enabled.
    pub true_color: bool,
    grow_taller: u16,
    last_write: Option<usize>,
    current: Vec<ScreenSpot>,
    previous: RefCell<Vec<ScreenSpot>>,
}

impl Screen {
    /// New screen buffer of the given size
    #[must_use]
    pub fn new(size: Point) -> Self {
        let mut me = Self::default();
        me.resize(size);
        me
    }
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.current.clear();
        self.previous.get_mut().clear();
        self.last_write = None;
    }
    /// Resizes screen buffer to target dimensions.
    pub fn resize(&mut self, size: Point) {
        let height = size.row;
        let width = size.col;
        self.grow_taller = height.saturating_sub(self.height);
        //TODO: move stuff around instead?
        let capacity = (width as usize)
            .checked_mul(height as usize)
            .expect("buffer resize");
        trace!("Resizing the screen to {}x{}={}", width, height, capacity);
        self.width = width;
        self.height = height;
        self.current = Vec::with_capacity(capacity);
        self.previous = RefCell::new(Vec::with_capacity(capacity));
        self.clear();
    }
    /// Returns current screen dimensions as a Point.
    pub fn size(&self) -> Point {
        point(self.width, self.height)
    }
    /// Set the position of the last write
    pub fn set_last_write(&mut self, pos: Point) {
        self.last_write = Some((pos.row * self.width + pos.col) as usize);
    }
    /// Get the position of the last write
    pub fn last_write(&self) -> Option<Point> {
        self.last_write.map(|idx| {
            point(
                (idx % self.width as usize) as u16,
                (idx / self.width as usize) as u16,
            )
        })
    }
    /// Get a symbol at given coordinates
    ///
    /// If the coordinates are out of bounds or if the symbol is not set
    /// you get None
    pub fn get(&self, point: Point) -> Option<(&ScreenStyle, &str)> {
        let Point { col, row } = point;
        self.get_idx((row * self.width + col) as usize)
    }
    /// Get a symbol at given order index
    ///
    /// If the coordinates are out of bounds or if the symbol is not set
    /// you get None
    pub fn get_idx(&self, idx: usize) -> Option<(&ScreenStyle, &str)> {
        if idx > self.size().area() as usize {
            warn!("looking outside of screen @{idx}");
        }
        if idx >= self.current.len() {
            None
        } else {
            let s = &self.current[idx];
            if s.is_empty() {
                None
            } else {
                Some((s.style().unwrap_or(&ScreenStyle::DEFAULT), s.symbol()))
            }
        }
    }
    /// Set a symbol at given coordinates
    ///
    /// If the coordinates are out of bounds the symbol is ignored
    pub fn set(&mut self, point: Point, style: impl Into<ScreenStyle>, symbol: impl AsRef<str>) {
        let Point { col, row } = point;
        if col >= self.width {
            trace!("drawing outside of screen @{col}x{row}");
            return;
        }
        if row >= self.height {
            trace!("drawing outside of screen @{col}x{row}");
            return;
        }
        self.set_idx(
            row as usize * self.width as usize + col as usize,
            style,
            symbol,
        );
    }
    /// Set a symbol at given order index
    ///
    /// If the coordinates are out of bounds the symbol is ignored
    pub fn set_idx(&mut self, idx: usize, style: impl Into<ScreenStyle>, symbol: impl AsRef<str>) {
        if idx >= self.size().area() as usize {
            trace!("drawing outside of screen @{idx}");
            return;
        }
        let owned = ScreenSpot::new(style.into(), symbol.as_ref());

        if idx >= self.current.len() {
            self.current.resize(idx, ScreenSpot::default());
            self.current.push(owned);
        } else {
            self.current[idx] = owned;
        }

        self.last_write = Some(idx);
    }
    /// Dump all the currently set screen symbols
    pub fn symbols(&self) -> impl Iterator<Item = (Point, &ScreenStyle, &str)> + '_ {
        self.current.iter().enumerate().filter_map(|(i, s)| {
            if s.is_empty() {
                None
            } else {
                Some((
                    point(i as u16 % self.width, i as u16 / self.width),
                    s.style().unwrap_or(&ScreenStyle::DEFAULT),
                    s.symbol(),
                ))
            }
        })
    }

    /// Returns iterator yielding modified cells between current and previous frame.
    pub fn diffs(
        &mut self,
        scope: Option<Window>,
        refresh: bool,
    ) -> impl Iterator<Item = (Point, &ScreenStyle, &str)> {
        DiffIterator::new(self, scope, refresh)
    }
    fn diff_spot(&self, spot: Point, refresh: bool) -> Option<(&ScreenStyle, &str)> {
        if !self.size().contains(&spot) {
            //out of range
            return None;
        }
        let idx = (spot.col + spot.row * self.width) as usize;
        let curr = self.current.get(idx).unwrap_or(&ScreenSpot::EMPTY);
        {
            let prev = self.previous.borrow();
            let prev = prev.get(idx).unwrap_or(&ScreenSpot::EMPTY);

            if prev == curr && !refresh {
                return None;
            }
        };

        self.remember(idx, curr.clone());

        Some((curr.style().unwrap_or(&ScreenStyle::DEFAULT), curr.symbol()))
    }

    // A little hack with interior mutability.
    // It stores the previous screen spot in a cache
    // so we do not care too much about it's correctness
    // as the cache is only used to detect changes.
    // All this because of missing 'a iterator support
    fn remember(&self, idx: usize, spot: ScreenSpot) {
        let mut remember = self.previous.borrow_mut();
        let grow = idx >= remember.len();
        if grow {
            remember.resize(idx, ScreenSpot::Empty);
            remember.push(spot);
        } else {
            remember[idx] = spot;
        }
    }

    pub(crate) fn print(
        &mut self,
        scope: Option<Window>,
        mut commander: impl TerminalCommander,
    ) -> io::Result<()> {
        // clear space for drawing
        let mut busy = false;
        if self.wipe && self.grow_taller != 0 {
            busy = true;
            trace!("clearing screen");
            commander.send(MoveCursorBy(0, self.height as i16))?;
            for _ in (0..self.grow_taller).skip(1) {
                commander.send(AnsiPrint("\n").raw(true))?;
            }
            commander.send(ResetStyles)?;
            commander.send(ClearScreen)?;
        }
        self.grow_taller = 0;
        let true_color = self.true_color;
        let mut last_pos = point(0, 0);
        let mut last_style = ScreenStyle::DEFAULT;
        let mut last_was_wide = false;
        for (pos, style, symbol) in self.diffs(scope, false) {
            busy = true;
            if pos.col != last_pos.col.saturating_add(1) || pos.row != last_pos.row || last_was_wide
            {
                commander.send(MoveCursorTo(pos))?;
            }
            last_pos = pos;
            last_was_wide = symbol.len() != 1;

            if style != &last_style {
                let print = if true_color {
                    AnsiPrint(style).true_color()
                } else {
                    AnsiPrint(style).compatible_color()
                };
                commander.send(print)?;
                last_style = *style;
            }

            commander.send(AnsiPrint(symbol))?;
        }
        if busy {
            commander.send(MoveCursorBy(self.width as i16, self.height as i16))?;
        }
        Ok(())
    }
}

struct DiffIterator<'a> {
    pub screen: &'a Screen,
    pub scope: Window,
    refresh: bool,
    idx: usize,
}

impl<'a> DiffIterator<'a> {
    pub fn new(screen: &'a mut Screen, scope: Option<Window>, mut refresh: bool) -> Self {
        // forced refresh or refresh on init
        refresh |= screen.previous.borrow().is_empty();

        let scope = {
            let my_scope = screen.size().window();
            scope.map_or(my_scope, |scope| my_scope.relative_crop(scope))
        };

        Self {
            screen,
            scope,
            refresh,
            idx: 0,
        }
    }
}
impl<'a> Iterator for DiffIterator<'a> {
    type Item = (Point, &'a ScreenStyle, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.idx += 1;
            let pos = self.idx - 1;
            if pos >= self.scope.size().area() as usize {
                return None;
            }
            // adjust position to scope
            let pos = self.scope.position().area() as usize
                + self.screen.width as usize * (pos / self.scope.width as usize)
                + pos % self.scope.width as usize;

            let spot = point(
                (pos % self.screen.width as usize) as u16,
                (pos / self.screen.width as usize) as u16,
            );
            // check if it's within our scope
            let spot = (self.scope.contains_point(spot))?;

            if let Some((style, symbol)) = self.screen.diff_spot(spot, self.refresh) {
                break Some((spot, style, symbol));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
enum ScreenSpot {
    #[default]
    Empty,
    /// str pointer is usually larger than a few bytes
    /// This one avoids heap for short utf-8 symbols
    Short(ScreenStyle, u8, [u8; 14]),
    Long(ScreenStyle, Box<str>),
}

impl ScreenSpot {
    pub const EMPTY: ScreenSpot = ScreenSpot::Empty;
    pub fn new(style: ScreenStyle, symbol: &str) -> Self {
        let len = symbol.len();
        if len == 0 {
            ScreenSpot::Empty
        } else if len <= 14 {
            let mut short = [0; 14];
            short[0..len].copy_from_slice(symbol.as_bytes());
            ScreenSpot::Short(style, len as u8, short)
        } else {
            ScreenSpot::Long(style, symbol.into())
        }
    }
    pub fn style(&self) -> Option<&ScreenStyle> {
        match &self {
            ScreenSpot::Empty => None,
            ScreenSpot::Short(s, _, _) | ScreenSpot::Long(s, _) => Some(s),
        }
    }
    pub fn symbol(&self) -> &str {
        match &self {
            ScreenSpot::Empty => " ",
            ScreenSpot::Short(_, len, bytes) => {
                std::str::from_utf8(&bytes[0..*len as usize]).expect("valid symbol char")
            }
            ScreenSpot::Long(_, s) => s,
        }
    }
    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[test]
fn test_symbols() {
    let style = ScreenStyle::DEFAULT;
    let symbol = "x".to_owned();
    let mut screen = Screen::new(point(3, 2));
    screen.set(point(1, 1), style, &symbol);
    let symbols = screen.symbols().collect::<Vec<_>>();
    assert_eq!(symbols, vec![(point(1, 1), &style, symbol.as_str())]);
}

#[test]
fn test_spots_size() {
    assert_eq!(std::mem::size_of::<ScreenSpot>(), 32);
}

#[test]
fn test_diff_one() {
    let mut screen = Screen::default();
    screen.resize(point(10, 10));

    screen.set(point(1, 1), ScreenStyle::DEFAULT, "1");
    screen.set(point(2, 2), ScreenStyle::DEFAULT, "2");
    assert_eq!(screen.diff_spot(point(0, 0), false), None);
    assert_eq!(
        screen.diff_spot(point(1, 1), false),
        Some((&ScreenStyle::DEFAULT, "1"))
    );
}

#[test]
fn test_diff_multiple() {
    let mut screen = Screen::default();
    screen.resize(point(10, 10));

    // this will fill the cache
    screen.set(point(9, 9), ScreenStyle::DEFAULT, "9");
    screen.diffs(None, false).fold((), |(), _| ());

    // this is the test
    screen.set(point(1, 1), ScreenStyle::DEFAULT, "1");
    screen.set(point(2, 2), ScreenStyle::DEFAULT, "2");
    let mut a = screen.diffs(None, false);
    assert_eq!(a.next(), Some((point(1, 1), &ScreenStyle::DEFAULT, "1")));
    assert_eq!(a.next(), Some((point(2, 2), &ScreenStyle::DEFAULT, "2")));
    assert_eq!(a.next(), None);
}
