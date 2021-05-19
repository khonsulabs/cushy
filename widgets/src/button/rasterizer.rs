use std::ops::Deref;

use gooey_core::{
    euclid::{Length, Rect, Size2D, Vector2D},
    renderer::Renderer,
    styles::{ForegroundColor, Points, Srgba, Style},
    Transmogrifier,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::button::{Button, ButtonTransmogrifier};

const BUTTON_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ButtonTransmogrifier {
    type State = ();
    type Widget = Button;
}

impl<R: Renderer> WidgetRasterizer<R> for ButtonTransmogrifier {
    fn render(
        &self,
        _state: &Self::State,
        rasterizer: &Rasterizer<R>,
        button: &Button,
        bounds: Rect<f32, Points>,
    ) {
        if let Some(scene) = rasterizer.deref() {
            scene.fill_rect(
                &bounds,
                &Style::new().with(ForegroundColor(Srgba::new(0., 1., 0., 1.).into())),
            );

            let text_size = scene.measure_text(&button.label, &Style::default());

            let center = bounds.center();
            scene.render_text(
                &button.label,
                center - Vector2D::from_lengths(text_size.width, text_size.height()) / 2.
                    + Vector2D::from_lengths(Length::default(), text_size.ascent),
                &Style::default(),
            );
        }
    }

    fn content_size(
        &self,
        _state: &Self::State,
        button: &Button,
        rasterizer: &Rasterizer<R>,
        _constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        if let Some(scene) = rasterizer.deref() {
            // TODO should be wrapped width
            let text_size = scene.measure_text(&button.label, &Style::default());
            (Vector2D::from_lengths(text_size.width, text_size.height())
                + Vector2D::from_lengths(BUTTON_PADDING * 2., BUTTON_PADDING * 2.))
            .to_size()
        } else {
            Size2D::default()
        }
    }
}
