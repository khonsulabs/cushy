use gooey_widgets::{Button, Flex, Label};

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let label = cx.new_value(String::from("0"));
        let mut counter = 0;

        Flex::rows(cx)
            .with_widget(Label::new(label))
            .with_widget(Button::new("Increment").on_click(move |_| {
                counter += 1;
                label.replace(counter.to_string());
            }))
            .finish()
    })
}
