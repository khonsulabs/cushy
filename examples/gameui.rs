use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::Color;
use cushy::value::{Destination, Dynamic};
use cushy::widget::{MakeWidget, HANDLED, IGNORED};
use cushy::widgets::input::InputValue;
use cushy::Run;

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
        .and(
            chat_message
                .to_input()
                .on_key(move |input| match (input.state, input.logical_key) {
                    (ElementState::Pressed, Key::Named(NamedKey::Enter)) => {
                        let new_message = chat_message.take();
                        chat_log.map_mut(|mut chat_log| {
                            chat_log.push_str(&new_message);
                            chat_log.push('\n');
                        });
                        HANDLED
                    }
                    _ => IGNORED,
                }),
        )
        .into_rows()
        .expand()
        .run()
}
