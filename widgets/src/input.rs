use gooey_core::{Callback, Context, StyledWidget, Widget};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug, Default)]
pub struct Input {
    value: String,
    password: bool,
    selection_start: usize,
    selection_end: Option<usize>,
    changed: Callback<()>,
    selection_changed: Callback<()>,
}

impl Input {
    pub fn build() -> Builder {
        Builder::default()
    }

    pub fn new<S: Into<String>>(value: S, changed: Callback<()>) -> StyledWidget<Self> {
        StyledWidget::from(Self {
            value: value.into(),
            changed,
            ..Input::default()
        })
    }

    pub fn set_value(&mut self, value: impl Into<String>, context: &Context<Self>) {
        self.value = value.into();
        context.send_command(Command::ValueSet);
    }

    pub fn set_selection(&mut self, start: usize, end: Option<usize>, context: &Context<Self>) {
        self.selection_start = start;
        self.selection_end = end;
        context.send_command(Command::SelectionSet);
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn password_mode(&self) -> bool {
        self.password
    }

    pub fn set_password_mode(&mut self, enabled: bool, context: &Context<Self>) {
        if self.password != enabled {
            self.password = enabled;
            context.send_command(Command::PasswordModeSet);
        }
    }
}

#[derive(Debug)]
pub enum Command {
    ValueSet,
    SelectionSet,
    PasswordModeSet,
}

#[derive(Debug)]
pub enum Event {
    ValueChanged,
    SelectionChanged,
}

impl Widget for Input {
    type Command = Command;
    type Event = Event;

    const CLASS: &'static str = "gooey-input";
    const FOCUSABLE: bool = true;
}

#[derive(Debug)]
pub struct InputTransmogrifier;

#[derive(Debug, Default)]
#[must_use]
pub struct Builder {
    input: Input,
}

impl Builder {
    pub fn value<S: Into<String>>(mut self, value: S) -> Self {
        self.input.value = value.into();
        self
    }

    pub fn password(mut self) -> Self {
        self.input.password = true;
        self
    }

    pub fn on_changed(mut self, callback: Callback) -> Self {
        self.input.changed = callback;
        self
    }

    pub fn on_selection_changed(mut self, callback: Callback) -> Self {
        self.input.selection_changed = callback;
        self
    }

    pub fn finish(self) -> StyledWidget<Input> {
        StyledWidget::from(self.input)
    }
}
