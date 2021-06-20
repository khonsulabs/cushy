use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    renderer::Renderer,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::container::{Container, ContainerTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ContainerTransmogrifier {
    type State = ();
    type Widget = Container;
}

impl<R: Renderer> WidgetRasterizer<R> for ContainerTransmogrifier {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        context.frontend.with_transmogrifier(
            context.widget.child.id(),
            |child_transmogrifier, mut child_context| {
                let render_size = context
                    .frontend
                    .renderer()
                    .map(|r| r.size())
                    .unwrap_or_default();
                let size = child_transmogrifier.content_size(
                    &mut child_context,
                    Size2D::new(Some(render_size.width), Some(render_size.height)),
                );
                let remaining_size = (render_size.to_vector()
                    - size.to_vector()
                    - context.widget.padding.minimum_size().to_vector())
                .to_size();

                // TODO respect alignment
                let child_rect = Rect::new(
                    Point2D::new(remaining_size.width / 2., remaining_size.height / 2.),
                    size,
                );

                child_transmogrifier.render_within(&mut child_context, child_rect, context.style);
            },
        );
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.child.id(),
                |child_transmogrifier, mut child_context| {
                    let size = child_transmogrifier.content_size(&mut child_context, constraints);
                    (size.to_vector() + context.widget.padding.minimum_size().to_vector()).to_size()
                },
            )
            .unwrap_or_default()
    }
}
