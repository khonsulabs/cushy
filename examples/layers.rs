use gooey::widget::MakeWidget;
use gooey::widgets::Space;
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    Space::colored(Color::RED)
        .and("Layers stack widgets on top of each other")
        .into_layers()
        .centered()
        .run()
}
