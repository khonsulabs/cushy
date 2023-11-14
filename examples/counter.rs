use std::string::ToString;

use gooey::value::{Dynamic, StringValue};
use gooey::widget::MakeWidget;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let counter = Dynamic::new(0i32);
    let label = counter.map_each(ToString::to_string);

    label
        .width(Lp::points(100))
        .and("+".into_button().on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() += 1;
            }
        })))
        .and("-".into_button().on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() -= 1;
            }
        })))
        .into_columns()
        .centered()
        .expand()
        .run()
}
