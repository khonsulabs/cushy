use cushy::figures::units::Px;
use cushy::styles::components::HorizontalAlignment;
use cushy::styles::HorizontalAlign;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::input::{InputValue, MaskedString};
use cushy::Run;

fn main() -> cushy::Result {
    let contents = Dynamic::from("Hello World");
    let password = Dynamic::new(MaskedString::default());

    "Text Input Field:"
        .and(contents.into_input())
        .and("Masked Input Field:")
        .and(password.into_input())
        .into_rows()
        .width(Px::new(100)..Px::new(800))
        .with_local(&HorizontalAlignment, HorizontalAlign::Center)
        .expand_horizontally()
        .pad()
        .vertical_scroll()
        .centered()
        .run()
}
