//! This example shows how dynamic values make it easy to communicate state back
//! to widgets from multiple threads.

use std::time::Duration;

use gooey::animation::ZeroToOne;
use gooey::value::{Dynamic, Switchable};
use gooey::widget::MakeWidget;
use gooey::widgets::progress::{Progress, Progressable};
use gooey::Run;

#[derive(Debug, Default, Eq, PartialEq)]
struct Task {
    progress: Dynamic<Progress>,
}

fn main() -> gooey::Result {
    let task = Dynamic::new(None::<Task>);

    task.switcher(|task, dynamic| {
        if let Some(task) = task {
            // A background thread is running, show a progress bar.
            task.progress.clone().progress_bar().make_widget()
        } else {
            // There is no background task. Show a button that will start one.
            "Start"
                .into_button()
                .on_click({
                    let task = dynamic.clone();
                    move |()| {
                        let background_task = Task::default();
                        spawn_background_thread(&background_task.progress, &task);
                        task.set(Some(background_task));
                    }
                })
                .make_widget()
        }
    })
    .contain()
    .centered()
    .run()
}

fn spawn_background_thread(progress: &Dynamic<Progress>, task: &Dynamic<Option<Task>>) {
    let progress = progress.clone();
    let task = task.clone();
    std::thread::spawn(move || background_task(&progress, &task));
}

fn background_task(progress: &Dynamic<Progress>, task: &Dynamic<Option<Task>>) {
    for i in 0_u8..=10 {
        progress.set(Progress::Percent(ZeroToOne::new(f32::from(i) / 10.)));
        std::thread::sleep(Duration::from_millis(100));
    }
    task.set(None);
}
