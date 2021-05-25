use gooey_core::{euclid::Size2D, renderer::Renderer, styles::Points};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::component::{Behavior, Component, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<R> for ComponentTransmogrifier<B> {
    fn render(&self, _state: &Self::State, rasterizer: &Rasterizer<R>, container: &Component<B>) {
        rasterizer.ui.with_transmogrifier(
            container.content.id(),
            rasterizer,
            |child_transmogrifier, child_state, child_widget| {
                let bounds = rasterizer
                    .renderer()
                    .map(|r| r.bounds())
                    .unwrap_or_default();
                child_transmogrifier.render_within(child_state, rasterizer, child_widget, bounds);
            },
        );
    }

    fn content_size(
        &self,
        _state: &Self::State,
        container: &Component<B>,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        rasterizer
            .ui
            .with_transmogrifier(
                container.content.id(),
                rasterizer,
                |child_transmogrifier, child_state, child_widget| {
                    child_transmogrifier.content_size(
                        child_state,
                        child_widget,
                        rasterizer,
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
