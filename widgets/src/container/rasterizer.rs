use gooey_core::{
    figures::{Point, Rect, Size},
    Scaled, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{ContentArea, Rasterizer, Renderer, WidgetRasterizer};

use crate::container::{Container, ContainerTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ContainerTransmogrifier {
    type State = ();
    type Widget = Container;
}

impl<R: Renderer> WidgetRasterizer<R> for ContainerTransmogrifier {
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        context.frontend.with_transmogrifier(
            context.widget.child.id(),
            |child_transmogrifier, mut child_context| {
                let child_content_area = child_transmogrifier
                    .content_size(
                        &mut child_context,
                        Size::new(
                            Some(content_area.size.content.width),
                            Some(content_area.size.content.height),
                        ),
                    )
                    .total_size();
                let remaining_size = content_area.size.content - child_content_area;

                // TODO respect Alignment + Vertical alignment
                let child_rect = Rect::sized(
                    Point::new(remaining_size.width / 2., remaining_size.height / 2.),
                    child_content_area,
                );
                child_transmogrifier.render_within(
                    &mut child_context,
                    child_rect,
                    Some(context.registration.id()),
                    context.style,
                );
            },
        );
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.child.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier
                        .content_size(&mut child_context, constraints)
                        .content
                },
            )
            .unwrap_or_default()
    }
}
