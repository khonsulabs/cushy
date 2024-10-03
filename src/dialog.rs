//! Modal dialogs such as message boxes and file pickers.

use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use figures::units::Lp;
use parking_lot::Mutex;

use crate::styles::components::{PrimaryColor, WidgetBackground};
use crate::styles::DynamicComponent;
use crate::value::{Destination, Dynamic, Source};
use crate::widget::{MakeWidget, OnceCallback, SharedCallback, WidgetList};
use crate::widgets::button::{ButtonKind, ClickCounter};
use crate::widgets::input::InputValue;
use crate::widgets::layers::Modal;
use crate::widgets::Custom;
use crate::ModifiersExt;

#[cfg(feature = "native-dialogs")]
mod native;

#[derive(Clone, Debug)]
struct MessageButtons {
    kind: MessageButtonsKind,
    affirmative: MessageButton,
    negative: Option<MessageButton>,
    cancel: Option<MessageButton>,
}

#[derive(Clone, Debug, Copy)]
enum MessageButtonsKind {
    YesNo,
    OkCancel,
}

/// A button in a [`MessageBox`].
///
/// This type implements [`From`] for several types:
///
/// - `String`, `&str`: A button with the string's contents as the caption that
///   dismisses the message box.
/// - `FnMut()` implementors: A button with the default caption given its
///   context that invokes the closure when chosen.
///
/// To create a button with a custom caption that invokes a closure when chosen,
/// use [`MessageButton::custom`].
#[derive(Clone, Debug, Default)]
pub struct MessageButton {
    callback: OptionalCallback,
    caption: String,
}

impl MessageButton {
    /// Returns a button with a custom caption that invokes `on_click` when
    /// selected.
    pub fn custom<F>(caption: impl Into<String>, mut on_click: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        Self {
            callback: OptionalCallback(Some(SharedCallback::new(move |()| on_click()))),
            caption: caption.into(),
        }
    }
}

impl From<String> for MessageButton {
    fn from(value: String) -> Self {
        Self {
            callback: OptionalCallback::default(),
            caption: value,
        }
    }
}

impl From<&'_ String> for MessageButton {
    fn from(value: &'_ String) -> Self {
        Self::from(value.clone())
    }
}

impl From<&'_ str> for MessageButton {
    fn from(value: &'_ str) -> Self {
        Self::from(value.to_string())
    }
}

