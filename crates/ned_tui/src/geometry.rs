//! 2D screen structures and arithmetics

use std::{borrow::Borrow, fmt, ops};

/// Represents a 2D rectangle in a terminal screen
#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Window {
    /// screen cell left position, zero based
    pub col: u16,
    /// screen cell top position, zero based
    pub row: u16,
    /// screen cell width
    pub width: u16,
    /// screen cell height
    pub height: u16,
}
/// Represents 2D coordinates in a terminal screen
#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub struct Point {
    /// screen cell left position or width, zero based
    pub col: u16,
    /// screen cell top position or height, zero based
    pub row: u16,
}
impl From<(u16, u16)> for Point {
    fn from(p: (u16, u16)) -> Self {
        point(p.0, p.1)
    }
}
/// New 2D coordinates for a terminal screen
pub fn point(col: u16, row: u16) -> Point {
    Point { col, row }
}
/// New 2D rectangle for a terminal screen
pub fn window(position: Point, size: Point) -> Window {
    Window {
        col: position.col,
        row: position.row,
        width: size.col,
        height: size.row,
    }
}

impl Point {
    /// create a window the size of the point position
    pub fn window(&self) -> Window {
        Window {
            col: 0,
            row: 0,
            width: self.col,
            height: self.row,
        }
    }
    /// The total number of discrete points on the terminal screen
    pub fn area(&self) -> u32 {
        self.col as u32 * self.row as u32
    }
    /// Is the given coordinate smaller or equal to self?
    pub fn contains(&self, o: &Point) -> bool {
        self.col > o.col && self.row > o.row
    }

    /// Check if the point appears withing the given window
    pub fn is_in<T: Borrow<Window>>(&self, w: T) -> bool {
        let w = w.borrow();
        w.col <= self.col
            && w.row <= self.row
            && w.col.saturating_add(w.width) > self.col
            && w.row.saturating_add(w.height) > self.row
    }
    /// What is the col/row coordinate of the nth point within the bounds of self?
    pub fn point_at_idx(&self, idx: u32) -> Option<Point> {
        if idx >= self.area() {
            None
        } else {
            let col = (idx % self.col as u32) as u16;
            let row = (idx / self.col as u32) as u16;
            Some(point(col, row))
        }
    }
    /// What is the order index of the given coordinates within the bounds of self?
    pub fn idx_at_point(&self, p: Point) -> Option<u32> {
        if p.row >= self.row || p.col >= self.col {
            None
        } else {
            let w = self.col as u32;
            let x = p.col as u32;
            let y = p.row as u32;
            Some(y * w + x)
        }
    }
}
impl Window {
    /// Crop to a relative overlapping subset of the window.
    ///
    /// ```diagram
    ///                   my row
    ///                   |
    ///                   v
    ///               |--------------------|
    ///               |              +     |
    ///               | self         +row  |
    ///               |              +     |
    ///  my col ----> |              v     |
    ///               |           {================}
    ///               |++col++++> {XXXXXXXX|       }
    ///               |           {XresultX|       }
    ///               |           {XXXXXXXX|       }
    ///               |-----------{--------|       }
    ///                           {                }
    ///                           { relative other }
    ///                           {================}
    /// ```
    pub fn relative_crop(&self, w: impl Borrow<Window>) -> Window {
        let w = w.borrow() + self.position();
        self.intersection(w)
    }
    /// Crop to an absolute overlapping subset of the window.
    ///
    /// ```diagram
    ///
    ///                   my row     other row
    ///                   |          +
    ///                   v          +
    ///               |--------------+-----|
    ///               |              +     |
    ///               | self         +     |
    ///               |              +     |
    ///  my col ----> |              v     |
    ///               |           {================}
    ///  other col +++++++++++++> {XXXXXXXX|       }
    ///               |           {XresultX|       }
    ///               |           {XXXXXXXX|       }
    ///               |-----------{--------|       }
    ///                           {                }
    ///                           { absolute other }
    ///                           {================}
    /// ```
    pub fn intersection(&self, w: impl Borrow<Window>) -> Window {
        let o = w.borrow();
        let pos = point(u16::max(self.col, o.col), u16::max(self.row, o.row));
        window(
            pos,
            point(
                u16::min(
                    self.col.saturating_add(self.width).saturating_sub(pos.col),
                    o.col.saturating_add(o.width).saturating_sub(pos.col),
                ),
                u16::min(
                    self.row.saturating_add(self.height).saturating_sub(pos.row),
                    o.row.saturating_add(o.height).saturating_sub(pos.row),
                ),
            ),
        )
    }
    /// Check if the given point appears withing the window
    /// and if it does, return same, otherwise none
    pub fn contains_point<T: Borrow<Point>>(&self, p: T) -> Option<T> {
        if p.borrow().is_in(self) {
            Some(p)
        } else {
            None
        }
    }
    /// Restrict the window to given space
    ///
    /// ```diagram
    ///
    ///                  my row     trim col
    ///                  |          +
    ///                  v          +
    ///               XXXXXXXXXXXXXX+------
    ///               XXXXXXXXXXXXXX+     |
    ///               XXXXXXXXXXXXXX+     |
    ///               XXXXresultXXXX+     |
    ///  my col ----> XXXXXXXXXXXXXX+     |
    ///               XXXXXXXXXXXXXX+     |
    ///  trim row +++++++++++++++++++     |
    ///               |                   |
    ///               |             self  |
    ///               ---------------------
    ///
    /// ```
    pub fn trim(&self, p: impl Borrow<Point>) -> Window {
        self.intersection(p.borrow().window())
    }
    /// The col/row coordinates of the bottom right corner
    pub fn bottom_right(&self) -> Point {
        point(
            self.col.saturating_add(self.width),
            self.row.saturating_add(self.height),
        )
    }
    /// The col/row coordinates of the top left corner
    pub fn position(&self) -> Point {
        point(self.col, self.row)
    }
    /// The width/height coordinates of the rectangle
    pub fn size(&self) -> Point {
        point(self.width, self.height)
    }
    /// If the window has zero area
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c{} r{} ", self.col, self.row)
    }
}
impl fmt::Display for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "c{} r{} w{} h{}",
            self.col, self.row, self.width, self.height
        )
    }
}

