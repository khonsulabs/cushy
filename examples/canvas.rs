use cushy::figures::{Angle, IntoSigned, Point, Px2D, Rect, Size};
use cushy::widgets::Canvas;
use cushy::{Run, Tick};
use kludgine::shapes::Shape;
use kludgine::text::{Text, TextOrigin};
use kludgine::{Color, DrawableExt};

fn main() -> cushy::Result<()> {
    let mut angle = Angle::degrees(0);
    Canvas::new(move |context| {
        angle += Angle::degrees(1);

        let center = Point::from(context.gfx.size()).into_signed() / 2;
        context.gfx.draw_text(
            Text::new("Canvas exposes the full power of Kludgine", Color::WHITE)
                .origin(TextOrigin::Center)
                .translate_by(center - Point::px(0, 100)),
        );
        context.gfx.draw_shape(
            Shape::filled_rect(
                Rect::new(Point::px(-50, -50), Size::px(100, 100)),
                Color::RED,
            )
            .translate_by(center)
            .rotate_by(angle),
        )
    })
    .tick(Tick::redraws_per_second(60))
    .run()
}
