use gooey::styles::components::TextColor;
use gooey::styles::Styles;
use gooey::widget::{MakeWidget, Widget};
use gooey::widgets::stack::Stack;
use gooey::widgets::{Button, Style};
use gooey::{styles, Run};
use kludgine::Color;

fn main() -> gooey::Result {
    Stack::rows(Button::new("Green").and(red_text(Button::new("Red"))))
        .with_styles(Styles::new().with(&TextColor, Color::GREEN))
        .run()
}

/// Creating reusable style helpers that work with any Widget is straightfoward
fn red_text(w: impl Widget) -> Style {
    Style::new(styles!(TextColor => Color::RED), w)
}
