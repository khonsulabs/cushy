use cushy::styles::components::CornerRadius;
use cushy::styles::Dimension;
use cushy::value::{Dynamic, MapEachCloned};
use cushy::widget::MakeWidget;
use cushy::widgets::container::ContainerShadow;
use cushy::widgets::slider::Slidable;
use cushy::Run;
use figures::units::Lp;
use figures::{Point, Size};
use kludgine::shapes::CornerRadii;

fn main() -> cushy::Result {
    let top_left = Dynamic::new(Lp::mm(1));
    let top_right = Dynamic::new(Lp::mm(1));
    let bottom_right = Dynamic::new(Lp::mm(1));
    let bottom_left = Dynamic::new(Lp::mm(1));
    let corners = (&top_left, &top_right, &bottom_right, &bottom_left).map_each_cloned(
        |(top_left, top_right, bottom_right, bottom_left)| {
            CornerRadii {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
            }
            .map(Dimension::from)
        },
    );

    let offset_x = Dynamic::new(Lp::ZERO);
    let offset_y = Dynamic::new(Lp::ZERO);
    let offset = (&offset_x, &offset_y).map_each_cloned(|(x, y)| Point::new(x, y));

    let radius = Dynamic::new(Lp::mm(1));
    let spread = Dynamic::new(Lp::mm(1));

    let shadow = (&offset, &radius, &spread).map_each_cloned(|(offset, radius, spread)| {
        ContainerShadow::new(offset)
            .blur_radius(radius)
            .spread(spread)
    });

    "Corner Radii"
        .h3()
        .and("Top Left")
        .and(top_left.slider_between(Lp::ZERO, Lp::inches(1)))
        .and("Top right")
        .and(top_right.slider_between(Lp::ZERO, Lp::inches(1)))
        .and("Bottom Right")
        .and(bottom_right.slider_between(Lp::ZERO, Lp::inches(1)))
        .and("Bottom Left")
        .and(bottom_left.slider_between(Lp::ZERO, Lp::inches(1)))
        .and("Shadow".h3())
        .and("Offset X")
        .and(offset_x.slider_between(Lp::inches_f(-0.5), Lp::inches_f(0.5)))
        .and("Offset Y")
        .and(offset_y.slider_between(Lp::inches_f(-0.5), Lp::inches_f(0.5)))
        .and("Radius")
        .and(radius.slider_between(Lp::ZERO, Lp::inches_f(0.5)))
        .and("Spread")
        .and(spread.slider_between(Lp::ZERO, Lp::inches_f(0.5)))
        .into_rows()
        .and(
            "Preview"
                .h3()
                .and(
                    "Hello, World!"
                        .size(Size::squared(Lp::inches(2)))
                        .contain()
                        .shadow(shadow)
                        .with(&CornerRadius, corners)
                        .centered()
                        .contain()
                        .expand(),
                )
                .into_rows()
                .expand(),
        )
        .into_columns()
        .expand()
        .contain()
        .pad()
        .run()
}
