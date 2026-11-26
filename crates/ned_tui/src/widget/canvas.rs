use crate::prelude::*;

/// A styling widget
///
/// Apply a default style to the given scope, filling the area background first.
///
/// The widget wraps the inner widget.
///
/// It is different from StyledContent and plan String/&str
/// - it claims the whole available space
/// - it fills the space first with background color
///
///
/// ```rust
/// # use termit::prelude::*;
/// let content = " text "; // or any other Widget
/// let ui = Canvas::new(content).back(Color::red(true));
/// ```
#[derive(Debug)]
pub struct Canvas<W> {
    content: Pretty<Fill<W>>,
}
impl Default for Canvas<()> {
    fn default() -> Self {
        Self {
            content: Default::default(),
        }
    }
}
impl<W> Canvas<W> {
    /// Creates a new canvas wrapping the given widget.
    pub fn new(widget: W) -> Self {
        Canvas {
            content: Pretty::new(Fill::new(widget)),
        }
    }
    /// Replaces the contained widget with new content while preserving canvas styling.
    pub fn contain<W2>(self, content: W2) -> Canvas<W2> {
        let (_, style) = self.content.into_inner();
        Canvas::new(content).apply(style)
    }
    /// Returns an immutable reference to the inner content widget.
    pub fn content(&self) -> &W {
        self.content.content()
    }
    /// Returns a mutable reference to the inner content widget.
    pub fn content_mut(&mut self) -> &mut W {
        self.content.content_mut()
    }
}

impl<W> Styled for Canvas<W> {
    type Cute = Self;

    fn pretty(self) -> Self::Cute {
        self
    }
}

impl<W> Stylish for Canvas<W> {
    fn style(&self) -> &Style {
        self.content.style()
    }

    fn style_mut(&mut self) -> &mut Style {
        self.content.style_mut()
    }
}
impl<W> AnchorPlacementEnabled for Canvas<W> {}
impl<W, M, A: AppEvent> Widget<M, A> for Canvas<W>
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
        self.content.update_asserted(model, input, screen, painter)
    }
}
