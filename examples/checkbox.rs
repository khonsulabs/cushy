use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::checkbox::CheckboxState;
use gooey::widgets::Checkbox;
use gooey::Run;

fn main() -> gooey::Result {
    let checkbox_state = Dynamic::new(CheckboxState::Checked);
    let label = checkbox_state.map_each(|state| format!("Check Me! Current: {state:?}"));

    Checkbox::new(checkbox_state.clone(), label)
        .and("Maybe".into_button().on_click(move |()| {
            checkbox_state.update(CheckboxState::Indeterminant);
        }))
        .into_columns()
        .centered()
        .expand()
        .run()
}
