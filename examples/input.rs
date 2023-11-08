use gooey::widgets::{Expand, Input};
use gooey::Run;

fn main() -> gooey::Result {
    Expand::new(Input::new("Hello")).run()
}
