use cushy::figures::units::Lp;
use cushy::reactive::value::Dynamic;
use cushy::styles::components::{LineHeight, TextSize, VerticalAlignment};
use cushy::styles::VerticalAlign;
use cushy::widget::{MakeWidget, WidgetList};
use cushy::widgets::wrap::WrapAlign;
use cushy::Run;
use rand::{rng, Rng};

const EXPLANATION: &str = "This example demonstrates the Wrap widget. Each word shown here is an individual Label widget that is being positioned by the Wrap widget.";

fn main() -> cushy::Result {
    let mut rng = rng();
    let words = EXPLANATION
        .split_ascii_whitespace()
        .map(|word| {
            let text_size = Lp::points(rng.random_range(14..48));
            word.with(&TextSize, text_size).with(&LineHeight, text_size)
        })
        .collect::<WidgetList>();

    let align = Dynamic::<WrapAlign>::default();
    let vertical_align = Dynamic::<VerticalAlign>::default();

    let editors = "Settings"
        .h3()
        .and(
            "Wrap Align"
                .h5()
                .and(align.new_radio(WrapAlign::Start).labelled_by("Start"))
                .and(align.new_radio(WrapAlign::End).labelled_by("End"))
                .and(align.new_radio(WrapAlign::Center).labelled_by("Center"))
                .and(
                    align
                        .new_radio(WrapAlign::SpaceAround)
                        .labelled_by("Space Around"),
                )
                .and(
                    align
                        .new_radio(WrapAlign::SpaceEvenly)
                        .labelled_by("Space Evenly"),
                )
                .and(
                    align
                        .new_radio(WrapAlign::SpaceBetween)
                        .labelled_by("Space Between"),
                )
                .into_rows()
                .contain(),
        )
        .and(
            "Vertical Align"
                .h5()
                .and(
                    vertical_align
                        .new_radio(VerticalAlign::Top)
                        .labelled_by("Top"),
                )
                .and(
                    vertical_align
                        .new_radio(VerticalAlign::Center)
                        .labelled_by("Center"),
                )
                .and(
                    vertical_align
                        .new_radio(VerticalAlign::Bottom)
                        .labelled_by("Bottom"),
                )
                .into_rows()
                .contain(),
        )
        .into_rows();

    let preview = "Preview"
        .h3()
        .and(
            words
                .into_wrap()
                .align(align)
                .with(&VerticalAlignment, vertical_align)
                .expand_horizontally()
                .contain()
                .pad()
                .expand(),
        )
        .into_rows()
        .expand();

    editors.and(preview).into_columns().pad().run()
}
