use crate::{Kludgine, KludgineRenderer};
use gooey_core::Materializer;
use gooey_widgets::button::{Button, ButtonMaterializer};
use kludgine::prelude::*;

impl Materializer<Kludgine> for ButtonMaterializer {
    type Widget = Button;
}

#[async_trait]
impl KludgineRenderer for ButtonMaterializer {
    async fn render(&self, scene: &Target, state: &Button, bounds: Rect<f32, Scaled>) {
        Shape::rect(bounds)
            .fill(Fill::new(Color::GREEN))
            .render_at(Point::default(), scene)
            .await;

        let scale = scene.scale_factor().await;
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
        .await
        .unwrap();
        let size = text.size().await / scale;
        text.render(
            scene,
            Point::new(0., bounds.center().y - size.to_vector().y / 2.),
            true,
        )
        .await
        .unwrap();
    }
}
