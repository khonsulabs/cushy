use gooey::dynamic::Dynamic;
use gooey::widget::Widget;
use gooey::widgets::{Button, Input};
use gooey::EventLoopError;

fn main() -> Result<(), EventLoopError> {
    Input::new("Hello").run()
}
