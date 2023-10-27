use std::string::ToString;

use gooey::children::Children;
use gooey::dynamic::Dynamic;
use gooey::widgets::array::Array;
use gooey::widgets::{Button, Label};
use gooey::{EventLoopError, Run};

fn main() -> Result<(), EventLoopError> {
    let counter = Dynamic::new(0i32);
    let label = counter.map_each(ToString::to_string);
    Array::rows(
        Children::new()
            .with_widget(Label::new(label))
            .with_widget(Button::new("+").on_click(counter.with_clone(|counter| {
                move |_| {
                    counter.set(counter.get() + 1);
                }
            })))
            .with_widget(Button::new("-").on_click(counter.with_clone(|counter| {
                move |_| {
                    counter.set(counter.get() - 1);
                }
            }))),
    )
    .run()
}
