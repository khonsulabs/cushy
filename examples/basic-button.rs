use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::Run;

// begin rustme snippet: readme
fn main() -> gooey::Result {
    // Create a dynamic usize.
    let count = Dynamic::new(0_isize);
    // Create a dynamic that contains `count.to_string()`
    let count_label = count.map_each(ToString::to_string);

    // Create a new button whose text is our dynamic string.
    count_label
        .into_button()
        // Set the `on_click` callback to a closure that increments the counter.
        .on_click(count.with_clone(|count| move |_| count.set(count.get() + 1)))
        // Position the button in the center
        .centered()
        // Run the application
        .run()
}
// end rustme snippet
