use std::path::PathBuf;
use std::thread;

use rfd::{FileDialog, MessageDialog, MessageDialogResult};

use super::{
    coalesce_empty, FilePicker, MessageBox, MessageButtons, MessageButtonsKind, MessageLevel, Mode,
    OpenMessageBox, PickFile,
};
use crate::window::WindowHandle;
use crate::App;

impl MessageButtons {
    fn as_rfd_buttons(&self) -> rfd::MessageButtons {
        let cancel_is_custom = self
            .cancel
            .as_ref()
            .map_or(false, |b| !b.caption.is_empty());
        match self.kind {
            MessageButtonsKind::YesNo => {
                let negative = self.negative.as_ref().expect("no button");
                if cancel_is_custom
                    || !self.affirmative.caption.is_empty()
                    || !negative.caption.is_empty()
                {
                    if let Some(cancel) = &self.cancel {
                        rfd::MessageButtons::YesNoCancelCustom(
                            coalesce_empty(&self.affirmative.caption, "Yes").to_string(),
                            coalesce_empty(&negative.caption, "No").to_string(),
                            coalesce_empty(&cancel.caption, "Yes").to_string(),
                        )
                    } else {
                        rfd::MessageButtons::OkCancelCustom(
                            coalesce_empty(&self.affirmative.caption, "Yes").to_string(),
                            coalesce_empty(&negative.caption, "No").to_string(),
                        )
                    }
                } else if self.cancel.is_some() {
                    rfd::MessageButtons::YesNoCancel
                } else {
                    rfd::MessageButtons::YesNo
                }
            }
            MessageButtonsKind::OkCancel => {
                if let Some(cancel) = &self.cancel {
                    if !self.affirmative.caption.is_empty() || !cancel.caption.is_empty() {
                        rfd::MessageButtons::OkCancelCustom(
                            coalesce_empty(&self.affirmative.caption, "OK").to_string(),
                            coalesce_empty(&cancel.caption, "Cancel").to_string(),
                        )
                    } else {
                        rfd::MessageButtons::OkCancel
                    }
                } else if !self.affirmative.caption.is_empty() {
                    rfd::MessageButtons::OkCustom(self.affirmative.caption.clone())
                } else {
                    rfd::MessageButtons::Ok
                }
            }
        }
    }
}

impl From<MessageLevel> for rfd::MessageLevel {
    fn from(value: MessageLevel) -> Self {
        match value {
            MessageLevel::Error => rfd::MessageLevel::Error,
            MessageLevel::Warning => rfd::MessageLevel::Warning,
            MessageLevel::Info => rfd::MessageLevel::Info,
        }
    }
}

impl OpenMessageBox for WindowHandle {
    fn open_message_box(&self, message: &MessageBox) {
        let message = message.clone();
        self.execute(move |context| {
            // Get access to the winit handle from the window thread.
            let winit = context.winit().cloned();
            // We can't utilize the window handle outside of the main thread
            // with winit, so we now need to move execution to the event loop
            // thread.
            let Some(app) = context.app().cloned() else {
                return;
            };
            app.execute(move |_app| {
                let mut dialog = MessageDialog::new()
                    .set_title(message.title)
                    .set_buttons(message.buttons.as_rfd_buttons())
                    .set_description(message.description)
                    .set_level(message.level.into());
                if let Some(winit) = winit {
                    dialog = dialog.set_parent(&winit);
                }
                thread::spawn(move || {
                    handle_message_result(&dialog.show(), &message.buttons);
                });
            });
        });
    }
}

impl OpenMessageBox for App {
    fn open_message_box(&self, message: &MessageBox) {
        let shutdown_guard = self.prevent_shutdown();
        let message = message.clone();
        self.execute(move |_app| {
            let dialog = MessageDialog::new()
                .set_title(message.title)
                .set_buttons(message.buttons.as_rfd_buttons())
                .set_description(message.description)
                .set_level(message.level.into());
            thread::spawn(move || {
                handle_message_result(&dialog.show(), &message.buttons);
                drop(shutdown_guard);
            });
        });
    }
}

