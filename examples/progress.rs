use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::progress::Progressable;
use gooey::widgets::slider::Slidable;
use gooey::widgets::Checkbox;
use gooey::Run;

fn main() -> gooey::Result {
    let indeterminant = Dynamic::new(false);
    let value = Dynamic::new(0_u8);
    let progress = (&indeterminant, &value)
        .map_each(|(&indeterminant, &value)| (!indeterminant).then_some(value));

    value
        .slider()
        .and(progress.progress_bar())
        .and(Checkbox::new(indeterminant.clone(), "Indeterminant"))
        .into_rows()
        .centered()
        .expand()
        .run()
}
