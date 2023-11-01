use gooey::widgets::{Label, Scroll};
use gooey::Run;

fn main() -> gooey::Result {
    Scroll::new(Label::new(include_str!("../src/widgets/scroll.rs"))).run()
}
