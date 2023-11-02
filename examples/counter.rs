use std::string::ToString;

use gooey::value::Dynamic;
use gooey::widgets::{Button, Label, Spacing, Stack};
use gooey::{widgets, Run};

fn main() -> gooey::Result {
    let counter = Dynamic::new(0i32);
    let label = counter.map_each(ToString::to_string);
    Spacing::auto(Stack::columns(widgets![
        Label::new(label),
        Button::new("+").on_click(counter.with_clone(|counter| {
            move |_| {
                counter.set(counter.get() + 1);
            }
        })),
        Button::new("-").on_click(counter.with_clone(|counter| {
            move |_| {
                counter.set(counter.get() - 1);
            }
        })),
    ]))
    .run()
}