fn handle_message_result(result: &MessageDialogResult, buttons: &MessageButtons) {
    match result {
        MessageDialogResult::Ok | MessageDialogResult::Yes => {
            buttons.affirmative.callback.invoke();
        }
        MessageDialogResult::No => {
            buttons
                .negative
                .as_ref()
                .expect("no button")
                .callback
                .invoke();
        }
        MessageDialogResult::Cancel => {
            if matches!(buttons.kind, MessageButtonsKind::YesNo) && buttons.cancel.is_none() {
                // Cancel means No in this situation.
                buttons
                    .negative
                    .as_ref()
                    .expect("no button")
                    .callback
                    .invoke();
            } else {
                buttons
                    .cancel
                    .as_ref()
                    .expect("cancel button")
                    .callback
                    .invoke();
            }
        }
        MessageDialogResult::Custom(caption) => {
            let (default_affirmative, default_negative) = match buttons.kind {
                MessageButtonsKind::YesNo => ("Yes", Some("No")),
                MessageButtonsKind::OkCancel => ("OK", None),
            };

            if coalesce_empty(&buttons.affirmative.caption, default_affirmative) == caption {
                buttons.affirmative.callback.invoke();
            } else if let Some(negative) = buttons.negative.as_ref().filter(|negative| {
                &negative.caption == caption
                    || default_negative
                        .map_or(false, |default_negative| default_negative == caption)
            }) {
                negative.callback.invoke();
            } else if let Some(cancel) = buttons
                .cancel
                .as_ref()
                .filter(|cancel| coalesce_empty(&cancel.caption, "Cancel") == caption)
            {
                cancel.callback.invoke();
            } else {
                unreachable!("no matching button")
            }
        }
    }
}

fn create_file_dialog(picker: FilePicker) -> FileDialog {
    let mut dialog = FileDialog::new();

    if !picker.title.is_empty() {
        dialog = dialog.set_title(picker.title);
    }

    if let Some(directory) = picker.directory {
        dialog = dialog.set_directory(directory);
    }

    if !picker.file_name.is_empty() {
        dialog = dialog.set_file_name(picker.file_name);
    }

    for ty in picker.types {
        dialog = dialog.add_filter(ty.name, &ty.extensions);
    }

    if let Some(can_create) = picker.can_create_directories {
        dialog = dialog.set_can_create_directories(can_create);
    }
    dialog
}

fn show_picker_in_window(window: &WindowHandle, picker: &FilePicker, mode: Mode) {
    let picker = picker.clone();
    window.execute(move |context| {
        // Get access to the winit handle from the window thread.
        let winit = context.winit().cloned();
        // We can't utilize the window handle outside of the main thread
        // with winit, so we now need to move execution to the event loop
        // thread.
        let Some(app) = context.app().cloned() else {
            return;
        };
        app.execute(move |_app| {
            let mut dialog = create_file_dialog(picker);

            if let Some(winit) = winit {
                dialog = dialog.set_parent(&winit);
            }

            // Now that we've set the parent, we can move this to its own
            // blocking thread to be shown.
            thread::spawn(move || match mode {
                Mode::File(on_dismiss) => on_dismiss.invoke(dialog.pick_file()),
                Mode::SaveFile(on_dismiss) => on_dismiss.invoke(dialog.save_file()),
                Mode::Files(on_dismiss) => on_dismiss.invoke(dialog.pick_files()),
                Mode::Folder(on_dismiss) => on_dismiss.invoke(dialog.pick_folder()),
                Mode::Folders(on_dismiss) => on_dismiss.invoke(dialog.pick_folders()),
            });
        });
    });
}

impl PickFile for WindowHandle {
    fn pick_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_window(self, picker, Mode::file(callback));
    }

    fn save_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_window(self, picker, Mode::save_file(callback));
    }

    fn pick_files<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        show_picker_in_window(self, picker, Mode::files(callback));
    }

    fn pick_folder<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_window(self, picker, Mode::folder(callback));
    }

    fn pick_folders<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        show_picker_in_window(self, picker, Mode::folders(callback));
    }
}

fn show_picker_in_app(app: &App, picker: &FilePicker, mode: Mode) {
    let picker = picker.clone();
    app.execute(move |_| {
        let dialog = create_file_dialog(picker);

        // Now that we've set the parent, we can move this to its own
        // blocking thread to be shown.
        thread::spawn(move || match mode {
            Mode::File(on_dismiss) => on_dismiss.invoke(dialog.pick_file()),
            Mode::SaveFile(on_dismiss) => on_dismiss.invoke(dialog.save_file()),
            Mode::Files(on_dismiss) => on_dismiss.invoke(dialog.pick_files()),
            Mode::Folder(on_dismiss) => on_dismiss.invoke(dialog.pick_folder()),
            Mode::Folders(on_dismiss) => on_dismiss.invoke(dialog.pick_folders()),
        });
    });
}

impl PickFile for App {
    fn pick_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_app(self, picker, Mode::file(callback));
    }

    fn save_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_app(self, picker, Mode::save_file(callback));
    }

    fn pick_files<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        show_picker_in_app(self, picker, Mode::files(callback));
    }

    fn pick_folder<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        show_picker_in_app(self, picker, Mode::folder(callback));
    }

    fn pick_folders<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        show_picker_in_app(self, picker, Mode::folders(callback));
    }
}
