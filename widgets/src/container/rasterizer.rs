use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    renderer::Renderer,
    stylecs::Points,
    Transmogrifier,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::container::{Container, ContainerTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ContainerTransmogrifier {
    type Widget = Container;
}

impl<R: Renderer> WidgetRasterizer<R> for ContainerTransmogrifier {
    fn render(&self, rasterizer: &Rasterizer<R>, state: &Container, bounds: Rect<f32, Points>) {
        if let Some(child_transmogrifier) =
            rasterizer.ui.transmogrifier(state.child.widget_type_id())
        {
            let size = child_transmogrifier.content_size(
                state.child.as_ref(),
                rasterizer,
                Size2D::new(Some(bounds.size.width), Some(bounds.size.height)),
            );
            let remaining_size = (bounds.size.to_vector()
                - size.to_vector()
                - state.padding.minimum_size().to_vector())
            .to_size();

            // TODO respect alignment
            let child_rect = Rect::new(
                Point2D::new(remaining_size.width / 2., remaining_size.height / 2.),
                size,
            );

            child_transmogrifier.render(rasterizer, state.child.as_ref(), child_rect);
        }
    }

    fn content_size(
        &self,
        state: &Container,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        if let Some(child_transmogrifier) =
            rasterizer.ui.transmogrifier(state.child.widget_type_id())
        {
            let size =
                child_transmogrifier.content_size(state.child.as_ref(), rasterizer, constraints);
            (size.to_vector() + state.padding.minimum_size().to_vector()).to_size()
        } else {
            Size2D::default()
        }
    }
}
