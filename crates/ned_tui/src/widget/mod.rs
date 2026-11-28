//! Components for building UIs

mod button;
mod canvas;
mod fill;
mod grid;
mod placement;
mod pretty;
mod stack;
mod text;
mod textbox;

use std::borrow::Cow;
use std::marker::PhantomData;

pub use self::button::*;
pub use self::canvas::*;
pub use self::fill::*;
pub use self::grid::*;
pub use self::placement::*;
pub use self::stack::*;
pub use self::text::*;
pub use self::textbox::*;
use crate::prelude::*;

/// A widget, the main trait really
///
/// In one call:
/// - process the input
/// - update the widget
/// - update the model
/// - paint the UI to the screen
pub trait Widget<M, A: AppEvent> {
    /// Do the widget magic.
    /// The returned scope must be contained in the painter AND screen scopes
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window;

    /// calls update and checks returned scope to conform to requirements in debug builds
    fn update_asserted(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        let Window {
            col,
            row,
            width,
            height,
        } = self.update(model, input, screen, painter);
        // requirements for all widgets
        {
            let expected = painter.scope();
            debug_assert!(
                col >= expected.col,
                "Widget left ({col}) is before the expected scope {expected:?} (widget {col}x{row}x{width}x{height})"
            );
            debug_assert!(
                row >= expected.row,
                "Widget top ({row}) is above the expected scope {expected:?} (widget {col}x{row}x{width}x{height})"
            );
            debug_assert!(
                col.saturating_add(width) <= expected.col.saturating_add(expected.width),
                "Widget right ({col}+{width}) does not fit the expected scope {expected:?} (widget {col}x{row}x{width}x{height})"
            );
            debug_assert!(
                row.saturating_add(height) <= expected.row.saturating_add(expected.height),
                "Widget bottom ({row}+{height}) does not fit the expected scope {expected:?} (widget {col}x{row}x{width}x{height})"
            );
        }
        Window {
            col,
            row,
            width,
            height,
        }
    }
}

/// Dummy widget that does nothing at all
impl<M, A: AppEvent> Widget<M, A> for () {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        _screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        painter.scope().trim(point(0, 0))
    }
}
impl AnchorPlacementEnabled for String {}
/// Owned String displaying widget
impl<M, A: AppEvent> Widget<M, A> for String {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        painter.paint(self, screen, 0, false)
    }
}
impl AnchorPlacementEnabled for &str {}
/// String reference displaying widget
impl<M, A: AppEvent> Widget<M, A> for &str {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        painter.paint(self, screen, 0, false)
    }
}
impl AnchorPlacementEnabled for Cow<'_, str> {}
/// String reference displaying widget
impl<M, A: AppEvent> Widget<M, A> for Cow<'_, str> {
    fn update(
        &mut self,
        _model: &mut M,
        _input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        painter.paint(self, screen, 0, false)
    }
}

impl<M, A: AppEvent> AnchorPlacementEnabled for Box<dyn Widget<M, A> + '_> {}
/// Widget for boxed widgets
impl<M, A: AppEvent> Widget<M, A> for Box<dyn Widget<M, A> + '_> {
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        self.as_mut().update(model, input, screen, painter)
    }
}

impl<W> AnchorPlacementEnabled for &mut W {}
/// Widget for mutable reference of widgets
impl<'a, M, W, A: AppEvent> Widget<M, A> for &'a mut W
where
    W: Widget<M, A> + 'a,
{
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        Widget::update(*self, model, input, screen, painter)
    }
}

/// Extensions for all widgets
pub trait WidgetExt<M, A: AppEvent> {
    /// Decorate the given widget with a loader and saver.
    ///
    /// This turns any generic widget into a model aware widget.
    /// You can interact with widget and model properties or
    /// replace the widget/state altogether. It is a convenience
    /// over coding your own decorator, but if the logic gets too
    /// complex, that's what you should probably do.
    fn bind_with(
        self,
        loader: impl Fn(&mut Self, &M) + 'static,
        saver: impl Fn(&Self, &mut M) + 'static,
    ) -> WidgetBinder<Self, M, A>
    where
        Self: Sized;

