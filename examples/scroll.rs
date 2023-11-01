use gooey::value::Dynamic;
use gooey::widget::Widgets;
use gooey::widgets::{Button, Scroll, Stack};
use gooey::Run;

fn main() -> gooey::Result {
    Scroll::new(Stack::rows(
        (0..30)
            .map(|i| {
                let count = Dynamic::new(0);

                Button::new(count.map_each(move |count| format!("Row {i}: {count}"))).on_click(
                    move |_| {
                        count.map_mut(|count| *count += 1);
                    },
                )
            })
            .collect::<Widgets>(),
    ))
    .run()
}
