use cushy::reactive::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::label::Displayable;
use cushy::Run;

// begin rustme snippet: readme
fn main() -> cushy::Result {
    // Create a dynamic usize.
    let count = Dynamic::new(0_isize);

    // Create a new label displaying `count`
    count
        .to_label()
        // Use the label as the contents of a button
        .into_button()
        // Set the `on_click` callback to a closure that increments the counter.
        .on_click(move |_| count.set(count.get() + 1))
        // Run the application
        .run()
}
// end rustme snippet
