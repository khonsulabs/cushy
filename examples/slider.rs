use gooey::animation::{LinearInterpolate, PercentBetween};
use gooey::value::{Dynamic, StringValue};
use gooey::widget::MakeWidget;
use gooey::widgets::slider::Slidable;
use gooey::Run;
use kludgine::figures::units::Lp;
use kludgine::figures::Ranged;

fn main() -> gooey::Result {
    u8_slider()
        .and(enum_slider())
        .into_rows()
        .expand_horizontally()
        .width(..Lp::points(800))
        .centered()
        .expand()
        .run()
}

fn u8_slider() -> impl MakeWidget {
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
}

#[derive(LinearInterpolate, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum SlidableEnum {
    A,
    B,
    C,
}

impl PercentBetween for SlidableEnum {
    fn percent_between(&self, min: &Self, max: &Self) -> gooey::animation::ZeroToOne {
        let min = *min as u8;
        let max = *max as u8;
        let value = *self as u8;
        value.percent_between(&min, &max)
    }
}

impl Ranged for SlidableEnum {
    const MAX: Self = Self::C;
    const MIN: Self = Self::A;
}

fn enum_slider() -> impl MakeWidget {
    let enum_value = Dynamic::new(SlidableEnum::A);
    let enum_text = enum_value.map_each(|value| format!("{value:?}"));
    "Custom Enum"
        .and(enum_value.slider())
        .and(enum_text)
        .into_rows()
}
