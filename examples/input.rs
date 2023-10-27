use gooey::widgets::Input;
use gooey::{EventLoopError, Run};

fn main() -> Result<(), EventLoopError> {
    Input::new("Hello").run()
}
