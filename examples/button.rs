use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::button::ButtonOutline;
use gooey::widgets::Button;
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    // begin rustme snippet: readme
    // Create a dynamic usize.
    let count = Dynamic::new(0_isize);

    // Create a new button with a label that is produced by mapping the contents
    // of `count`.
    Button::new(count.map_each(ToString::to_string))
        // Set the `on_click` callback to a closure that increments the counter.
        .on_click(count.with_clone(|count| move |_| count.set(count.get() + 1)))
        .and(
            // Creates a second, outlined button
            Button::new(count.map_each(ToString::to_string))
                // Set the `on_click` callback to a closure that decrements the counter.
                .on_click(count.with_clone(|count| move |_| count.set(count.get() - 1)))
                .with(&ButtonOutline, Color::DARKRED),
        )
        .in_columns()
        // Run the button as an an application.
        .run()
    // end rustme snippet
}
