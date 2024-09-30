//! Modal dialogs such as message boxes and file pickers.

use std::marker::PhantomData;

use crate::widget::{MakeWidget, SharedCallback};
use crate::widgets::layers::Modal;

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
