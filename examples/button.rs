use gooey::value::Dynamic;
use gooey::widgets::Button;
use gooey::Run;

// begin rustme snippet: readme
fn main() -> gooey::Result {
    // Create a dynamic usize.
    let count = Dynamic::new(0_usize);

    // Create a new button with a label that is produced by mapping the contents
    // of `count`.
    Button::new(count.map_each(ToString::to_string))
        // Set the `on_click` callback to a closure that increments the counter.
        .on_click(count.with_clone(|count| move |_| count.set(count.get() + 1)))
        // Run the button as an an application.
        .run()
}
// end rustme snippet
