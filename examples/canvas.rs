use gooey::widget::Widget;
use gooey::widgets::Canvas;
use kludgine::figures::units::Px;
use kludgine::figures::{Angle, IntoSigned, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::Color;

fn main() -> gooey::Result<()> {
    let mut angle = Angle::degrees(0);
    Canvas::new(move |graphics, _window| {
        angle += Angle::degrees(1);

        let center = Point::from(graphics.size()).into_signed() / 2;
        graphics.draw_text(
            "Canvas exposes the full power of Kludgine",
            Color::WHITE,
            kludgine::text::TextOrigin::Center,
            center - Point::new(Px(0), Px(100)),
            None,
            None,
            None,
        );
        graphics.draw_shape(
            &Shape::filled_rect(
                Rect::new(Point::new(Px(-50), Px(-50)), Size::new(Px(100), Px(100))),
                Color::RED,
            ),
            center,
            Some(angle),
            None,
        )
    })
    .target_fps(60)
    .run()
}
