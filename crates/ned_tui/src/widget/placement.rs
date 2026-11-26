use crate::geometry::*;
use crate::widget::*;

/// Implement this trait to easily enable placement for a widget
/// by wrapping it in [PlacedWidget] decorator.
pub trait AnchorPlacementEnabled
where
    Self: Sized,
{
    /// Enabling method called by the related [AnchorPlaced] impl
    fn place(self) -> PlacedWidget<Self> {
        PlacedWidget {
            content: self,
            placement: AnchorPlacement::default(),
        }
    }
}

impl<T: AnchorPlacementEnabled> AnchorPlaced for T {
    type Placed = PlacedWidget<Self>;

    fn left(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        self.place().left(offset)
    }

    fn top(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        self.place().top(offset)
    }

    fn right(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        self.place().right(offset)
    }

    fn bottom(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        self.place().bottom(offset)
    }

    fn height(self, size: impl Into<Option<u16>>) -> Self::Placed {
        self.place().height(size)
    }

    fn width(self, size: impl Into<Option<u16>>) -> Self::Placed {
        self.place().width(size)
    }

    fn fill(self) -> Self::Placed {
        self.place().fill()
    }
}

/// Placement widget decorator
///
/// Constrains the placed_widgets's scope.
///
/// Wraps widgets that implement [AnchorPlacementEnabled].
#[derive(Default)]
pub struct PlacedWidget<W> {
    /// The inner widget being placed.
    pub content: W,
    /// The anchor placement constraints applied to the widget.
    pub placement: AnchorPlacement,
}
impl<W> PlacedWidget<W> {
    /// Creates a new placed widget wrapping the given content with default anchor placement.
    pub fn new(content: W) -> PlacedWidget<W> {
        Self {
            content,
            placement: AnchorPlacement::default(),
        }
    }
}
impl<M, W, A: AppEvent> Widget<M, A> for PlacedWidget<W>
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
        let scope = self.placement.place(painter.scope());
        let painter = painter.with_scope(scope);
        let child_scope = self.content.update_asserted(model, input, screen, &painter);
        let shift = point(
            child_scope.col.saturating_sub(scope.col),
            child_scope.row.saturating_sub(scope.row),
        );
        let shrink = point(
            scope
                .width
                .saturating_sub(child_scope.width)
                .saturating_add(shift.col),
            scope
                .height
                .saturating_sub(child_scope.height)
                .saturating_add(shift.row),
        );
        window(
            painter.scope().position() + shift,
            painter.scope().size() - shrink,
        )
    }
}
impl<W> AnchorPlaced for PlacedWidget<W> {
    type Placed = Self;

    fn left(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.left(offset);
        self
    }

    fn right(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.right(offset);
        self
    }

    fn top(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.top(offset);
        self
    }

    fn bottom(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.bottom(offset);
        self
    }

    fn width(mut self, size: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.width(size);
        self
    }
    fn height(mut self, size: impl Into<Option<u16>>) -> Self {
        self.placement = self.placement.height(size);
        self
    }

    fn fill(mut self) -> Self {
        self.placement = self.placement.fill();
        self
    }
}

/// Specifies how to place a widget in the space available
pub trait Placement {
    /// Where should the widget be placed in the given window?
    fn place(&self, window: Window) -> Window;
}

/// The default [`crate::widget::Placement`] that takes all available space (a.k.a. fill)
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Anchor;

impl Placement for Anchor {
    fn place(&self, window: Window) -> Window {
        window
    }
}

/// A unifying trait for all static anchor placement initialization
pub trait AnchorPlaced {
    /// The resulting placed widget type after applying placement constraints.
    type Placed;

    /// how far from the left?
    fn left(self, offset: impl Into<Option<u16>>) -> Self::Placed;

    /// how far from the top?
    fn top(self, offset: impl Into<Option<u16>>) -> Self::Placed;

    /// how far from the right?
    fn right(self, offset: impl Into<Option<u16>>) -> Self::Placed;

    /// how far from the botton?
    fn bottom(self, offset: impl Into<Option<u16>>) -> Self::Placed;

    /// how high?
    fn height(self, size: impl Into<Option<u16>>) -> Self::Placed;

    /// how wide?
    fn width(self, size: impl Into<Option<u16>>) -> Self::Placed;

    /// reset to fill the whole area
    fn fill(self) -> Self::Placed;
}

