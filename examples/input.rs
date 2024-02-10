use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::input::{InputValue, MaskedString};
use cushy::Run;
use figures::units::Px;

fn main() -> cushy::Result {
    let contents = Dynamic::from("Hello World");
    let password = Dynamic::new(MaskedString::default());

    "Text Input Field:"
        .and(contents.into_input())
        .and("Masked Input Field:")
        .and(password.into_input())
        .into_rows()
        .width(Px::new(100)..Px::new(800))
        .scroll()
        .centered()
        .run()
}
