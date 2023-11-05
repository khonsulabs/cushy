use std::string::ToString;

use gooey::value::Dynamic;
use gooey::widgets::{Align, Button, Expand, Label, Resize, Stack};
use gooey::{children, Run};
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let counter = Dynamic::new(0i32);
    let label = counter.map_each(ToString::to_string);
    Expand::new(Align::centered(Stack::columns(children![
        Resize::width(Lp::points(100), Label::new(label)),
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
    ])))
    .run()
}
