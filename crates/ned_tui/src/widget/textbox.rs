use super::Widget;
use crate::{
    geometry::{point, Window},
    prelude::*,
    Painter, Screen,
};
use std::marker::PhantomData;

/// A simplistic single line String editor.
///
/// It is suitable for simple use cases such as quick search input.
///
/// The cursor is always at the end. Backspace removes the last char.
///
/// Clicking on the TextBox activates it,
/// clicking somewhere else deactivates it.
/// You can override this by setting the
/// `active` property on each update.
///
/// No cloning/moving, it will edit the `&mut String` in the model directly.
///
/// The last part of the string that fits is visible (tail).
pub struct TextBox<F, M, A: AppEvent> {
    accessor: F,
    phantom: PhantomData<M>,
    phantom_app_event: PhantomData<A>,
    /// Whether the text box is currently focused and active for input.
    pub active: bool,
}

impl<F, M, A: AppEvent> TextBox<F, M, A>
where
    F: Fn(&mut M) -> Option<&mut String>,
{
    /// Create a textbox
    ///
    /// The `accessor` specifies how to get to a mutable String reference in the model
    pub fn new(accessor: F) -> Self {
        Self {
            accessor,
            active: false,
            phantom: PhantomData,
            phantom_app_event: PhantomData,
        }
    }
}
impl<F, M, A: AppEvent> AnchorPlacementEnabled for TextBox<F, M, A> {}
impl<F, M, A: AppEvent> Widget<M, A> for TextBox<F, M, A>
where
    F: Fn(&mut M) -> Option<&mut String>,
{
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        match input {
            Event::Key(key) if self.active => match key.keycode {
                KeyCode::Backspace => {
                    if let Some(ref mut text) = (self.accessor)(model) {
                        text.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut text) = (self.accessor)(model) {
                        text.push(c);
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => {
                if let MouseEventKind::Up(_) = mouse.kind {
                    self.active = point(mouse.column, mouse.row).is_in(painter.scope())
                }
            }
            Event::Refresh(_) => {
                let text = (self.accessor)(model)
                    .map(|s| s.as_str())
                    .unwrap_or_default();
                let split = text
                    .char_indices()
                    .rev()
                    .nth(painter.scope().width.saturating_sub(1) as usize);

                let visible = match split {
                    Some((pos, _)) => &text[pos..],
                    None => text,
                };

                painter.paint(visible, screen, 0, false);
            }
            _ => {}
        }

        painter.scope()
    }
}
