use gooey::widget::MakeWidget;
use gooey::Run;

fn main() -> gooey::Result {
    include_str!("../src/widgets/scroll.rs")
        .scroll()
        .expand()
        .run()
}
