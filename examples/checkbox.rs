use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::checkbox::{Checkable, CheckboxState};
use cushy::Run;

fn main() -> cushy::Result {
    let checkbox_state = Dynamic::new(CheckboxState::Checked);
    let label = checkbox_state.map_each(|state| format!("Check Me! Current: {state:?}"));

    checkbox_state
        .clone()
        .to_checkbox()
        .labelled_by(label)
        .and("Maybe".into_button().on_click(move |_| {
            checkbox_state.set(CheckboxState::Indeterminant);
        }))
        .into_columns()
        .centered()
        .expand()
        .run()
}