impl<RHS: Borrow<Window>> ops::BitAnd<RHS> for Window {
    type Output = Window;
    /// get the intersection of the two windows
    fn bitand(self, rhs: RHS) -> Self::Output {
        (&self).bitand(rhs)
    }
}
impl<RHS: Borrow<Window>> ops::BitAnd<RHS> for &Window {
    type Output = Window;
    /// get the intersection of the two windows
    fn bitand(self, o: RHS) -> Self::Output {
        self.intersection(o)
    }
}

impl ops::SubAssign for Point {
    /// move point closer by given coordinates
    fn sub_assign(&mut self, o: Self) {
        *self = *self - o
    }
}
impl ops::AddAssign for Point {
    /// move point away by given coordinates
    fn add_assign(&mut self, o: Self) {
        *self = *self + o
    }
}
impl<RHS: Borrow<Point>> ops::Sub<RHS> for Point {
    type Output = Point;
    /// move point closer by given coordinates
    fn sub(self, o: RHS) -> Self::Output {
        (&self).sub(o)
    }
}
impl<RHS: Borrow<Point>> ops::Sub<RHS> for &Point {
    type Output = Point;
    /// move point closer by given coordinates
    fn sub(self, o: RHS) -> Self::Output {
        let o = o.borrow();
        point(
            self.col.saturating_sub(o.col),
            self.row.saturating_sub(o.row),
        )
    }
}
impl<RHS: Borrow<Point>> ops::Add<RHS> for Point {
    type Output = Point;
    /// move point away by given coordinates
    fn add(self, o: RHS) -> Self::Output {
        (&self).add(o)
    }
}
impl<RHS: Borrow<Point>> ops::Add<RHS> for &Point {
    type Output = Point;
    /// move point away by given coordinates
    fn add(self, o: RHS) -> Self::Output {
        let o = o.borrow();
        point(
            self.col.saturating_add(o.col),
            self.row.saturating_add(o.row),
        )
    }
}

