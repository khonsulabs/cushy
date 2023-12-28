use cushy::styles::components::{TextColor, TextSize};
use cushy::widget::MakeWidget;
use cushy::widgets::stack::Stack;
use cushy::widgets::Style;
use cushy::Run;
use kludgine::figures::units::Lp;
use kludgine::Color;

fn main() -> cushy::Result {
    Stack::rows("Green".and(red_text("Red")))
        .with(&TextColor, Color::GREEN)
        // Local styles are not inherited. In this situation, the text size is
        // being applied to the stack, which has no text. The labels are
        // children of the stack, and they will render at the default text size.
        .with_local(&TextSize, Lp::inches(10))
        .run()
}

/// Creating reusable style helpers that work with any Widget is straightfoward
fn red_text(w: impl MakeWidget) -> Style {
    w.with(&TextColor, Color::RED)
}
