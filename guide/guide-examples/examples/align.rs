use cushy::figures::units::{Lp, Px};
use cushy::figures::{Point, Size};
use cushy::styles::{Edges, ThemePair};
use cushy::widget::MakeWidget;
use cushy::widgets::Space;
use guide_examples::BookExample;

fn content() -> impl MakeWidget {
    Space::primary().size(Size::squared(Px::new(32)))
}

fn main() {
    BookExample::new(
        "align-horizontal",
        "Default Behavior"
            .and(content())
            .and("align_left()")
            .and({
                // ANCHOR: align-left
                content().align_left()
                // ANCHOR_END: align-left
            })
            .and("pad_by().align_left()")
            .and({
                // ANCHOR: align-left-pad
                content()
                    .pad_by(Edges::default().with_left(Lp::inches(1)))
                    .align_left()
                // ANCHOR_END: align-left-pad
            })
            .and("centered()")
            .and({
                // ANCHOR: centered
                content().centered()
                // ANCHOR_END: centered
            })
            .and("pad_by().align_right()")
            .and({
                // ANCHOR: align-right-pad
                content()
                    .pad_by(Edges::default().with_right(Lp::inches(1)))
                    .align_right()
                // ANCHOR_END: align-right-pad
            })
            .and("align_right()")
            .and({
                // ANCHOR: align-right
                content().align_right()
                // ANCHOR_END: align-right
            })
            .into_rows(),
    )
    .still_frame(|recorder| {
        const LEFT: u32 = 40;
        const PADDING: u32 = 96;
        const RIGHT: u32 = 710;
        const CENTER: u32 = 375;

        let container_color = ThemePair::default().dark.surface.lowest_container;
        let primary = ThemePair::default().dark.primary.color;

        recorder.assert_pixel_color(Point::new(LEFT, 35), container_color, "surface");

        // Default fills the entire space
        recorder.assert_pixel_color(Point::new(LEFT, 70), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(CENTER, 70), primary, "default spacer");
        recorder.assert_pixel_color(Point::new(RIGHT, 70), primary, "default spacer");

        // align-left
        recorder.assert_pixel_color(Point::new(LEFT, 140), primary, "align-left spacer");
        recorder.assert_pixel_color(
            Point::new(LEFT + PADDING, 140),
            container_color,
            "align-left empty",
        );

        // align-left-pad
        recorder.assert_pixel_color(
            Point::new(LEFT + PADDING, 215),
            primary,
            "align-left-pad spacer",
        );
        recorder.assert_pixel_color(
            Point::new(LEFT, 215),
            container_color,
            "align-left-pad empty before",
        );
        recorder.assert_pixel_color(
            Point::new(CENTER, 215),
            container_color,
            "align-left-pad empty after",
        );

        // centered
        recorder.assert_pixel_color(Point::new(CENTER, 295), primary, "centered spacer");
        recorder.assert_pixel_color(
            Point::new(LEFT + PADDING, 295),
            container_color,
            "centered empty before",
        );
        recorder.assert_pixel_color(
            Point::new(RIGHT - PADDING, 295),
            container_color,
            "centered empty after",
        );

        // align-right-pad
        recorder.assert_pixel_color(
            Point::new(RIGHT - PADDING, 360),
            primary,
            "align-right-pad spacer",
        );
        recorder.assert_pixel_color(
            Point::new(CENTER, 360),
            container_color,
            "align-right-pad empty before",
        );
        recorder.assert_pixel_color(
            Point::new(RIGHT, 360),
            container_color,
            "align-right-pad empty after",
        );

        // align-right
        recorder.assert_pixel_color(Point::new(RIGHT, 435), primary, "align-right spacer");
        recorder.assert_pixel_color(
            Point::new(RIGHT - PADDING, 435),
            container_color,
            "align-right empty",
        );
    });
}

#[test]
fn runs() {
    main();
}
