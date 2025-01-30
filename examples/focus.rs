use cushy::figures::units::Lp;
use cushy::reactive::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::input::InputValue;
use cushy::widgets::slider::Slidable;
use cushy::widgets::Custom;
use cushy::Run;

fn main() -> cushy::Result {
    let allow_blur = Dynamic::new(true);
    "Input Field"
        .and(Dynamic::<String>::default().into_input())
        .and("Range Slider")
        .and(Dynamic::<u8>::default().slider_between(0_u8, 100_u8))
        .and("Range Slider")
        .and(Dynamic::new(10..=30).slider_between(0_u8, 100_u8))
        .and("Allow Custom Widget to Lose Focus".into_checkbox(allow_blur.clone()))
        .and(
            Custom::empty()
                .on_accept_focus(|context| context.enabled())
                .on_redraw(|context| {
                    context.fill(context.theme().secondary.color);
                    if context.focused(true) {
                        context.draw_focus_ring();
                    }
                })
                .on_allow_blur(move |_| allow_blur.get())
                .height(Lp::inches(1)),
        )
        .into_rows()
        .run()
}
