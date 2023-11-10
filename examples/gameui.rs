use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, HANDLED, IGNORED};
use gooey::widgets::{Input, Label, Space, Stack};
use gooey::Run;
use kludgine::app::winit::event::ElementState;
use kludgine::app::winit::keyboard::Key;
use kludgine::Color;

fn main() -> gooey::Result {
    let chat_log = Dynamic::new("Chat log goes here.\n".repeat(100));
    let chat_message = Dynamic::new(String::new());

    Stack::rows(
        Stack::columns(
            Label::new(chat_log.clone())
                .vertical_scroll()
                .expand()
                .and(Space::colored(Color::RED).expand_weighted(2)),
        )
        .expand()
        .and(Input::new(chat_message.clone()).on_key(move |input| {
            match (input.state, input.logical_key) {
                (ElementState::Pressed, Key::Enter) => {
                    let new_message = chat_message.map_mut(|text| std::mem::take(text));
                    chat_log.map_mut(|chat_log| {
                        chat_log.push_str(&new_message);
                        chat_log.push('\n');
                    });
                    HANDLED
                }
                _ => IGNORED,
            }
        })),
    )
    .expand()
    .run()
}
