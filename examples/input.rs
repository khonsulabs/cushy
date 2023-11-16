use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::input::{InputValue, MaskedString};
use gooey::Run;
use kludgine::figures::units::Px;

fn main() -> gooey::Result {
    let contents = Dynamic::from("Hello World");
    let password = Dynamic::new(MaskedString::default());

    "Text Input Field:"
        .and(contents.into_input())
        .and("Masked Input Field:")
        .and(password.into_input())
        .into_rows()
        .width(Px(100)..Px(800))
        .scroll()
        .centered()
        .expand()
        .run()
}