impl ops::AddAssign<Point> for Window {
    /// move window away by given coordinates
    fn add_assign(&mut self, o: Point) {
        *self = *self + o
    }
}
impl ops::SubAssign<Point> for Window {
    /// move window closer by given coordinates
    fn sub_assign(&mut self, o: Point) {
        *self = *self - o
    }
}

impl<RHS: Borrow<Point>> ops::Sub<RHS> for Window {
    type Output = Window;
    /// move window closer by given coordinates
    fn sub(self, o: RHS) -> Self::Output {
        (&self).sub(o)
    }
}
impl<RHS: Borrow<Point>> ops::Sub<RHS> for &Window {
    type Output = Window;
    /// move window closer by given coordinates
    fn sub(self, o: RHS) -> Self::Output {
        let o = o.borrow();
        window(
            point(
                self.col.saturating_sub(o.col),
                self.row.saturating_sub(o.row),
            ),
            self.size(),
        )
    }
}
impl<RHS: Borrow<Point>> ops::Add<RHS> for Window {
    type Output = Window;
    /// move window away by given coordinates
    fn add(self, o: RHS) -> Self::Output {
        (&self).add(o)
    }
}
impl<RHS: Borrow<Point>> ops::Add<RHS> for &Window {
    type Output = Window;
    /// move window away by given coordinates
    fn add(self, o: RHS) -> Self::Output {
        let o = o.borrow();
        window(
            point(
                self.col.saturating_add(o.col),
                self.row.saturating_add(o.row),
            ),
            self.size(),
        )
    }
}

#[test]
fn test_window_relative_crop() {
    let op = window(point(0, 0), point(10_000, 30_000));
    let crop = window(point(10, 3), point(100, 300)).relative_crop(op);
    assert_eq!(crop.col, (10));
    assert_eq!(crop.row, (3));
    assert_eq!(crop.width, (100));
    assert_eq!(crop.height, (300));

    let op = window(point(10, 3), point(100, 300));
    let crop = window(point(0, 0), point(10_000, 30_000)).relative_crop(op);
    assert_eq!(crop.col, (10));
    assert_eq!(crop.row, (3));
    assert_eq!(crop.width, (100));
    assert_eq!(crop.height, (300));
}

#[test]
fn test_window_intersection() {
    let a = window(point(50, 30), point(5, 3));
    let b = window(point(51, 31), point(100, 300));
    let crop = a.intersection(b);
    assert_eq!(crop.col, (51));
    assert_eq!(crop.row, (31));
    assert_eq!(crop.width, (4));
    assert_eq!(crop.height, (2));

    let c = window(point(1, 1), point(1, 1));
    let crop = a & c;
    assert_eq!(crop.col, (50));
    assert_eq!(crop.row, (30));
    assert_eq!(crop.width, (0));
    assert_eq!(crop.height, (0));
    let crop_commutative = c & a;
    assert_eq!(crop_commutative, crop)
}

#[test]
fn test_size_contains() {
    let p = point(3, 2);
    assert!(p.contains(&point(0, 0)));
    assert!(p.contains(&point(2, 1)));
    assert!(!p.contains(&point(3, 1)));
    assert!(!p.contains(&point(2, 2)));
}

#[test]
fn test_idx_at_point() {
    let p = point(3, 2);
    assert_eq!(p.idx_at_point(point(0, 0)), Some(0));
    assert_eq!(p.idx_at_point(point(1, 0)), Some(1));
    assert_eq!(p.idx_at_point(point(1, 1)), Some(4));
    assert_eq!(p.idx_at_point(point(2, 1)), Some(5));
    assert_eq!(p.idx_at_point(point(3, 1)), None);
    assert_eq!(p.idx_at_point(point(1, 2)), None);

    assert_eq!(p.point_at_idx(0), Some(point(0, 0)));
    assert_eq!(p.point_at_idx(5), Some(point(2, 1)));
    assert_eq!(p.point_at_idx(6), None);
}
