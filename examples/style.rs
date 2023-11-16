use gooey::styles::components::TextColor;
use gooey::widget::MakeWidget;
use gooey::widgets::stack::Stack;
use gooey::widgets::Style;
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    Stack::rows("Green".and(red_text("Red")))
        .with(&TextColor, Color::GREEN)
        .run()
}

/// Creating reusable style helpers that work with any Widget is straightfoward
fn red_text(w: impl MakeWidget) -> Style {
    w.with(&TextColor, Color::RED)
}
