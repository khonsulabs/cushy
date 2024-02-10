use cushy::widget::MakeWidget;
use cushy::Run;

fn main() -> cushy::Result {
    include_str!("../src/widgets/scroll.rs")
        .scroll()
        .expand()
        .run()
}
