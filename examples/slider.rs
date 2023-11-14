use gooey::value::{Dynamic, StringValue};
use gooey::widget::MakeWidget;
use gooey::widgets::slider::Slidable;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let min_text = Dynamic::new(u8::MIN.to_string());
    let min = min_text.map_each(|min| min.parse().unwrap_or(u8::MIN));
    let max_text = Dynamic::new(u8::MAX.to_string());
    let max = max_text.map_each(|max| max.parse().unwrap_or(u8::MAX));
    let value = Dynamic::new(128_u8);
    let value_text = value.map_each(ToString::to_string);

    "Min"
        .and(min_text.into_input())
        .and("Max")
        .and(max_text.into_input())
        .into_columns()
        .centered()
        .and(value.slider_between(min, max))
        .and(value_text.centered())
        .into_rows()
        .expand_horizontally()
        .width(..Lp::points(800))
        .centered()
        .expand()
        .run()
}
