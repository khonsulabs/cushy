use gooey::kludgine::app::winit::keyboard::{Key, NamedKey};
use gooey::value::{Dynamic, Validations};
use gooey::widget::{Children, MakeWidget, IGNORED};
use gooey::widgets::checkbox::Checkable;
use gooey::widgets::checkbox::CheckboxState;
use gooey::widgets::input::InputValue;
use gooey::widgets::layers::OverlayLayer;
use gooey::Run;

struct State {
    tasks: Dynamic<Vec<String>>,
    task: Dynamic<String>,
}

fn main() -> gooey::Result {
    let state = State {
        tasks: Dynamic::new(vec![
            "Buy milk".to_string(),
            "Buy eggs".to_string(),
            "Buy bread".to_string(),
        ]),
        task: Dynamic::default(),
    };
    let tooltips = OverlayLayer::default();
    let validations = Validations::default();

    let task_field = "Add a new task"
        .and(
            state
                .task
                .clone()
                .into_input()
                .on_key(|key| {
                    if let Key::Named(NamedKey::Enter) = key.logical_key {
                        // state.tasks.lock().push(state.task.get());
                    }
                    IGNORED
                })
                .placeholder("New task...")
                .validation(validations.validate(&state.task, |u: &String| {
                    if u.is_empty() {
                        Err("Task cannot be empty")
                    } else {
                        Ok(())
                    }
                }))
                .hint("Press enter to add task")
                .tooltip(&tooltips, "Enter a task to add it to the list"),
        )
        .into_rows();

    let task_list = state.tasks.map_each({
        move |tasks| {
            tasks
                .iter()
                .map(|task| {
                    let checkbox_state = Dynamic::new(CheckboxState::Checked);
                    let label = Dynamic::new(task.clone());
                    checkbox_state
                        .clone()
                        .into_checkbox(label)
                        .align_left()
                        .make_widget()
                })
                .collect::<Children>()
        }
    });

    let ui = task_field
        .and(task_list.into_rows().vertical_scroll())
        .into_rows()
        .expand()
        .contain()
        .pad();

    ui.and(tooltips).into_layers().run()
}
