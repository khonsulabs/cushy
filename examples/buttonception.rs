//! This example shows off how buttons are able to use any widget, including
//! buttons, as their "label". The widget hierarchy constructed for this example
//! is:
//!
//! ```text
//! ┏ Align
//! ┃ ┏ Stack
//! ┃ ┃ ┏ Button
//! ┃ ┃ ┃ ┏ Stack
//! ┃ ┃ ┃ ┃ ┏ "Yo dawg!"
//! ┃ ┃ ┃ ┃ ┣ Button
//! ┃ ┃ ┃ ┃ ┃ ┏ Stack
//! ┃ ┃ ┃ ┃ ┃ ┃ ┏ "I heard you like buttons"
//! ┃ ┃ ┃ ┃ ┃ ┃ ┣ Button
//! ┃ ┃ ┃ ┃ ┃ ┃ ┃ ╺ "So I put buttons in your buttons"
//! ┃ ┃ ┣ clicked_button Label
//! ```

use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::button::{ButtonHoverBackground, ButtonHoverForeground};
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    let clicked_button = Dynamic::<&'static str>::default();

    let inner_button = "So I put buttons in your buttons"
        .into_button()
        .on_click({
            let clicked_button = clicked_button.clone();
            move |()| clicked_button.set("inner button clicked")
        })
        .with(&ButtonHoverBackground, Color::RED)
        .with(&ButtonHoverForeground, Color::WHITE);

    let middle_button = "I heard you like buttons"
        .and(inner_button)
        .into_rows()
        .into_button()
        .on_click({
            let clicked_button = clicked_button.clone();
            move |()| clicked_button.set("middle button clicked")
        });

    let outer_button = "Yo dawg!"
        .and(middle_button)
        .into_rows()
        .into_button()
        .on_click({
            let clicked_button = clicked_button.clone();
            move |()| clicked_button.set("outer button clicked")
        });

    outer_button
        .and(clicked_button)
        .into_rows()
        .centered()
        .run()
}
