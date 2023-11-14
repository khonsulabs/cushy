use std::string::ToString;

use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::{Button, Label};
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let counter = Dynamic::new(0i32);
    let label = counter.map_each(ToString::to_string);

    Label::new(label)
        .width(Lp::points(100))
        .and(Button::new("+").on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() += 1;
            }
        })))
        .and(Button::new("-").on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() -= 1;
            }
        })))
        .into_columns()
        .centered()
        .expand()
        .run()
}
