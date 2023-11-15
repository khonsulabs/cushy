use gooey::widget::MakeWidget;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    include_str!("./nested-scroll.rs")
        .vertical_scroll()
        .height(Lp::inches(3))
        .and(
            include_str!("./canvas.rs")
                .vertical_scroll()
                .height(Lp::inches(3)),
        )
        .into_rows()
        .vertical_scroll()
        .expand()
        .run()
}
