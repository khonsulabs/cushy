use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::progress::Progressable;
use gooey::widgets::slider::Slidable;
use gooey::widgets::Checkbox;
use gooey::Run;
use kludgine::figures::units::Lp;
use kludgine::figures::Size;

fn main() -> gooey::Result {
    let indeterminant = Dynamic::new(false);
    let value = Dynamic::new(0_u8);
    let progress = (&indeterminant, &value)
        .map_each(|(&indeterminant, &value)| (!indeterminant).then_some(value));

    value
        .clone()
        .slider()
        .and(progress.clone().progress_bar())
        .and(Checkbox::new(indeterminant.clone(), "Indeterminant"))
        .into_rows()
        .fit_horizontally()
        .expand()
        .and(value.slider())
        .and(progress.progress_bar())
        .into_columns()
        .pad()
        .size(Size::squared(Lp::inches(3)))
        .centered()
        .expand()
        .run()
}
