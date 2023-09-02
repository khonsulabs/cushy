use std::string::ToString;

use gooey::App;
use gooey_core::EventLoopError;
use gooey_widgets::Button;

fn main() -> Result<(), EventLoopError> {
    App::default().run(|cx, _window| {
        let counter = cx.new_dynamic(0i32);
        let label = counter.map_each(ToString::to_string).unwrap();
        Button::new(label).on_click(move |_| {
            counter.set(counter.get().unwrap() + 1);
        })
    })
}
