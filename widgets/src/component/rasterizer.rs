use gooey_core::{
    euclid::{Rect, Size2D},
    renderer::Renderer,
    styles::Points,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::component::{Behavior, Component, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<Rasterizer<R>> for ComponentTransmogrifier<B> {
    fn render(
        &self,
        _state: &Self::State,
        rasterizer: &Rasterizer<R>,
        container: &Component<B>,
        bounds: Rect<f32, Points>,
    ) {
        rasterizer.ui.with_transmogrifier(
            container.content.id(),
            rasterizer,
            |child_transmogrifier, child_state, child_widget| {
                child_transmogrifier.render(child_state, rasterizer, child_widget, bounds);
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
