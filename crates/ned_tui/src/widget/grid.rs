use super::{AnchorPlacementEnabled, AppEvent, Event, Painter, Screen, Widget};
use crate::geometry::{point, window, Point, Window};

/// A grid layout widget
///
/// The number of rows, cells and their widths and heights
/// are determined by occupied cells and the max weight of the cells.
///
/// The plain Grid is for a single inner widget type,
/// but convenience methods are there to support boxed widgets.
pub struct Grid<W> {
    row_weights: Vec<u8>,
    col_weights: Vec<u8>,
    cells: Vec<(usize, usize, W)>,
}
impl<W> Default for Grid<W> {
    fn default() -> Self {
        Self {
            row_weights: Default::default(),
            col_weights: Default::default(),
            cells: Default::default(),
        }
    }
}

impl<W> Grid<W> {
    /// Add a widget to a cell.
    ///
    /// cell: (x,y)
    /// weight: (`x_weigt,y_weight`)
    ///
    /// There can be multiple widgets in one cell.
    /// For each row and column the max value of all cells' weights apply.
    ///
    /// If you just add just one widget to cell 3,5, the grid will have 4 columns and 6 rows.
    pub fn add(mut self, cell: (usize, usize), weight: (u8, u8), widget: W) -> Self {
        let (col, row) = cell;
        let (col_weight, row_weight) = weight;
        if row >= self.row_weights.len() {
            self.row_weights.resize(row + 1, 0);
        }
        if self.row_weights[row] < row_weight {
            self.row_weights[row] = row_weight;
        }

        if col >= self.col_weights.len() {
            self.col_weights.resize(col + 1, 0);
        }
        if self.col_weights[col] < col_weight {
            self.col_weights[col] = col_weight;
        }
        self.cells.push((col, row, widget));
        self
    }
    fn cell_scope(size: Point, element: usize, col_weights: &[u8], row_weights: &[u8]) -> Window {
        let cols = col_weights.len();
        let rows = row_weights.len();

        if element >= cols * rows {
            // the element is outside of the grid, use free layout then
            return size.window();
        }

        let row = element / cols;
        let col = element % cols;

        let cell_x = Self::cell_size(col_weights, col, size.col);
        let cell_y = Self::cell_size(row_weights, row, size.row);

        window(point(cell_x.0, cell_y.0), point(cell_x.1, cell_y.1))
    }

    /// Calculate the start and width of the cell relative to the size
    fn cell_size(grid: &[u8], idx: usize, size: u16) -> (u16, u16) {
        if idx >= grid.len() {
            // beyond the defined cells, take all the space
            return (0, size);
        }

        let sum = grid.iter().fold(0u16, |s, c| s + u16::from(*c));

        let scale = if sum == 0 {
            0f32
        } else {
            f32::from(size) / f32::from(sum)
        };

        let start = grid[..idx]
            .iter()
            .copied()
            .fold(0u16, |s, c| s + (scale * f32::from(c)) as u16);

        let cell = if idx + 1 == grid.len() {
            // special case for last cell that takes the remainder
            size - start
        } else {
            (scale * f32::from(grid[idx])) as u16
        };

        (start, cell)
    }

    /// Find which cell is at the given spot
    #[must_use]
    pub fn get_cell_for_point(&self, point: Point, scope: Window) -> Option<(usize, usize)> {
        for col in 0..self.col_weights.len() {
            for row in 0..self.row_weights.len() {
                let cell_scope = Self::cell_scope(
                    scope.size(),
                    row * self.col_weights.len() + col,
                    &self.col_weights,
                    &self.row_weights,
                ) + scope.position();
                if point.is_in(cell_scope) {
                    return Some((col, row));
                }
            }
        }
        None
    }

    /// Get the calculated scope for a given cell order idx
    #[must_use]
    pub fn get_cell_scope_by_idx(&self, i: usize, scope: Window) -> Window {
        Self::cell_scope(scope.size(), i, &self.col_weights, &self.row_weights) + scope.position()
    }
    /// Get the calculated scope for a given cell
    #[must_use]
    pub fn get_cell_scope(&self, i: (usize, usize), scope: Window) -> Window {
        let (x, y) = i;
        let i = x + y * self.cols();
        self.get_cell_scope_by_idx(i, scope)
    }