    /// A variation on `bind_with`, replacing the widget using a loader function.
    fn replace_with(self, loader: impl Fn(Self, &M) -> Self + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Default,
        Self: Sized;
    /// A variation on `bind_with`, only a loader
    fn load_with(self, loader: impl Fn(&mut Self, &M) + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Sized;
    /// A variation on `bind_with`, only a saver
    fn save_with(self, saver: impl Fn(&Self, &mut M) + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Sized;
    /// A variation on `bind_with`, bind specific model and widget properties
    fn bind_property<FW, FM, D>(self, widget: FW, model: FM) -> WidgetPropertyBinder<Self, M, D, A>
    where
        Self: Sized,
        FW: Fn(&mut Self) -> &mut D + 'static,
        FM: Fn(&mut M) -> &mut D + 'static,
        FW: 'static,
        FM: 'static,
        D: Default;
}

impl<T, M, A: AppEvent> WidgetExt<M, A> for T
where
    T: Widget<M, A>,
{
    fn bind_with(
        self,
        loader: impl Fn(&mut Self, &M) + 'static,
        saver: impl Fn(&Self, &mut M) + 'static,
    ) -> WidgetBinder<Self, M, A>
    where
        Self: Sized,
    {
        WidgetBinder {
            bound_widget: self,
            saver: Some(Box::new(saver) as Box<dyn Fn(&Self, &mut M)>),
            loader: Some(Box::new(loader) as Box<dyn Fn(&mut Self, &M)>),
            phantom_app_event: PhantomData,
        }
    }

    fn replace_with(self, loader: impl Fn(Self, &M) -> Self + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Sized,
        Self: Default,
    {
        self.load_with(move |w, m| {
            let what = std::mem::take(w);
            *w = loader(what, m);
        })
    }
    fn load_with(self, loader: impl Fn(&mut Self, &M) + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Sized,
    {
        WidgetBinder {
            bound_widget: self,
            saver: None,
            loader: Some(Box::new(loader) as Box<dyn Fn(&mut Self, &M)>),
            phantom_app_event: PhantomData,
        }
    }

    fn save_with(self, saver: impl Fn(&Self, &mut M) + 'static) -> WidgetBinder<Self, M, A>
    where
        Self: Sized,
    {
        WidgetBinder {
            bound_widget: self,
            saver: Some(Box::new(saver) as Box<dyn Fn(&Self, &mut M)>),
            loader: None,
            phantom_app_event: PhantomData,
        }
    }

    fn bind_property<FW, FM, D>(
        self,
        widget_property: FW,
        model_property: FM,
    ) -> WidgetPropertyBinder<Self, M, D, A>
    where
        Self: Sized,
        FW: Fn(&mut Self) -> &mut D,
        FM: Fn(&mut M) -> &mut D,
        FW: 'static,
        FM: 'static,
        D: Default,
    {
        WidgetPropertyBinder {
            bound_widget: self,
            widget_property: Box::new(widget_property),
            model_property: Box::new(model_property),
            phantom_app_event: PhantomData,
        }
    }
}

/// Property binding widget decorator.
///
/// Use it with the [`crate::widget::WidgetExt::bind_property()`] extension.
pub struct WidgetPropertyBinder<W, M, D, A: AppEvent> {
    /// The inner widget bound to model properties.
    pub bound_widget: W,
    /// Closure that binds widget state to model data.
    widget_property: Box<dyn Fn(&mut W) -> &mut D>,
    /// Closure that binds model data to widget state.
    model_property: Box<dyn Fn(&mut M) -> &mut D>,
    /// Phantom type parameter for the app event type.
    phantom_app_event: PhantomData<A>,
}
impl<W, M, D, A: AppEvent> AnchorPlacementEnabled for WidgetPropertyBinder<W, M, D, A> {}
impl<W, M, D, A: AppEvent> Widget<M, A> for WidgetPropertyBinder<W, M, D, A>
where
    W: Widget<M, A>,
    D: Default,
{
    fn update(
        &mut self,
        model: &mut M,
        input: &Event<A>,
        screen: &mut Screen,
        painter: &Painter,
    ) -> Window {
        *(self.widget_property)(&mut self.bound_widget) =
            std::mem::take((self.model_property)(model));

        let scope = self
            .bound_widget
            .update_asserted(model, input, screen, painter);

        *(self.model_property)(model) =
            std::mem::take((self.widget_property)(&mut self.bound_widget));

        scope
    }
}

/// Writes the widget's state back into the model.
pub type Saver<W, M> = Box<dyn Fn(&W, &mut M)>;
/// Reads the widget's state out of the model.
pub type Loader<W, M> = Box<dyn Fn(&mut W, &M)>;

/// Property binding widget decorator.
///
/// Use it with the
/// [`crate::widget::WidgetExt::bind_with()`],
/// [`crate::widget::WidgetExt::load_with()`],
/// [`crate::widget::WidgetExt::save_with()`]
/// extensions.
#[derive(Default)]
pub struct WidgetBinder<W, M, A: AppEvent> {
    /// The inner widget bound to model loader/saver functions.
    pub bound_widget: W,
    /// Saves widget state back into the model.
    saver: Option<Saver<W, M>>,
    /// Loads widget state from the model.
    loader: Option<Loader<W, M>>,
    /// Phantom type parameter for the app event type.
    phantom_app_event: PhantomData<A>,
}

impl<W, M, A: AppEvent> AnchorPlacementEnabled for WidgetBinder<W, M, A> {}
impl<W, M, A: AppEvent> Widget<M, A> for WidgetBinder<W, M, A>
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
        if let Some(loader) = self.loader.as_ref() {
            loader(&mut self.bound_widget, model);
        }

        let scope = self
            .bound_widget
            .update_asserted(model, input, screen, painter);

        if let Some(saver) = self.saver.as_ref() {
            saver(&self.bound_widget, model);
        }

        scope
    }
}
