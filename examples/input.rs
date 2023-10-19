use gooey::widget::Widget;
use gooey::widgets::Input;
use gooey::EventLoopError;

fn main() -> Result<(), EventLoopError> {
    Input::new("Hello").run()
}
