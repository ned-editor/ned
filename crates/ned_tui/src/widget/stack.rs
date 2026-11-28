use std::ops::DerefMut;

use crate::{
    geometry::{point, window},
    widget::{AnchorPlacementEnabled, AppEvent, Event, Painter, Screen, Widget, Window},
};

/// A stack layout widget
///
/// Layered stack: Widgets are placed on top of each other.
///
/// Flat stack: Widgets are added until they fill up
/// a horizontal (`east_south`) or vertical (`south_east`) line.
/// Then - if there is space - another line is started.
///
/// The plain Stack is for single widget type.
/// Convenience `*_box*` methods are added for `Box`ed widgets.
#[derive(Debug, Default)]
pub struct Stack<S> {
    layout: StackLayout,
    widgets: S,
}

impl<S> Stack<S> {
    /// Creates a horizontal stack layout placing widgets from west to east, wrapping south.
    pub fn east_south(widgets: S) -> Self {
        Self {
            layout: StackLayout::EastSouth,
            widgets,
        }
    }
    /// Creates a vertical stack layout placing widgets from north to south, wrapping east.
    pub fn south_east(widgets: S) -> Self {
        Self {
            layout: StackLayout::SouthEast,
            widgets,
        }
    }
    /// Creates a layered stack layout overlaying widgets on top of each other.
    pub fn layered(widgets: S) -> Self {
        Self {
            layout: StackLayout::Layered,
            widgets,
        }
    }
    /// Returns an immutable reference to the collection of widgets.
    pub fn content(&self) -> &S {
        &self.widgets
    }
    /// Returns a mutable reference to the collection of widgets.
    pub fn content_mut(&mut self) -> &mut S {
        &mut self.widgets
    }
}

impl<S> Stack<Vec<S>> {
    // Builder-style accumulation, unrelated to `std::ops::Add`.
    /// Adds a widget to the stack.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, widget: S) -> Self {
        self.widgets.push(widget);
        self
    }
}

impl<'a, M, A: AppEvent> Stack<Vec<Box<dyn Widget<M, A> + 'a>>> {
    /// Creates an east-south stack layout from boxed widgets.
    #[must_use]
    pub fn east_south_boxes(widgets: Vec<Box<dyn Widget<M, A> + 'a>>) -> Self {
        Self::east_south(widgets)
    }
    /// Creates a south-east stack layout from boxed widgets.
    #[must_use]
    pub fn south_east_boxes(widgets: Vec<Box<dyn Widget<M, A> + 'a>>) -> Self {
        Self::south_east(widgets)
    }
    /// Creates a layered stack layout from boxed widgets.
    #[must_use]
    pub fn layered_boxes(widgets: Vec<Box<dyn Widget<M, A> + 'a>>) -> Self {
        Self::layered(widgets)
    }
    /// Adds a boxed widget to the stack.
    pub fn add_box(mut self, widget: impl Widget<M, A> + 'a) -> Self {
        self.widgets.push(Box::new(widget));
        self
    }
}
impl<S> AnchorPlacementEnabled for Stack<S> {}
impl<M, S, W, A: AppEvent> Widget<M, A> for Stack<S>
where
    W: Widget<M, A>,
    S: DerefMut<Target = [W]>,
{
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        let pscope = painter.scope();
        let mut position = pscope.position();
        let mut bulk = 0;
        let mut size = point(0, 0);
        for widget in self.widgets.deref_mut().iter_mut() {
            let scope = pscope & window(position, pscope.size());
            let scope = widget.update_asserted(model, input, screen, &painter.with_scope(scope));
            let botom_right = scope.bottom_right();
            size = point(
                u16::max(size.col, botom_right.col - pscope.col),
                u16::max(size.row, botom_right.row - pscope.row),
            );

            match self.layout {
                StackLayout::Layered => {}
                StackLayout::EastSouth => {
                    bulk = u16::max(bulk, scope.height);
                    position = point(scope.col.saturating_add(scope.width), position.row);
                    if position.col >= pscope.col.saturating_add(pscope.width) {
                        // reached the end, make a new row
                        position = point(pscope.col, position.row.saturating_add(bulk));
                        bulk = 0;
                    }
                }
                StackLayout::SouthEast => {
                    bulk = u16::max(bulk, scope.width);
                    position = point(position.col, scope.row.saturating_add(scope.height));
                    if position.row >= pscope.row.saturating_add(pscope.height) {
                        // reached the end, make a new column
                        position = point(position.col.saturating_add(bulk), pscope.row);
                        bulk = 0;
                    }
                }
            }
        }

        window(pscope.position(), size)
        //painter.scope()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
enum StackLayout {
    #[default]
    Layered,
    EastSouth,
    SouthEast,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Widget;
    use crate::widget::Fill;
    use std::marker::PhantomData;

    /// just takes all space
    #[derive(Default, Debug, Clone, Copy)]
    struct MockRogueSpacer<M> {
        phantom: PhantomData<M>,
    }
    impl<M, A: AppEvent> Widget<M, A> for MockRogueSpacer<M> {
        fn update(
            &mut self,
            _model: &mut M,
            _input: &crate::input::events::Event<A>,
            _screen: &mut crate::Screen,
            _painter: &crate::Painter,
        ) -> crate::prelude::Window {
            // badly behaved widget does not respect scope
            Window {
                col: 1,
                row: 1,
                width: u16::MAX,
                height: u16::MAX,
            }
        }

        fn update_asserted(
            &mut self,
            model: &mut M,
            input: &Event<A>,
            screen: &mut Screen,
            painter: &Painter,
        ) -> Window {
            //skip checks
            self.update(model, input, screen, painter)
        }
    }

    #[test]
    fn test_oversize() {
        let stacks = vec![
            Stack::east_south(vec![
                MockRogueSpacer::default(),
                MockRogueSpacer::default(),
                MockRogueSpacer::default(),
            ]),
            Stack::south_east(vec![
                MockRogueSpacer::default(),
                MockRogueSpacer::default(),
                MockRogueSpacer::default(),
            ]),
        ];

        let painter = Painter::default();
        let painter = painter.with_scope(window(point(65530, 65530), point(65530, 65530)));

        let mut screen = Screen::default();
        let event = Event::<()>::Refresh(1);

        for mut s in stacks {
            s.update(&mut (), &event, &mut screen, &painter);
        }
    }
    #[test]
    fn test_reported_scope() {
        let stacks = vec![
            Stack::<Vec<Fill<()>>>::east_south(vec![
                Fill::default(),
                Fill::default(),
                Fill::default(),
            ]),
            Stack::south_east(vec![Fill::default(), Fill::default(), Fill::default()]),
        ];

        let painter = Painter::default();
        let painter = painter.with_scope(window(point(5, 3), point(4, 2)));

        let mut screen = Screen::default();
        let event = Event::<()>::Refresh(1);

        for mut s in stacks {
            s.update_asserted(&mut (), &event, &mut screen, &painter);
        }
    }
}
