use std::string::ToString;

use gooey::App;
use gooey_core::{Children, EventLoopError};
use gooey_widgets::{Button, Flex, Label};

fn main() -> Result<(), EventLoopError> {
    App::default().run(|cx, _window| {
        let counter = cx.new_dynamic(0i32);
        let label = counter.map_each(ToString::to_string).unwrap();

        Flex::rows(
            Children::new(cx)
                .with_widget(Label::new(label, cx))
                .with_widget(Button::new("+").on_click(move |_| {
                    counter.set(counter.get().unwrap() + 1);
                }))
                .with_widget(Button::new("-").on_click(move |_| {
                    counter.set(counter.get().unwrap().saturating_sub(1));
                })),
        )
    })
}
