use cushy::styles::Hsl;
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::color::HslPicker;
use cushy::widgets::Space;
use cushy::Run;
use figures::units::Lp;
use figures::Size;
use kludgine::Color;

fn main() -> cushy::Result {
    let hsl = Dynamic::new(Hsl::from(Color::RED));
    let color = hsl.map_each_cloned(Color::from);
    "Picker"
        .and(HslPicker::new(hsl).expand())
        .into_rows()
        .expand()
        .and(
            "Picked Color"
                .and(Space::colored(color).size(Size::squared(Lp::inches(1))))
                .into_rows(),
        )
        .into_columns()
        .pad()
        .expand()
        .run()
}
