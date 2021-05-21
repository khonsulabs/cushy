use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    renderer::Renderer,
    styles::Points,
    Transmogrifier,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::container::{Container, ContainerTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ContainerTransmogrifier {
    type State = ();
    type Widget = Container;
}

impl<R: Renderer> WidgetRasterizer<Rasterizer<R>> for ContainerTransmogrifier {
    fn render(
        &self,
        _state: &Self::State,
        rasterizer: &Rasterizer<R>,
        container: &Container,
        bounds: Rect<f32, Points>,
    ) {
        rasterizer.ui.with_transmogrifier(
            container.child.id(),
            |child_transmogrifier, child_state, child_widget, _channels| {
                let size = child_transmogrifier.content_size(
                    child_state,
                    child_widget,
                    rasterizer,
                    Size2D::new(Some(bounds.size.width), Some(bounds.size.height)),
                );
                let remaining_size = (bounds.size.to_vector()
                    - size.to_vector()
                    - container.padding.minimum_size().to_vector())
                .to_size();

                // TODO respect alignment
                let child_rect = Rect::new(
                    Point2D::new(remaining_size.width / 2., remaining_size.height / 2.),
                    size,
                );

                child_transmogrifier.render(child_state, rasterizer, child_widget, child_rect);
            },
        );
    }

    fn content_size(
        &self,
        _state: &Self::State,
        container: &Container,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        rasterizer
            .ui
            .with_transmogrifier(
                container.child.id(),
                |child_transmogrifier, child_state, child_widget, _channels| {
                    let size = child_transmogrifier.content_size(
                        child_state,
                        child_widget,
                        rasterizer,
                        constraints,
                    );
                    (size.to_vector() + container.padding.minimum_size().to_vector()).to_size()
                },
            )
            .unwrap_or_default()
    }
}
