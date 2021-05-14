use crate::{Kludgine, KludgineRenderer};
use gooey_core::Materializer;
use gooey_widgets::button::{Button, ButtonMaterializer};
use kludgine::prelude::*;

impl Materializer<Kludgine> for ButtonMaterializer {
    type Widget = Button;
}

impl KludgineRenderer for ButtonMaterializer {
    fn render(&self, scene: &Target, state: &Button, bounds: Rect<f32, Scaled>) {
        Shape::rect(bounds)
            .fill(Fill::new(Color::GREEN))
            .render_at(Point::default(), scene);

        let scale = scene.scale_factor();
        let text = Text::span(
            &state.label,
            Style::new().with(ForegroundColor(Color::BLACK.into())),
        )
        .wrap(
            scene,
            TextWrap::SingleLine {
                max_width: bounds.size.width(),
                truncate: true,
                alignment: Alignment::Center,
            },
        )
        .unwrap();
        let size = text.size() / scale;
        text.render(
            scene,
            Point::new(0., bounds.center().y - size.to_vector().y / 2.),
            true,
        )
        .unwrap();
    }
}
