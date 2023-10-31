use gooey::value::Dynamic;
use gooey::widgets::Button;
use gooey::Run;

fn main() -> gooey::Result {
    let count = Dynamic::new(0_usize);
    Button::new(count.map_each(ToString::to_string))
        .on_click(count.with_clone(|count| move |_| count.set(count.get() + 1)))
        .run()
}