impl<F> From<F> for MessageButton
where
    F: FnMut() + Send + 'static,
{
    fn from(mut value: F) -> Self {
        Self {
            callback: OptionalCallback(Some(SharedCallback::new(move |()| value()))),
            caption: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct OptionalCallback(Option<SharedCallback>);

impl OptionalCallback {
    fn invoke(&self) {
        if let Some(callback) = &self.0 {
            callback.invoke(());
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq, Copy, Debug)]
enum MessageLevel {
    Error,
    Warning,
    #[default]
    Info,
}

/// A marker indicating a [`MessageBoxBuilder`] does not have a preference
/// between a yes/no/cancel or an ok/cancel configuration.
pub enum Undecided {}

/// Specializes a [`MessageBoxBuilder`] for an Ok/Cancel dialog.
pub enum OkCancel {}

/// Specializes a [`MessageBoxBuilder`] for a Yes/No dialog.
pub enum YesNoCancel {}

/// A builder for a [`MessageBox`].
#[must_use]
pub struct MessageBoxBuilder<Kind>(MessageBox, PhantomData<Kind>);

impl<Kind> MessageBoxBuilder<Kind> {
    fn new(message: MessageBox) -> MessageBoxBuilder<Kind> {
        Self(message, PhantomData)
    }

    /// Sets the explanation text and returns self.
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.0.description = explanation.into();
        self
    }

    /// Displays this message as a warning.
    ///
    /// When using native dialogs, not all platforms support this stylization.
    pub fn warning(mut self) -> Self {
        self.0.level = MessageLevel::Warning;
        self
    }

    /// Displays this message as an error.
    ///
    /// When using native dialogs, not all platforms support this stylization.
    pub fn error(mut self) -> Self {
        self.0.level = MessageLevel::Error;
        self
    }

    /// Adds a cancel button and returns self.
    pub fn with_cancel(mut self, cancel: impl Into<MessageButton>) -> Self {
        self.0.buttons.cancel = Some(cancel.into());
        self
    }

    /// Returns the completed message box.
    #[must_use]
    pub fn finish(self) -> MessageBox {
        self.0
    }
}

impl MessageBoxBuilder<Undecided> {
    /// Sets the yes button and returns self.
    pub fn with_yes(
        Self(mut message, _): Self,
        yes: impl Into<MessageButton>,
    ) -> MessageBoxBuilder<YesNoCancel> {
        message.buttons.kind = MessageButtonsKind::YesNo;
        message.buttons.affirmative = yes.into();
        MessageBoxBuilder(message, PhantomData)
    }

    /// Sets the ok button and returns self.
    pub fn with_ok(
        Self(mut message, _): Self,
        ok: impl Into<MessageButton>,
    ) -> MessageBoxBuilder<OkCancel> {
        message.buttons.affirmative = ok.into();
        MessageBoxBuilder(message, PhantomData)
    }
}

impl MessageBoxBuilder<YesNoCancel> {
    /// Sets the no button and returns self.
    pub fn with_no(mut self, no: impl Into<MessageButton>) -> Self {
        self.0.buttons.negative = Some(no.into());
        self
    }
}

impl MessageBoxBuilder<OkCancel> {}

/// A dialog that displays a message.
#[derive(Debug, Clone)]
pub struct MessageBox {
    level: MessageLevel,
    title: String,
    description: String,
    buttons: MessageButtons,
}

impl MessageBox {
    fn new(title: String, kind: MessageButtonsKind) -> Self {
        Self {
            level: MessageLevel::default(),
            title,
            description: String::default(),
            buttons: MessageButtons {
                kind,
                affirmative: MessageButton::default(),
                negative: None,
                cancel: None,
            },
        }
    }

    /// Returns a builder for a dialog displaying `message`.
    pub fn build(message: impl Into<String>) -> MessageBoxBuilder<Undecided> {
        MessageBoxBuilder::new(Self::new(message.into(), MessageButtonsKind::OkCancel))
    }

    /// Returns a dialog displaying `message` with an `OK` button that dismisses
    /// the dialog.
    #[must_use]
    pub fn message(message: impl Into<String>) -> Self {
        Self::build(message).finish()
    }

    /// Sets the explanation text and returns self.
    #[must_use]
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.description = explanation.into();
        self
    }

    /// Displays this message as a warning.
    ///
    /// When using native dialogs, not all platforms support this stylization.
    #[must_use]
    pub fn warning(mut self) -> Self {
        self.level = MessageLevel::Warning;
        self
    }

    /// Displays this message as an error.
    ///
    /// When using native dialogs, not all platforms support this stylization.
    #[must_use]
    pub fn error(mut self) -> Self {
        self.level = MessageLevel::Error;
        self
    }

    /// Adds a cancel button and returns self.
    #[must_use]
    pub fn with_cancel(mut self, cancel: impl Into<MessageButton>) -> Self {
        self.buttons.cancel = Some(cancel.into());
        self
    }

    /// Opens this dialog in the given target.
    ///
    /// A target can be a [`Modal`] layer, a [`WindowHandle`], or an [`App`].
    pub fn open(&self, open_in: &impl OpenMessageBox) {
        open_in.open_message_box(self);
    }
}

/// A type that can open a [`MessageBox`] as a modal dialog.
pub trait OpenMessageBox {
    /// Opens `message` as a modal dialog.
    fn open_message_box(&self, message: &MessageBox);
}

fn coalesce_empty<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.is_empty() {
        s2
    } else {
        s1
    }
}

impl OpenMessageBox for Modal {
    fn open_message_box(&self, message: &MessageBox) {
        let dialog = self.build_dialog(
            message
                .title
                .as_str()
                .h5()
                .and(message.description.as_str())
                .into_rows(),
        );
        let (default_affirmative, default_negative) = match &message.buttons.kind {
            MessageButtonsKind::OkCancel => ("OK", None),
            MessageButtonsKind::YesNo => ("Yes", Some("No")),
        };
        let on_ok = message.buttons.affirmative.callback.clone();
        let mut dialog = dialog.with_default_button(
            coalesce_empty(&message.buttons.affirmative.caption, default_affirmative),
            move || on_ok.invoke(),
        );
        if let (Some(negative), Some(default_negative)) =
            (&message.buttons.negative, default_negative)
        {
            let on_negative = negative.callback.clone();
            dialog = dialog.with_button(
                coalesce_empty(&negative.caption, default_negative),
                move || {
                    on_negative.invoke();
                },
            );
        }

        if let Some(cancel) = &message.buttons.cancel {
            let on_cancel = cancel.callback.clone();
            dialog
                .with_cancel_button(coalesce_empty(&cancel.caption, "Cancel"), move || {
                    on_cancel.invoke();
                })
                .show();
        } else {
            dialog.show();
        }
    }
}

/// A dialog that can pick one or more files or directories.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FilePicker {
    types: Vec<FileType>,
    directory: Option<PathBuf>,
    file_name: String,
    title: String,
    can_create_directories: Option<bool>,
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl FilePicker {
    /// Returns a new file picker dialog.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            types: Vec::new(),
            directory: None,
            file_name: String::new(),
            title: String::new(),
            can_create_directories: None,
        }
    }

    /// Sets the title of the dialog and returns self.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the initial file name for the dialog and returns self.
    #[must_use]
    pub fn with_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = file_name.into();
        self
    }

    /// Enables directory creation within the dialog and returns self.
    #[must_use]
    pub fn allowing_directory_creation(mut self, allowed: bool) -> Self {
        self.can_create_directories = Some(allowed);
        self
    }

    /// Adds the list of type filters to the dialog and returns self.
    ///
    /// These type filters are used for the dialog to only show related files
    /// and restrict what extensions are allowed to be picked.
    #[must_use]
    pub fn with_types<Type>(mut self, types: impl IntoIterator<Item = Type>) -> Self
    where
        Type: Into<FileType>,
    {
        self.types = types.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the initial directory for the dialog and returns self.
    #[must_use]
    pub fn with_initial_directory(mut self, directory: impl AsRef<Path>) -> Self {
        self.directory = Some(directory.as_ref().to_path_buf());
        self
    }

    /// Shows a picker that selects a single file and invokes `on_dismiss` when
    /// the dialog is dismissed.
    pub fn pick_file<Callback>(&self, pick_in: &impl PickFile, on_dismiss: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        pick_in.pick_file(self, on_dismiss);
    }

    /// Shows a picker that creates a new file and invokes `on_dismiss` when the
    /// dialog is dismissed.
    pub fn save_file<Callback>(&self, pick_in: &impl PickFile, on_dismiss: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        pick_in.save_file(self, on_dismiss);
    }

    /// Shows a picker that selects one or more files and invokes `on_dismiss`
    /// when the dialog is dismissed.
    pub fn pick_files<Callback>(&self, pick_in: &impl PickFile, on_dismiss: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        pick_in.pick_files(self, on_dismiss);
    }

    /// Shows a picker that selects a single folder/directory and invokes
    /// `on_dismiss` when the dialog is dismissed.
    pub fn pick_folder<Callback>(&self, pick_in: &impl PickFile, on_dismiss: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        pick_in.pick_folder(self, on_dismiss);
    }

    /// Shows a picker that selects one or more folders/directorys and invokes
    /// `on_dismiss` when the dialog is dismissed.
    pub fn pick_folders<Callback>(&self, pick_in: &impl PickFile, on_dismiss: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        pick_in.pick_folders(self, on_dismiss);
    }
}

