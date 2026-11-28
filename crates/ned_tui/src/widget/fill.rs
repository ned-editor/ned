use crate::widget::{AnchorPlacementEnabled, AppEvent, Event, Painter, Screen, Widget, Window};

/// A widget to fill/wipe all available space (with painter background) before drawing the content.
///
/// The widget wraps the inner content widget.
///
/// ```rust
/// # use termit::prelude::*;
/// let content = " text "; // or any other Widget
/// let ui = Fill::new(content);
/// ```
///
#[derive(Debug, Default)]
pub struct Fill<W> {
    content: W,
}

impl<W> Fill<W> {
    /// Creates a new fill widget wrapping the given widget.
    pub fn new(widget: W) -> Self {
        Fill { content: widget }
    }
    /// Returns an immutable reference to the inner content widget.
    pub fn content(&self) -> &W {
        &self.content
    }
    /// Returns a mutable reference to the inner content widget.
    pub fn content_mut(&mut self) -> &mut W {
        &mut self.content
    }
}
impl<W> AnchorPlacementEnabled for Fill<W> {}
impl<W, M, A: AppEvent> Widget<M, A> for Fill<W>
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
        if matches!(input, Event::Refresh(_)) {
            painter.fill(screen);
        }

        self.content.update_asserted(model, input, screen, painter);

        painter.scope()
    }
}
