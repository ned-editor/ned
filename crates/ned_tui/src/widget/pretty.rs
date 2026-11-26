use std::ops::DerefMut;

use crate::prelude::*;

impl<W> AnchorPlacementEnabled for Pretty<W> {}
impl<W, M, A: AppEvent> Widget<M, A> for Pretty<W>
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
        let painter = painter.apply(self.style());
        self.deref_mut()
            .update_asserted(model, input, screen, &painter)
    }
}
