use gooey::widget::MakeWidget;
use gooey::widgets::Label;
use gooey::Run;

fn main() -> gooey::Result {
    Label::new(include_str!("../src/widgets/scroll.rs"))
        .scroll()
        .run()
}
