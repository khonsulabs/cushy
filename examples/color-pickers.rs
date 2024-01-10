use cushy::styles::Hsla;
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::color::{HslaPicker, RgbaPicker};
use cushy::widgets::Space;
use cushy::Run;
use figures::units::Lp;
use figures::Size;
use kludgine::Color;

fn main() -> cushy::Result {
    let color = Dynamic::new(Color::RED);
    let color_as_string = color.map_each(|color| format!("{color:?}"));

    let hsl = color.linked(|color| Hsla::from(*color), |hsl| Color::from(*hsl));

    "HSLa Picker"
        .and(HslaPicker::new(hsl).expand())
        .and("RGBa Picker")
        .and(RgbaPicker::new(color.clone()))
        .into_rows()
        .expand()
        .and(
            "Picked Color"
                .and(Space::colored(color).size(Size::squared(Lp::inches(1))))
                .and(color_as_string)
                .into_rows()
                .fit_horizontally(),
        )
        .into_columns()
        .pad()
        .expand()
        .run()
}