/// A file type filter used in a [`FilePicker`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileType {
    name: String,
    extensions: Vec<String>,
}

impl FileType {
    /// Returns a new file type from the given name and list of file extensions.
    pub fn new<Extension>(
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = Extension>,
    ) -> Self
    where
        Extension: Into<String>,
    {
        Self {
            name: name.into(),
            extensions: extensions.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns true if the given path matches this file type's extensions.
    #[must_use]
    pub fn matches(&self, path: &Path) -> bool {
        let Some(extension) = path.extension() else {
            return false;
        };
        self.extensions.iter().any(|test| **test == *extension)
    }
}

impl<Name, Extension, const EXTENSIONS: usize> From<(Name, [Extension; EXTENSIONS])> for FileType
where
    Name: Into<String>,
    Extension: Into<String>,
{
    fn from((name, extensions): (Name, [Extension; EXTENSIONS])) -> Self {
        Self::new(name, extensions)
    }
}

/// Shows a [`FilePicker`] in a given mode.
pub trait PickFile {
    /// Shows a picker that selects a single file and invokes `on_dismiss` when
    /// the dialog is dismissed.
    fn pick_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static;
    /// Shows a picker that creates a new file and invokes `on_dismiss` when the
    /// dialog is dismissed.
    fn save_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static;
    /// Shows a picker that selects one or more files and invokes `on_dismiss`
    /// when the dialog is dismissed.
    fn pick_files<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static;
    /// Shows a picker that selects a single folder/directory and invokes
    /// `on_dismiss` when the dialog is dismissed.
    fn pick_folder<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static;
    /// Shows a picker that selects one or more folders/directorys and invokes
    /// `on_dismiss` when the dialog is dismissed.
    fn pick_folders<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static;
}

#[derive(Clone, Copy, Debug)]
enum ModeKind {
    File,
    SaveFile,
    Files,
    Folder,
    Folders,
}

impl ModeKind {
    const fn is_multiple(self) -> bool {
        matches!(self, ModeKind::Files | ModeKind::Folders)
    }

    const fn is_file(self) -> bool {
        matches!(self, ModeKind::File | ModeKind::Files | ModeKind::SaveFile)
    }
}

enum ModeCallback {
    Single(OnceCallback<Option<PathBuf>>),
    Multiple(OnceCallback<Option<Vec<PathBuf>>>),
}

enum Mode {
    File(OnceCallback<Option<PathBuf>>),
    SaveFile(OnceCallback<Option<PathBuf>>),
    Files(OnceCallback<Option<Vec<PathBuf>>>),
    Folder(OnceCallback<Option<PathBuf>>),
    Folders(OnceCallback<Option<Vec<PathBuf>>>),
}

impl Mode {
    fn file<Callback>(callback: Callback) -> Self
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        Self::File(OnceCallback::new(callback))
    }

    fn save_file<Callback>(callback: Callback) -> Self
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        Self::SaveFile(OnceCallback::new(callback))
    }

