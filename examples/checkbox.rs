use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::checkbox::{Checkable, CheckboxState};
use cushy::Run;

fn main() -> cushy::Result {
    let checkbox_state = Dynamic::new(CheckboxState::Checked);
    let label = checkbox_state.map_each(|state| format!("Check Me! Current: {state:?}"));

    checkbox_state
        .clone()
        .into_checkbox(label)
        .and("Maybe".into_button().on_click(move |()| {
            checkbox_state.set(CheckboxState::Indeterminant);
        }))
        .into_columns()
        .centered()
        .expand()
        .run()
}
