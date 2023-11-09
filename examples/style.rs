use gooey::styles::components::TextColor;
use gooey::widget::{MakeWidget, Widget};
use gooey::widgets::stack::Stack;
use gooey::widgets::{Button, Style};
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    Stack::rows(Button::new("Green").and(red_text(Button::new("Red"))))
        .with(&TextColor, Color::GREEN)
        .run()
}

/// Creating reusable style helpers that work with any Widget is straightfoward
fn red_text(w: impl Widget) -> Style {
    w.with(&TextColor, Color::RED)
}
