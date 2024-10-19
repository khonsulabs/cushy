use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::Run;

fn main() -> cushy::Result {
    let has_unsaved_changes = Dynamic::new(true);

    "Prevent Closing"
        .into_checkbox(has_unsaved_changes.clone())
        .into_window()
        .on_close_requested(move |()| !has_unsaved_changes.get())
        .run()
}
