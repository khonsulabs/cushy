use gooey_core::Children;
use gooey_widgets::{Button, Flex, Label};

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counter = cx.new_dynamic(0i32);
        let label = counter.map_each(|count| count.to_string()).unwrap();

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
