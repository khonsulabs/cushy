use gooey::App;
use gooey_core::events::MouseEvent;
use gooey_core::{Children, Context};
use gooey_widgets::{Button, Flex};

fn position_button(cx: &Context) -> Button {
    let label = cx.new_dynamic("click".to_string());
    Button::new(label).on_click(move |event: MouseEvent| {
        label.set(format!("{event:?}"));
    })
}

fn main() {
    App::default().run(|cx, _window| {
        Flex::rows(Children::new(cx).with(position_button).with(|cx| {
            Flex::columns(
                Children::new(cx)
                    .with(position_button)
                    .with(position_button),
            )
        }))
    })
}
