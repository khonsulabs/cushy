use cushy::styles::Color;
use cushy::widget::MakeWidget;
use cushy::widgets::Space;
use cushy::Run;

fn main() -> cushy::Result {
    Space::colored(Color::RED)
        .and("Layers stack widgets on top of each other")
        .into_layers()
        .centered()
        .run()
}
