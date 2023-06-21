use gooey_core::Children;
use gooey_widgets::{Button, Flex};

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counters = cx.new_value(Children::new(cx));

        Flex::rows(
            Children::new(cx)
                .with_widget(
                    Button::default()
                        .label("Create Counter")
                        .on_click(move |_| {
                            counters.map_mut({
                                |counters| {
                                    counters.push(|cx| {
                                        let label = cx.new_value(String::from("0"));
                                        let mut counter = 0;
                                        Button::default().label(label).on_click(move |_| {
                                            counter += 1;
                                            label.set(counter.to_string());
                                        })
                                    })
                                }
                            });
                        }),
                )
                .with_widget(Flex::columns(counters)),
        )
    })
}
