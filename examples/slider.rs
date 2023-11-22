use gooey::animation::{LinearInterpolate, PercentBetween};
use gooey::value::{Dynamic, ForEach};
use gooey::widget::MakeWidget;
use gooey::widgets::checkbox::Checkable;
use gooey::widgets::input::InputValue;
use gooey::widgets::slider::Slidable;
use gooey::Run;
use kludgine::figures::units::Lp;
use kludgine::figures::Ranged;

fn main() -> gooey::Result {
    let enabled = Dynamic::new(true);
    u8_slider()
        .and(u8_range_slider())
        .and(enum_slider())
        .into_rows()
        .with_enabled(enabled.clone())
        .and(enabled.into_checkbox("Enabled"))
        .into_rows()
        .expand_horizontally()
        .contain()
        .width(..Lp::points(800))
        .centered()
        .expand()
        .run()
}

fn u8_slider() -> impl MakeWidget {
    let min = Dynamic::new(u8::MIN);
    let min_text = min.linked_string();
    let max = Dynamic::new(u8::MAX);
    let max_text = max.linked_string();
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

fn u8_range_slider() -> impl MakeWidget {
    let range = Dynamic::new(42..=127);
    let start = range.map_each_unique(|range| *range.start());
    let end = range.map_each_unique(|range| *range.end());
    (&start, &end).for_each({
        let range = range.clone();
        move |(start, end)| {
            let _result = range.try_update(*start..=*end);
        }
    });

    let min = Dynamic::new(u8::MIN);
    let min_text = min.linked_string();
    let start_text = start.linked_string();
    let end_text = end.linked_string();
    let max = Dynamic::new(u8::MAX);
    let max_text = max.linked_string();
    let value_text = range.map_each(|r| format!("{}..={}", r.start(), r.end()));

    "Min"
        .and(min_text.into_input())
        .and("Start")
        .and(start_text.into_input())
        .and("End")
        .and(end_text.into_input())
        .and("Max")
        .and(max_text.into_input())
        .into_columns()
        .centered()
        .and(range.slider_between(min, max))
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
