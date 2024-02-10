use std::array;

use cushy::figures::units::{Lp, Px};
use cushy::figures::{Point, Size};
use cushy::styles::ThemePair;
use cushy::widget::MakeWidget;
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::{Grid, Space};
use guide_examples::book_example;

// ANCHOR: content
fn content() -> impl MakeWidget {
    Space::primary().size(Size::squared(Px::new(32)..))
}
// ANCHOR_END: content

fn align_left() -> impl MakeWidget {
    // ANCHOR: align-left
    content().align_left()
    // ANCHOR_END: align-left
}

fn centered() -> impl MakeWidget {
    // ANCHOR: horizontal-center
    content().centered()
    // ANCHOR_END: horizontal-center
}

fn align_right() -> impl MakeWidget {
    // ANCHOR: align-right
    content().align_right()
    // ANCHOR_END: align-right
}

fn align_horizontal() -> impl MakeWidget {
    Grid::from_rows(
        GridWidgets::new()
            .and(("Unaligned", content()))
            .and(("align_left()", align_left()))
            .and(("centered()", centered()))
            .and(("align_right()", align_right())),
    )
    .dimensions([
        GridDimension::FitContent,
        GridDimension::Fractional { weight: 1 },
    ])
}

fn align_top() -> impl MakeWidget {
    // ANCHOR: align-top
    content().align_top()
    // ANCHOR_END: align-top
}

fn align_bottom() -> impl MakeWidget {
    // ANCHOR: align-bottom
    content().align_bottom()
    // ANCHOR_END: align-bottom
}

fn align_vertical() -> impl MakeWidget {
    Grid::from_rows(
        GridWidgets::new()
            .and(("Unaligned", "align_top()", "centered()", "align_bottom()"))
            .and((
                content().height(Lp::inches(1)).centered(),
                align_top(),
                centered(),
                align_bottom(),
            )),
    )
    .dimensions(array::from_fn(|_| GridDimension::Fractional { weight: 1 }))
}

fn align() -> impl MakeWidget {
    "Horizontal Alignment"
        .and(align_horizontal().contain())
        .and("Vertical Alignment")
        .and(align_vertical().contain())
        .into_rows()
}

fn main() {
    let theme = ThemePair::default();
    let container_color = theme.dark.surface.low_container;
    let primary = theme.dark.primary.color;
    book_example!(align).still_frame(|recorder| {
        const LEFT: u32 = 145;
        const RIGHT: u32 = 705;
        const H_CENTER: u32 = (RIGHT + LEFT) / 2;
        const TOP: u32 = 282;
        const BOTTOM: u32 = 345;
        const V_CENTER: u32 = (TOP + BOTTOM) / 2;

        // Verify the inner container color
        recorder.assert_pixel_color(Point::new(32, 62), container_color, "surface");

        // Default fills the entire space
        recorder.assert_pixel_color(Point::new(LEFT, 78), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(H_CENTER, 78), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(RIGHT, 78), primary, "default spacer");

        // align-left
        recorder.assert_pixel_color(Point::new(LEFT, 110), primary, "align-left spacer");
        recorder.assert_pixel_color(
            Point::new(H_CENTER, 110),
            container_color,
            "align-left empty",
        );

        // centered
        recorder.assert_pixel_color(Point::new(H_CENTER, 142), primary, "centered spacer");
        recorder.assert_pixel_color(
            Point::new(LEFT, 142),
            container_color,
            "centered empty before",
        );
        recorder.assert_pixel_color(
            Point::new(RIGHT, 142),
            container_color,
            "centered empty after",
        );

        // align-right
        recorder.assert_pixel_color(Point::new(RIGHT, 175), primary, "align-right spacer");
        recorder.assert_pixel_color(
            Point::new(V_CENTER, 175),
            container_color,
            "align-right empty",
        );

        // Default fills the entire space
        recorder.assert_pixel_color(Point::new(115, TOP), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(115, V_CENTER), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(115, BOTTOM), primary, "default spacer");

        // align-top
        recorder.assert_pixel_color(Point::new(285, TOP), primary, "align-top spacer");
        recorder.assert_pixel_color(
            Point::new(285, V_CENTER),
            container_color,
            "align-top empty",
        );

        // centered
        recorder.assert_pixel_color(Point::new(460, V_CENTER), primary, "centered spacer");
        recorder.assert_pixel_color(
            Point::new(460, TOP),
            container_color,
            "centered empty before",
        );
        recorder.assert_pixel_color(
            Point::new(460, BOTTOM),
            container_color,
            "centered empty after",
        );

        // align-bottom
        recorder.assert_pixel_color(Point::new(635, BOTTOM), primary, "align-bottom spacer");
        recorder.assert_pixel_color(
            Point::new(635, V_CENTER),
            container_color,
            "align-bottom empty",
        );
    });
}

#[test]
fn runs() {
    main();
}
