use gooey::children::Children;
use gooey::styles::{Styles, TextColor};
use gooey::widget::Widget;
use gooey::widgets::array::Array;
use gooey::widgets::{Button, Style};
use gooey::window::Window;
use gooey::{styles, EventLoopError, Run};
use kludgine::Color;

fn main() -> Result<(), EventLoopError> {
    Window::for_widget(
        Array::rows(
            Children::new()
                .with_widget(Button::new("Default"))
                .with_widget(red_text(Button::new("Styled"))),
        )
        .with_styles(Styles::new().with(&TextColor, Color::GREEN)),
    )
    .run()
}

/// Creating reusable style helpers that work with any Widget is straightfoward
fn red_text(w: impl Widget) -> Style {
    Style::new(styles!(TextColor => Color::RED), w)
}
