use gooey_core::{euclid::Size2D, renderer::Renderer, Points, TransmogrifierContext};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::component::{Behavior, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<R> for ComponentTransmogrifier<B> {
    fn render(&self, context: TransmogrifierContext<Self, Rasterizer<R>>) {
        context.frontend.with_transmogrifier(
            context.widget.content.id(),
            |child_transmogrifier, mut child_context| {
                let bounds = context
                    .frontend
                    .renderer()
                    .map(|r| r.bounds())
                    .unwrap_or_default();
                child_transmogrifier.render_within(&mut child_context, bounds, context.style);
            },
        );
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.content.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier.content_size(&mut child_context, constraints)
                },
            )
            .unwrap_or_default()
    }
}

impl<B: Behavior, R: Renderer> From<ComponentTransmogrifier<B>>
    for gooey_rasterizer::RegisteredTransmogrifier<R>
{
    fn from(transmogrifier: ComponentTransmogrifier<B>) -> Self {
        Self(std::boxed::Box::new(transmogrifier))
    }
}