    /// get the number of rows in the grid
    #[must_use]
    pub fn rows(&self) -> usize {
        self.row_weights.len()
    }
    /// get the number of columns in the grid
    #[must_use]
    pub fn cols(&self) -> usize {
        self.col_weights.len()
    }
}

impl<'a, M, A: AppEvent> Grid<Box<dyn Widget<M, A> + 'a>> {
    /// Create a Grid with boxed widgets
    #[must_use]
    pub fn boxed() -> Self {
        Self {
            row_weights: Default::default(),
            col_weights: Default::default(),
            cells: Default::default(),
        }
    }
    /// Add a boxed widget to the cell
    pub fn add_box(
        self,
        cell: (usize, usize),
        weight: (u8, u8),
        widget: impl Widget<M, A> + 'a,
    ) -> Self {
        self.add(cell, weight, Box::new(widget))
    }
}
impl<W> AnchorPlacementEnabled for Grid<W> {}
impl<W, M, A: AppEvent> Widget<M, A> for Grid<W>
where
    W: Widget<M, A>,
{
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        for cell in &mut self.cells {
            let (col, row, element) = cell;
            let ord = *row * self.col_weights.len() + *col;
            let cell_scope = Self::cell_scope(
                painter.scope().size(),
                ord,
                &self.col_weights,
                &self.row_weights,
            );
            let absolute_scope = painter.scope().relative_crop(cell_scope);

            element.update_asserted(model, input, screen, &painter.with_scope(absolute_scope));
        }
        painter.scope()
    }
}

#[test]
fn test_layout() {
    let row_weights = vec![100, 100];
    let col_weights = vec![100, 100, 100];

    let size = point(10, 13);
    assert_eq!(
        Grid::<()>::cell_scope(size, 0usize, &col_weights, &row_weights),
        window(point(0, 0), point(3, 6))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 2usize, &col_weights, &row_weights),
        window(point(6, 0), point(4, 6))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 3usize, &col_weights, &row_weights),
        window(point(0, 6), point(3, 7))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 5usize, &col_weights, &row_weights),
        window(point(6, 6), point(4, 7))
    );
}

#[test]
fn test_layout2() {
    let row_weights = vec![50, 50];
    let col_weights = vec![50, 50, 50];
    let size = point(148, 109);
    assert_eq!(
        Grid::<()>::cell_scope(size, 0usize, &col_weights, &row_weights),
        window(point(0, 0), point(49, 54))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 2usize, &col_weights, &row_weights),
        window(point(98, 0), point(50, 54))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 3usize, &col_weights, &row_weights),
        window(point(0, 54), point(49, 55))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 4usize, &col_weights, &row_weights),
        window(point(49, 54), point(49, 55))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 5usize, &col_weights, &row_weights),
        window(point(98, 54), point(50, 55))
    );
}

#[test]
fn test_layout3() {
    let row_weights = vec![50, 50, 50];
    let col_weights = vec![50, 50, 50];
    let size = point(5, 5);
    assert_eq!(
        Grid::<()>::cell_scope(size, 0usize, &col_weights, &row_weights),
        window(point(0, 0), point(1, 1))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 4usize, &col_weights, &row_weights),
        window(point(1, 1), point(1, 1))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 8usize, &col_weights, &row_weights),
        window(point(2, 2), point(3, 3))
    );
}

#[test]
fn test_layout_0() {
    let row_weights = vec![0, 0, 0];
    let col_weights = vec![0, 1, 0];
    let size = point(5, 5);
    assert_eq!(
        Grid::<()>::cell_scope(size, 0usize, &col_weights, &row_weights),
        window(point(0, 0), point(0, 0))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 4usize, &col_weights, &row_weights),
        window(point(0, 0), point(5, 0))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 8usize, &col_weights, &row_weights),
        window(point(5, 0), point(0, 5))
    );
}

#[test]
fn test_layout_free() {
    let row_weights = vec![];
    let col_weights = vec![];
    let size = point(5, 5);
    assert_eq!(
        Grid::<()>::cell_scope(size, 0usize, &col_weights, &row_weights),
        window(point(0, 0), point(5, 5))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 4usize, &col_weights, &row_weights),
        window(point(0, 0), point(5, 5))
    );
    assert_eq!(
        Grid::<()>::cell_scope(size, 8usize, &col_weights, &row_weights),
        window(point(0, 0), point(5, 5))
    );
}