    fn files<Callback>(callback: Callback) -> Self
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        Self::Files(OnceCallback::new(callback))
    }

    fn folder<Callback>(callback: Callback) -> Self
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        Self::Folder(OnceCallback::new(callback))
    }

    fn folders<Callback>(callback: Callback) -> Self
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        Self::Folders(OnceCallback::new(callback))
    }

    fn into_callback(self) -> ModeCallback {
        match self {
            Mode::File(once_callback)
            | Mode::SaveFile(once_callback)
            | Mode::Folder(once_callback) => ModeCallback::Single(once_callback),
            Mode::Files(once_callback) | Mode::Folders(once_callback) => {
                ModeCallback::Multiple(once_callback)
            }
        }
    }

    fn kind(&self) -> ModeKind {
        match self {
            Mode::File(_) => ModeKind::File,
            Mode::SaveFile(_) => ModeKind::SaveFile,
            Mode::Files(_) => ModeKind::Files,
            Mode::Folder(_) => ModeKind::Folder,
            Mode::Folders(_) => ModeKind::Folders,
        }
    }
}

impl PickFile for Modal {
    fn pick_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        let modal = self.clone();
        self.present(FilePickerWidget {
            picker: picker.clone(),
            mode: Mode::file(move |result| {
                modal.dismiss();
                callback(result);
            }),
        });
    }

    fn save_file<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        let modal = self.clone();
        self.present(FilePickerWidget {
            picker: picker.clone(),
            mode: Mode::save_file(move |result| {
                modal.dismiss();
                callback(result);
            }),
        });
    }

    fn pick_files<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        let modal = self.clone();
        self.present(FilePickerWidget {
            picker: picker.clone(),
            mode: Mode::files(move |result| {
                modal.dismiss();
                callback(result);
            }),
        });
    }

    fn pick_folder<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<PathBuf>) + Send + 'static,
    {
        let modal = self.clone();
        self.present(FilePickerWidget {
            picker: picker.clone(),
            mode: Mode::folder(move |result| {
                modal.dismiss();
                callback(result);
            }),
        });
    }

    fn pick_folders<Callback>(&self, picker: &FilePicker, callback: Callback)
    where
        Callback: FnOnce(Option<Vec<PathBuf>>) + Send + 'static,
    {
        let modal = self.clone();
        self.present(FilePickerWidget {
            picker: picker.clone(),
            mode: Mode::folders(move |result| {
                modal.dismiss();
                callback(result);
            }),
        });
    }
}

