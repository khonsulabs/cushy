use gooey_core::style::{FontSize, Px};
use gooey_core::Children;
use gooey_widgets::{Button, Flex, Label, LabelExt as _};

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counter = cx.new_value(10i32);
        let label = counter.map_each(|count| count.to_string()).unwrap();
        let font_size = counter
            .map_each(|count| FontSize::from(Px(*count)))
            .unwrap();

        Flex::rows(
            Children::new(cx)
                .with_widget(Label::new(label, cx).font_size(font_size))
                .with_widget(Button::new("+").on_click(move |_| {
                    counter.set(counter.get().unwrap() + 1);
                }))
                .with_widget(Button::new("-").on_click(move |_| {
                    counter.set(counter.get().unwrap().saturating_sub(1));
                })),
        )
    })
}
