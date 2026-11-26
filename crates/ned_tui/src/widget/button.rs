use super::Widget;
use crate::{
    Painter, Screen,
    geometry::{Window, point},
    prelude::*,
};

/// The action a [`Button`] runs, given its own content and the model.
pub type OnClick<W, M> = Box<dyn Fn(&mut W, &mut M)>;

/// A button that can be clicked to trigger an action
pub struct Button<W, M> {
    /// The inner content widget rendered inside the button.
    pub content: W,
    /// The click handler callback.
    pub on_click: OnClick<W, M>,
}

impl<W, M> Button<W, M> {
    /// Create a button with the given text
    pub fn new(content: W) -> Self {
        Self {
            content,
            on_click: Box::new(|_, _| {}),
        }
    }
    /// Adds an action to be executed when the button is clicked.
    pub fn on_click(mut self, action: impl Fn(&mut W, &mut M) + 'static) -> Self
    where
        Self: 'static,
    {
        self.on_click = Box::new(move |w, m| {
            (self.on_click)(w, m);
            action(w, m);
        });
        self
    }
}
impl<W, M> AnchorPlacementEnabled for Button<W, M> {}
impl<W, M, A: AppEvent> Widget<M, A> for Button<W, M>
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
        let scope = self.content.update_asserted(model, input, screen, painter);
        if let Event::Mouse(mouse) = input {
            match mouse.kind {
                MouseEventKind::Up(_) if point(mouse.column, mouse.row).is_in(scope) => {
                    (self.on_click)(&mut self.content, model);
                }
                _ => {}
            }
        }
        scope
    }
}
