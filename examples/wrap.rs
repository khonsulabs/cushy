use cushy::styles::components::{LineHeight, TextSize};
use cushy::value::Dynamic;
use cushy::widget::{Children, MakeWidget};
use cushy::widgets::wrap::{VerticalAlign, WrapAlign};
use cushy::Run;
use kludgine::figures::units::Lp;
use rand::{thread_rng, Rng};

const EXPLANATION: &str = "This example demonstrates the Wrap widget. Each word shown here is an individual Label widget that is being positioned by the Wrap widget.";

fn main() -> cushy::Result {
    let mut rng = thread_rng();
    let words = EXPLANATION
        .split_ascii_whitespace()
        .map(|word| {
            let text_size = Lp::points(rng.gen_range(14..48));
            word.with(&TextSize, text_size).with(&LineHeight, text_size)
        })
        .collect::<Children>();

    let align = Dynamic::<WrapAlign>::default();
    let vertical_align = Dynamic::<VerticalAlign>::default();

    let editors = "Settings"
        .h3()
        .and(
            "Wrap Align"
                .h5()
                .and(align.new_radio(WrapAlign::Start, "Start"))
                .and(align.new_radio(WrapAlign::End, "End"))
                .and(align.new_radio(WrapAlign::Center, "Center"))
                .and(align.new_radio(WrapAlign::SpaceAround, "Space Around"))
                .and(align.new_radio(WrapAlign::SpaceEvenly, "Space Evenly"))
                .and(align.new_radio(WrapAlign::SpaceBetween, "Space Between"))
                .into_rows()
                .contain(),
        )
        .and(
            "Vertical Align"
                .h5()
                .and(vertical_align.new_radio(VerticalAlign::Top, "Top"))
                .and(vertical_align.new_radio(VerticalAlign::Middle, "Middle"))
                .and(vertical_align.new_radio(VerticalAlign::Bottom, "Bottom"))
                .into_rows()
                .contain(),
        )
        .into_rows();

    let preview = "Preview"
        .h3()
        .and(
            words
                .wrap()
                .align(align)
                .vertical_align(vertical_align)
                .expand_horizontally()
                .contain()
                .pad()
                .expand(),
        )
        .into_rows()
        .expand();

    editors.and(preview).into_columns().pad().run()
}
