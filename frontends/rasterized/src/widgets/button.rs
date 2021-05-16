use gooey_core::{
    euclid::{Length, Rect, Vector2D},
    renderer::TextOptions,
    stylecs::{palette::Srgba, Points},
    Renderer, Transmogrifier,
};
use gooey_widgets::button::{Button, ButtonTransmogrifier};

use crate::{Rasterized, WidgetRasterizer};

const BUTTON_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterized<R>> for ButtonTransmogrifier {
    type Widget = Button;
    type Context = R;

    fn content_size(
        &self,
        state: &Button,
        _constraints: gooey_core::euclid::Size2D<Option<f32>, gooey_core::stylecs::Points>,
        context: &Self::Context,
    ) -> gooey_core::euclid::Size2D<f32, gooey_core::stylecs::Points> {
        // TODO should be wrapped width
        let text_size = context.measure_text(&state.label, &TextOptions::default());
        (Vector2D::from_lengths(text_size.width, text_size.height())
            + Vector2D::from_lengths(BUTTON_PADDING * 2., BUTTON_PADDING * 2.))
        .to_size()
    }
}

impl<R: Renderer> WidgetRasterizer<R> for ButtonTransmogrifier {
    fn render(&self, scene: &R, state: &Button, bounds: Rect<f32, Points>) {
        scene.fill_rect(&bounds, Srgba::new(0., 1., 0., 1.));

        let text_size = scene.measure_text(&state.label, &TextOptions::default());

        let center = bounds.center();
        scene.render_text(
            &state.label,
            center - Vector2D::from_lengths(text_size.width, text_size.height()) / 2.,
            &TextOptions::default(),
        );
    }
}
