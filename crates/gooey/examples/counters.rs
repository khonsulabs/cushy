use gooey_core::WidgetValue;
use gooey_widgets::{Button, Flex};

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counters = cx.new_value(gooey_widgets::FlexChildren::new(cx.clone()));

        Flex::rows(cx)
            .with(|_| {
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
                                        label.replace(counter.to_string());
                                    })
                                })
                            }
                        });
                    })
            })
            .with(|_| Flex {
                children: WidgetValue::Value(counters),
            })
            .finish()
    })
}
