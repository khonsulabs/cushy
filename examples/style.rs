use gooey::styles::components::TextColor;
use gooey::styles::Styles;
use gooey::widget::{Widget, Widgets};
use gooey::widgets::array::Array;
use gooey::widgets::{Button, Style};
use gooey::window::Window;
use gooey::{styles, Run};
use kludgine::Color;

fn main() -> gooey::Result {
    Window::for_widget(
        Array::rows(
            Widgets::new()
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
