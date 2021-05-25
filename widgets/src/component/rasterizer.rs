use gooey_core::{euclid::Size2D, renderer::Renderer, styles::Points};
use gooey_rasterizer::{AnyRasterContext, RasterContext, WidgetRasterizer};

use crate::component::{Behavior, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<R> for ComponentTransmogrifier<B> {
    fn render(&self, context: RasterContext<Self, R>) {
        context.rasterizer.ui.with_transmogrifier(
            context.widget.content.id(),
            context.rasterizer,
            |child_transmogrifier, child_state, child_widget| {
                let bounds = context
                    .rasterizer
                    .renderer()
                    .map(|r| r.bounds())
                    .unwrap_or_default();
                child_transmogrifier.render_within(
                    AnyRasterContext::new(
                        context.widget.content.clone(),
                        child_state,
                        context.rasterizer,
                        child_widget,
                    ),
                    bounds,
                );
            },
        );
    }

    fn content_size(
        &self,
        context: RasterContext<Self, R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .rasterizer
            .ui
            .with_transmogrifier(
                context.widget.content.id(),
                context.rasterizer,
                |child_transmogrifier, child_state, child_widget| {
                    child_transmogrifier.content_size(
                        AnyRasterContext::new(
                            context.widget.content.clone(),
                            child_state,
                            context.rasterizer,
                            child_widget,
                        ),
                        constraints,
                    )
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
