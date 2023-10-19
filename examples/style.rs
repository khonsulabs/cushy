use gooey::children::Children;
use gooey::styles::{Styles, TextColor};
use gooey::widget::Widget;
use gooey::widgets::array::Array;
use gooey::widgets::{Button, Style};
use gooey::window::Window;
use gooey::EventLoopError;
use kludgine::Color;

fn main() -> Result<(), EventLoopError> {
    Window::for_widget(Array::rows(
        Children::new()
            .with_widget(Button::new("Default"))
            .with_widget(styled(Button::new("Styled"))),
    ))
    .styles(Styles::new().with(&TextColor, Color::GREEN))
    .run()
}

fn styled(w: impl Widget) -> Style {
    Style::new(Styles::new().with(&TextColor, Color::RED), w)
}
