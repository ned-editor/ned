use crate::prelude::*;
use std::borrow::Cow;

/// Allows for rich text flow composed of multiple parts formatted independently
#[derive(Default, Debug, Clone)]
pub struct Text<'a> {
    parts: Vec<Pretty<Cow<'a, str>>>,
}

impl<'a> Text<'a> {
    /// Creates a new rich text widget from styled string parts.
    pub fn new(parts: Vec<StylishStringy<'a>>) -> Self {
        Self { parts }
    }
    // Builder-style accumulation, unrelated to `std::ops::Add`.
    /// Appends a styled text part to the widget.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, part: impl Into<StylishStringy<'a>>) -> Self {
        self.parts.push(part.into());
        self
    }
}
impl<'a> AnchorPlacementEnabled for Text<'a> {}
impl<'a, M, A: AppEvent> Widget<M, A> for Text<'a> {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        screen: &mut crate::Screen,
        painter: &crate::Painter,
    ) -> crate::prelude::Window {
        let pscope = painter.scope();
        let mut scope = pscope;
        let mut offset = 0;
        let mut my_width = 0;
        let mut my_height = 0;

        for part in &mut self.parts {
            let scope_taken = painter.with_scope(scope).apply(*part.style()).paint(
                part.as_ref(),
                screen,
                offset,
                false,
            );
            my_width = u16::max(
                my_width,
                scope_taken
                    .width
                    .saturating_add(scope_taken.col)
                    .saturating_sub(pscope.col),
            );
            my_height = u16::max(
                my_height,
                scope_taken
                    .height
                    .saturating_add(scope_taken.row)
                    .saturating_sub(pscope.row),
            );
            scope = scope.relative_crop(window(
                point(
                    0,
                    scope_taken
                        .height
                        .saturating_add(scope_taken.row)
                        .saturating_sub(pscope.row)
                        .saturating_sub(1),
                ),
                scope.size(),
            ));
            offset = screen
                .last_write()
                // out of screen
                .unwrap_or_else(|| scope.position())
                .col
                .saturating_add(1);
        }

        window(pscope.position(), point(my_width, my_height))
    }
}
