use gooey::widgets::Input;
use gooey::Run;

fn main() -> gooey::Result {
    Input::new("Hello").run()
}