impl AnchorPlaced for Anchor {
    type Placed = AnchorPlacement;
    fn left(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().left(offset)
    }
    fn right(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().right(offset)
    }
    fn top(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().top(offset)
    }
    fn bottom(self, offset: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().bottom(offset)
    }
    fn width(self, size: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().width(size)
    }
    fn height(self, size: impl Into<Option<u16>>) -> Self::Placed {
        AnchorPlacement::default().height(size)
    }
    fn fill(self) -> Self::Placed {
        AnchorPlacement::default()
    }
}

/// The specific [`crate::widget::Placement`] specifying boundaries and size
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AnchorPlacement {
    /// how far from the left?
    pub left: Option<u16>,
    /// how far from the top?
    pub top: Option<u16>,
    /// how far from the right?
    pub right: Option<u16>,
    /// how far from the botton?
    pub bottom: Option<u16>,
    /// how high?
    pub width: Option<u16>,
    /// how wide?
    pub height: Option<u16>,
}

impl AnchorPlaced for AnchorPlacement {
    type Placed = Self;
    fn left(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.left = offset.into();
        self
    }
    fn right(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.right = offset.into();
        self
    }
    fn top(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.top = offset.into();
        self
    }
    fn bottom(mut self, offset: impl Into<Option<u16>>) -> Self {
        self.bottom = offset.into();
        self
    }
    fn width(mut self, size: impl Into<Option<u16>>) -> Self {
        self.width = size.into();
        self
    }
    fn height(mut self, size: impl Into<Option<u16>>) -> Self {
        self.height = size.into();
        self
    }
    fn fill(self) -> Self::Placed {
        AnchorPlacement::default()
    }
}

impl Placement for AnchorPlacement {
    /// Anchors the widget to sides
    /// if anchors are given
    ///
    /// If height or width are given
    /// without anchors or with both anchors
    /// it will center the widget
    fn place(&self, w: Window) -> Window {
        let s = w.size();

        let edges = |scope: u16, start: Option<u16>, size: Option<u16>, end: Option<u16>| match (
            start, size, end,
        ) {
            (None, None, None) => (0, scope),
            (None, None, Some(offset_end)) => (0, scope.saturating_sub(offset_end)),
            (Some(offset_start), None, None) => (
                offset_start.clamp(0, scope),
                scope.saturating_sub(offset_start),
            ),
            (None, Some(fixed_size), None) => (
                scope.saturating_sub(fixed_size) / 2,
                fixed_size.clamp(0, scope),
            ),
            (None, Some(fixed_size), Some(offset_end)) => (
                scope.saturating_sub(fixed_size).saturating_sub(offset_end),
                fixed_size.clamp(0, scope.saturating_sub(offset_end)),
            ),
            (Some(offset_start), Some(fixed_size), None) => (
                offset_start.clamp(0, scope),
                fixed_size.clamp(0, scope.saturating_sub(offset_start)),
            ),
            (Some(offset_start), None, Some(offset_end)) => (
                offset_start.clamp(0, scope),
                scope
                    .saturating_sub(offset_start)
                    .saturating_sub(offset_end),
            ),
            (Some(offset_start), Some(fixed_size), Some(offset_end)) => {
                let space = scope
                    .saturating_sub(offset_start)
                    .saturating_sub(offset_end);
                (
                    (offset_start.saturating_add(space.saturating_sub(fixed_size) / 2))
                        .clamp(0, scope),
                    fixed_size.clamp(0, space),
                )
            }
        };

        let x = edges(s.col, self.left, self.width, self.right);
        let y = edges(s.row, self.top, self.height, self.bottom);

        let placement = window(point(x.0, y.0), point(x.1, y.1));

        w.relative_crop(placement)
    }
}

#[test]
fn test_anchor_placement() {
    let sut = AnchorPlacement::default().top(0).height(1);
    let w = sut.place(window(point(0, 0), point(5, 5)));
    assert_eq!(w, window(point(0, 0), point(5, 1)));

    let sut = AnchorPlacement::default().height(1).bottom(0);
    let w = sut.place(window(point(0, 0), point(5, 5)));
    assert_eq!(w, window(point(0, 4), point(5, 1)));

    let sut = AnchorPlacement::default().top(1).bottom(1);
    let w = sut.place(window(point(2, 2), point(5, 5)));
    assert_eq!(w, window(point(2, 3), point(5, 3)));
}
