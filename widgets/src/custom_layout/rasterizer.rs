use gooey_core::{
    euclid::Size2D, renderer::Renderer, Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::custom_layout::{CustomLayout, CustomLayoutTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for CustomLayoutTransmogrifier {
    type State = ();
    type Widget = CustomLayout;
}

impl<R: Renderer> WidgetRasterizer<R> for CustomLayoutTransmogrifier {
    fn render(&self, context: TransmogrifierContext<Self, Rasterizer<R>>) {
        todo!()
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        todo!()
    }
}