struct FilePickerWidget {
    picker: FilePicker,
    mode: Mode,
}

impl MakeWidget for FilePickerWidget {
    #[allow(clippy::too_many_lines)]
    fn make_widget(self) -> crate::widget::WidgetInstance {
        let kind = self.mode.kind();
        let callback = Arc::new(Mutex::new(Some(self.mode.into_callback())));

        let title = if self.picker.title.is_empty() {
            match kind {
                ModeKind::File => "Select a file",
                ModeKind::SaveFile => "Save file",
                ModeKind::Files => "Select one or more files",
                ModeKind::Folder => "Select a folder",
                ModeKind::Folders => "Select one or more folders",
            }
        } else {
            &self.picker.title
        };

        let caption = match kind {
            ModeKind::File | ModeKind::Files | ModeKind::Folder | ModeKind::Folders => "Select",
            ModeKind::SaveFile => "Save",
        };

        let chosen_paths = Dynamic::<Vec<PathBuf>>::default();
        let confirm_enabled = chosen_paths.map_each(|paths| !paths.is_empty());

        let browsing_directory = Dynamic::new(
            self.picker
                .directory
                .or_else(|| env::current_dir().ok())
                .or_else(|| {
                    env::current_exe()
                        .ok()
                        .and_then(|exe| exe.parent().map(Path::to_path_buf))
                })
                .unwrap_or_default(),
        );

        let current_directory_files = browsing_directory.map_each(|dir| {
            let mut children = Vec::new();
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries.filter_map(Result::ok) {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        children.push((name, entry.path()));
                    }
                }
                Err(err) => return Err(format!("Error reading directory: {err}")),
            }
            Ok(children)
        });

        let multi_click_threshold = Dynamic::new(Duration::from_millis(500));

        let choose_file = SharedCallback::new({
            let chosen_paths = chosen_paths.clone();
            let callback = callback.clone();
            let types = self.picker.types.clone();
            move |()| {
                let chosen_paths = chosen_paths.get();
                match callback.lock().take() {
                    Some(ModeCallback::Single(cb)) => {
                        let mut chosen_path = chosen_paths.into_iter().next();
                        if let Some(chosen_path) = &mut chosen_path {
                            if matches!(kind, ModeKind::SaveFile)
                                && !types.iter().any(|t| t.matches(chosen_path))
                            {
                                if let Some(extension) =
                                    types.first().and_then(|ty| ty.extensions.first())
                                {
                                    let path = chosen_path.as_mut_os_string();
                                    path.push(".");
                                    path.push(extension);
                                }
                            }
                        }

                        cb.invoke(chosen_path);
                    }
                    Some(ModeCallback::Multiple(cb)) => {
                        cb.invoke(Some(chosen_paths));
                    }
                    None => {}
                }
            }
        });

        let file_list = current_directory_files
            .map_each({
                let chosen_paths = chosen_paths.clone();
                let allowed_types = self.picker.types.clone();
                let multi_click_threshold = multi_click_threshold.clone();
                let browsing_directory = browsing_directory.clone();
                let choose_file = choose_file.clone();
                move |files| match files {
                    Ok(files) => files
                        .iter()
                        .filter(|(name, path)| {
                            !name.starts_with('.') && path.is_dir()
                                || (kind.is_file()
                                    && allowed_types.iter().all(|ty| ty.matches(path)))
                        })
                        .map({
                            |(name, full_path)| {
                                let selected = chosen_paths.map_each({
                                    let full_path = full_path.clone();
                                    move |chosen| chosen.contains(&full_path)
                                });

                                name.align_left()
                                    .into_button()
                                    .kind(ButtonKind::Transparent)
                                    .on_click({
                                        let mut counter =
                                            ClickCounter::new(multi_click_threshold.clone(), {
                                                let browsing_directory = browsing_directory.clone();
                                                let choose_file = choose_file.clone();
                                                let full_path = full_path.clone();

                                                move |click_count, _| {
                                                    if click_count == 2 {
                                                        if full_path.is_dir() {
                                                            browsing_directory
                                                                .set(full_path.clone());
                                                        } else {
                                                            choose_file.invoke(());
                                                        }
                                                    }
                                                }
                                            })
                                            .with_maximum(2);

                                        let chosen_paths = chosen_paths.clone();
                                        let full_path = full_path.clone();
                                        move |click| {
                                            if kind.is_multiple()
                                                && click.map_or(false, |click| {
                                                    click.modifiers.state().primary()
                                                })
                                            {
                                                let mut paths = chosen_paths.lock();
                                                let mut removed = false;
                                                paths.retain(|p| {
                                                    if p == &full_path {
                                                        removed = true;
                                                        false
                                                    } else {
                                                        true
                                                    }
                                                });
                                                if !removed {
                                                    paths.push(full_path.clone());
                                                }
                                            } else {
                                                let mut paths = chosen_paths.lock();
                                                paths.clear();
                                                paths.push(full_path.clone());
                                            }

                                            counter.click(click);
                                        }
                                    })
                                    .with_dynamic(
                                        &WidgetBackground,
                                        DynamicComponent::new(move |ctx| {
                                            if selected.get_tracking_invalidate(ctx) {
                                                Some(ctx.get(&PrimaryColor).into())
                                            } else {
                                                None
                                            }
                                        }),
                                    )
                            }
                        })
                        .collect::<WidgetList>()
                        .into_rows()
                        .make_widget(),
                    Err(err) => err.make_widget(),
                }
            })
            .vertical_scroll()
            .expand();

        let file_ui = if matches!(kind, ModeKind::SaveFile) {
            let name = Dynamic::<String>::default();
            let name_weak = name.downgrade();
            name.set_source(chosen_paths.for_each(move |paths| {
                if paths.len() == 1 && paths[0].is_file() {
                    if let Some(path_name) = paths[0]
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                    {
                        if let Some(name) = name_weak.upgrade() {
                            name.set(path_name);
                        }
                    }
                }
            }));
            let browsing_directory = browsing_directory.clone();
            let chosen_paths = chosen_paths.clone();
            name.for_each(move |name| {
                let Ok(mut paths) = chosen_paths.try_lock() else {
                    return;
                };
                paths.clear();
                paths.push(browsing_directory.get().join(name));
            })
            .persist();
            file_list.and(name.into_input()).into_rows().make_widget()
        } else {
            file_list.make_widget()
        };

        let click_duration_probe = Custom::empty().on_mounted({
            move |ctx| multi_click_threshold.set(ctx.cushy().multi_click_threshold())
        });

        title
            .and(click_duration_probe)
            .into_columns()
            .and(file_ui.width(Lp::inches(6)).height(Lp::inches(4)))
            .and(
                "Cancel"
                    .into_button()
                    .on_click({
                        let mode = callback.clone();
                        move |_| match mode.lock().take() {
                            Some(ModeCallback::Single(cb)) => cb.invoke(None),
                            Some(ModeCallback::Multiple(cb)) => {
                                cb.invoke(None);
                            }
                            None => {}
                        }
                    })
                    .into_escape()
                    .and(
                        caption
                            .into_button()
                            .on_click(move |_| choose_file.invoke(()))
                            .into_default()
                            .with_enabled(confirm_enabled),
                    )
                    .into_columns()
                    .align_right(),
            )
            .into_rows()
            .contain()
            .make_widget()
    }
}
