use gooey::widgets::Canvas;
use gooey::{Run, Tick};
use kludgine::figures::units::Px;
use kludgine::figures::{Angle, IntoSigned, Point, Rect, Size};
use kludgine::shapes::Shape;
use kludgine::text::{Text, TextOrigin};
use kludgine::{Color, DrawableExt};

fn main() -> gooey::Result<()> {
    let mut angle = Angle::degrees(0);
    Canvas::new(move |context| {
        angle += Angle::degrees(1);

        let center = Point::from(context.gfx.size()).into_signed() / 2;
        context.gfx.draw_text(
            Text::new("Canvas exposes the full power of Kludgine", Color::WHITE)
                .origin(TextOrigin::Center)
                .translate_by(center - Point::new(Px(0), Px(100))),
        );
        context.gfx.draw_shape(
            Shape::filled_rect(
                Rect::new(Point::new(Px(-50), Px(-50)), Size::new(Px(100), Px(100))),
                Color::RED,
            )
            .translate_by(center)
            .rotate_by(angle),
        )
    })
    .tick(Tick::redraws_per_second(60))
    .run()
}
