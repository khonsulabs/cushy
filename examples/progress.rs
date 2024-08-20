use cushy::figures::units::Lp;
use cushy::figures::Size;
use cushy::value::{Dynamic, MapEach};
use cushy::widget::MakeWidget;
use cushy::widgets::progress::Progressable;
use cushy::widgets::slider::Slidable;
use cushy::Run;

fn main() -> cushy::Result {
    let indeterminant = Dynamic::new(false);
    let value = Dynamic::new(0_u8);
    let progress = (&indeterminant, &value)
        .map_each(|(&indeterminant, &value)| (!indeterminant).then_some(value));

    value
        .clone()
        .slider()
        .and(
            progress
                .clone()
                .progress_bar()
                .expand()
                .and(progress.clone().progress_bar().spinner())
                .into_columns(),
        )
        .and("Indeterminant".into_checkbox(indeterminant))
        .into_rows()
        .fit_horizontally()
        .expand()
        .and(value.slider())
        .and(progress.progress_bar())
        .into_columns()
        .pad()
        .size(Size::squared(Lp::inches(3)))
        .centered()
        .run()
}
