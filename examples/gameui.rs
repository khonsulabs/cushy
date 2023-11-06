use gooey::children;
use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, HANDLED, IGNORED};
use gooey::widgets::{Canvas, Expand, Input, Label, Scroll, Stack};
use kludgine::app::winit::event::ElementState;
use kludgine::app::winit::keyboard::Key;
use kludgine::figures::{Point, Rect};
use kludgine::shapes::Shape;
use kludgine::Color;

fn main() -> gooey::Result {
    let chat_log = Dynamic::new("Chat log goes here.\n".repeat(100));
    let chat_message = Dynamic::new(String::new());

    let input = Input::new(chat_message.clone())
        .on_key({
            let chat_log = chat_log.clone();
            move |input| match (input.state, input.logical_key) {
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
        })
        .make_widget();
    let input_id = input.id();

    Expand::new(Stack::rows(children![
        Expand::new(Stack::columns(children![
            Expand::new(Scroll::vertical(Label::new(chat_log))),
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
        input.clone(),
    ]))
    .with_next_focus(input_id)
    .run()
}
