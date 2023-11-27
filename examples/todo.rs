use gooey::value::Dynamic;
use gooey::widget::{Children, MakeWidget, WidgetInstance};
use gooey::widgets::input::InputValue;
use gooey::widgets::{Button, Label, Scroll, Stack};
use gooey::Run;

fn main() -> gooey::Result {
    let tasks = Dynamic::default();
    let children = Dynamic::default();
    tasks.for_each(children.with_clone(|children| {
        move |tasks: &Vec<Task>| {
            update_task_widgets(tasks, &mut children.lock());
        }
    }));

    let task_text = Dynamic::default();
    let valid = task_text.map_each(|text: &String| !text.is_empty());
    let task_form = Stack::columns(
        task_text.clone().into_input().expand().and(
            Button::new("Add Task")
                .on_click(move |_| {
                    tasks.lock().push(Task::new(task_text.take()));
                })
                .into_default()
                .with_enabled(valid),
        ),
    );
    let tasks = Scroll::vertical(Stack::rows(children));

    Stack::rows(task_form.and(tasks)).expand().run()
}

struct Task {
    widget: WidgetInstance,
}

impl Task {
    pub fn new(text: String) -> Self {
        let widget = Label::new(Dynamic::new(text)).align_left().make_widget();

        Self { widget }
    }
}

fn update_task_widgets(tasks: &[Task], children: &mut Children) {
    'tasks: for (index, task) in tasks.iter().enumerate() {
        for child in index..children.len() {
            if children[child].id() == task.widget.id() {
                if child != index {
                    children.swap(child, index);
                }
                continue 'tasks;
            }
        }

        children.insert(index, task.widget.clone());
    }

    children.truncate(tasks.len());
}
