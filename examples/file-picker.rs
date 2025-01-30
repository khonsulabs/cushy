use std::path::PathBuf;

use cushy::dialog::{FilePicker, PickFile};
use cushy::reactive::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget};
use cushy::widgets::button::ButtonClick;
use cushy::widgets::checkbox::Checkable;
use cushy::widgets::layers::Modal;
use cushy::window::{PendingWindow, WindowHandle};
use cushy::{App, Open};

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {
    let modal = Modal::new();
    let pending = PendingWindow::default();
    let window = pending.handle();
    let chosen_paths = Dynamic::<Vec<PathBuf>>::default();
    let picker_mode = Dynamic::default();
    let pick_multiple = Dynamic::new(false);
    let results = chosen_paths.map_each(|paths| {
        if paths.is_empty() {
            "None".make_widget()
        } else {
            paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .into_rows()
                .make_widget()
        }
    });

    pending
        .with_root(
            picker_mode
                .new_radio(PickerMode::SaveFile)
                .labelled_by("Save File")
                .and(
                    picker_mode
                        .new_radio(PickerMode::PickFile)
                        .labelled_by("Pick File"),
                )
                .and(
                    picker_mode
                        .new_radio(PickerMode::PickFolder)
                        .labelled_by("Pick Folder"),
                )
                .into_columns()
                .and(
                    pick_multiple
                        .to_checkbox()
                        .labelled_by("Select Multiple")
                        .with_enabled(
                            picker_mode.map_each(|kind| !matches!(kind, PickerMode::SaveFile)),
                        ),
                )
                .and(picker_buttons(
                    &picker_mode,
                    &pick_multiple,
                    app,
                    &window,
                    &modal,
                    &chosen_paths,
                ))
                .and("Result:")
                .and(results)
                .into_rows()
                .centered()
                .vertical_scroll()
                .expand()
                .and(modal)
                .into_layers(),
        )
        .open(app)?;
    Ok(())
}

#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
enum PickerMode {
    #[default]
    SaveFile,
    PickFile,
    PickFolder,
}

fn file_picker() -> FilePicker {
    FilePicker::new()
        .with_title("Pick a Rust source file")
        .with_types([("Rust Source", ["rs"])])
}

fn display_single_result(
    chosen_paths: &Dynamic<Vec<PathBuf>>,
) -> impl FnMut(Option<PathBuf>) + Send + 'static {
    let chosen_paths = chosen_paths.clone();
    move |path| {
        chosen_paths.set(path.into_iter().collect());
    }
}

fn display_multiple_results(
    chosen_paths: &Dynamic<Vec<PathBuf>>,
) -> impl FnMut(Option<Vec<PathBuf>>) + Send + 'static {
    let chosen_paths = chosen_paths.clone();
    move |path| {
        chosen_paths.set(path.into_iter().flatten().collect());
    }
}

fn picker_buttons(
    mode: &Dynamic<PickerMode>,
    pick_multiple: &Dynamic<bool>,
    app: &App,
    window: &WindowHandle,
    modal: &Modal,
    chosen_paths: &Dynamic<Vec<PathBuf>>,
) -> impl MakeWidget {
    "Show in Modal layer"
        .into_button()
        .on_click(show_picker_in(modal, chosen_paths, mode, pick_multiple))
        .and("Show above window".into_button().on_click(show_picker_in(
            window,
            chosen_paths,
            mode,
            pick_multiple,
        )))
        .and("Show in app".into_button().on_click(show_picker_in(
            app,
            chosen_paths,
            mode,
            pick_multiple,
        )))
        .into_rows()
}

fn show_picker_in(
    target: &(impl PickFile + Clone + Send + 'static),
    chosen_paths: &Dynamic<Vec<PathBuf>>,
    mode: &Dynamic<PickerMode>,
    pick_multiple: &Dynamic<bool>,
) -> impl FnMut(Option<ButtonClick>) + Send + 'static {
    let target = target.clone();
    let chosen_paths = chosen_paths.clone();
    let mode = mode.clone();
    let pick_multiple = pick_multiple.clone();
    move |_| {
        match mode.get() {
            PickerMode::SaveFile => {
                file_picker().save_file(&target, display_single_result(&chosen_paths))
            }
            PickerMode::PickFile if pick_multiple.get() => {
                file_picker().pick_files(&target, display_multiple_results(&chosen_paths))
            }
            PickerMode::PickFile => {
                file_picker().pick_file(&target, display_single_result(&chosen_paths))
            }
            PickerMode::PickFolder if pick_multiple.get() => {
                file_picker().pick_folders(&target, display_multiple_results(&chosen_paths))
            }
            PickerMode::PickFolder => {
                file_picker().pick_folder(&target, display_single_result(&chosen_paths))
            }
        };
    }
}
