use gooey::widgets::Canvas;
use gooey::{Run, Tick};
use kludgine::figures::units::Px;
use kludgine::figures::{Angle, IntoSigned, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::text::{Text, TextOrigin};
use kludgine::Color;

fn main() -> gooey::Result<()> {
    let mut angle = Angle::degrees(0);
    Canvas::new(move |context| {
        angle += Angle::degrees(1);

        let center = Point::from(context.graphics.size()).into_signed() / 2;
        context.graphics.draw_text(
            Text::new("Canvas exposes the full power of Kludgine", Color::WHITE)
                .origin(TextOrigin::Center),
            center - Point::new(Px(0), Px(100)),
            None,
            None,
        );
        context.graphics.draw_shape(
            &Shape::filled_rect(
                Rect::new(Point::new(Px(-50), Px(-50)), Size::new(Px(100), Px(100))),
                Color::RED,
            ),
            center,
            Some(angle),
            None,
        )
    })
    .tick(Tick::redraws_per_second(60))
    .run()
}
