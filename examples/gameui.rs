use gooey::value::Dynamic;
use gooey::widget::{HANDLED, IGNORED};
use gooey::widgets::{Canvas, Expand, Input, Label, Scroll, Stack};
use gooey::{widgets, Run};
use kludgine::app::winit::event::ElementState;
use kludgine::app::winit::keyboard::Key;
use kludgine::figures::{Point, Rect};
use kludgine::shapes::Shape;
use kludgine::Color;

fn main() -> gooey::Result {
    let chat_log = Dynamic::new("Chat log goes here.\n".repeat(100));
    let chat_message = Dynamic::new(String::new());

    Expand::new(Stack::rows(widgets![
        Expand::new(Stack::columns(widgets![
            Expand::new(Scroll::vertical(Label::new(chat_log.clone()))),
            Expand::weighted(
                2,
                Canvas::new(|context| {
                    let entire_canvas = Rect::from(context.graphics.size());
                    context.graphics.draw_shape(
                        &Shape::filled_rect(entire_canvas, Color::RED),
                        Point::default(),
                        None,
                        None,
                    );
                })
            )
        ])),
        Input::new(chat_message.clone()).on_key(move |input| {
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
        }),
    ]))
    .run()
}
