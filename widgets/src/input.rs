use gooey_core::{Callback, Context, StyledWidget, Widget};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug, Default)]
pub struct Input {
    value: String,
    selection_start: usize,
    selection_end: Option<usize>,
    changed: Callback<()>,
    selection_changed: Callback<()>,
}

impl Input {
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
}

#[derive(Debug)]
pub enum Command {
    ValueSet,
    SelectionSet,
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
}

#[derive(Debug)]
pub struct InputTransmogrifier;
