use gooey::dynamic::Dynamic;
use gooey::widget::Widget;
use gooey::widgets::Button;
use gooey::EventLoopError;

fn main() -> Result<(), EventLoopError> {
    let count = Dynamic::new(0_usize);
    Button::new(count.map_each(ToString::to_string))
        .on_click(count.with_clone(|count| move |_| count.set(count.get() + 1)))
        .run()
}
