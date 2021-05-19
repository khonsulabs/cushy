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

impl<R: Renderer> WidgetRasterizer<R> for ContainerTransmogrifier {
    fn render(
        &self,
        _state: &Self::State,
        rasterizer: &Rasterizer<R>,
        container: &Container,
        bounds: Rect<f32, Points>,
    ) {
        rasterizer.ui.with_transmogrifier(
            container.child.as_ref(),
            |child_transmogrifier, child_state| {
                let size = child_transmogrifier.content_size(
                    child_state,
                    container.child.as_ref(),
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

                child_transmogrifier.render(
                    child_state,
                    rasterizer,
                    container.child.as_ref(),
                    child_rect,
                );
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
                container.child.as_ref(),
                |child_transmogrifier, child_state| {
                    let size = child_transmogrifier.content_size(
                        child_state,
                        container.child.as_ref(),
                        rasterizer,
                        constraints,
                    );
                    (size.to_vector() + container.padding.minimum_size().to_vector()).to_size()
                },
            )
            .unwrap_or_default()
    }
}
