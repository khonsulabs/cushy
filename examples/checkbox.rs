use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::checkbox::{Checkable, CheckboxState};
use gooey::Run;

fn main() -> gooey::Result {
    let checkbox_state = Dynamic::new(CheckboxState::Checked);
    let label = checkbox_state.map_each(|state| format!("Check Me! Current: {state:?}"));

    checkbox_state
        .clone()
        .into_checkbox(label)
        .and("Maybe".into_button().on_click(move |()| {
            checkbox_state.update(CheckboxState::Indeterminant);
        }))
        .into_columns()
        .centered()
        .expand()
        .run()
}
