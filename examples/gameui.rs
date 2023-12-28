use cushy::value::Dynamic;
use cushy::widget::{MakeWidget, HANDLED, IGNORED};
use cushy::widgets::input::InputValue;
use cushy::Run;
use kludgine::app::winit::event::ElementState;
use kludgine::app::winit::keyboard::{Key, NamedKey};
use kludgine::Color;

fn main() -> cushy::Result {
    let chat_log = Dynamic::new("Chat log goes here.\n".repeat(100));
    let chat_message = Dynamic::new(String::new());

    chat_log
        .clone()
        .vertical_scroll()
        .expand()
        .and(Color::RED.expand_weighted(2))
        .into_columns()
        .expand()
        .and(chat_message.clone().into_input().on_key(move |input| {
            match (input.state, input.logical_key) {
                (ElementState::Pressed, Key::Named(NamedKey::Enter)) => {
                    let new_message = chat_message.map_mut(std::mem::take);
                    chat_log.map_mut(|chat_log| {
                        chat_log.push_str(&new_message);
                        chat_log.push('\n');
                    });
                    HANDLED
                }
                _ => IGNORED,
            }
        }))
        .into_rows()
        .expand()
        .run()
}
