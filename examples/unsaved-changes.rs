use cushy::{
    value::{Dynamic, Source},
    widget::MakeWidget,
    Run,
};
use figures::{units::UPx, Size};

fn main() -> cushy::Result {
    let has_unsaved_changes = Dynamic::new(true);
    let inner_size = Dynamic::new(Size::new(UPx::new(1000), UPx::new(800)));

    "Prevent Closing"
        .into_checkbox(has_unsaved_changes.clone())
        .into_window()
        .on_close_requested(move |()| !has_unsaved_changes.get())
        .inner_size(inner_size)
        .run()
}
