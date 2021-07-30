use gooey_core::{euclid::Length, Callback, Context, Points, StyledWidget, Widget};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

pub const LABEL_PADDING: Length<f32, Points> = Length::new(5.);

#[derive(Default, Debug)]
pub struct Checkbox {
    label: String,
    checked: bool,
    toggled: Callback,
}

impl Checkbox {
    pub fn build() -> Builder {
        Builder::new()
    }

    pub fn new<S: Into<String>>(label: S, checked: bool, toggled: Callback) -> StyledWidget<Self> {
        StyledWidget::from(Self {
            label: label.into(),
            toggled,
            checked,
        })
    }

    pub fn set_checked(&mut self, checked: bool, context: &Context<Self>) {
        self.checked = checked;
        context.send_command(CheckboxCommand::Toggled);
    }

    #[must_use]
    pub const fn checked(&self) -> bool {
        self.checked
    }

    pub fn set_label<S: Into<String>>(&mut self, label: S, context: &Context<Self>) {
        self.label = label.into();
        context.send_command(CheckboxCommand::LabelChanged);
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug)]
#[must_use]
pub struct Builder {
    checkbox: Checkbox,
}

impl Builder {
    fn new() -> Self {
        Self {
            checkbox: Checkbox::default(),
        }
    }

    pub fn labeled<S: Into<String>>(mut self, label: S) -> Self {
        self.checkbox.label = label.into();
        self
    }

    pub const fn checked(mut self) -> Self {
        self.checkbox.checked = true;
        self
    }

    pub fn on_clicked(mut self, clicked: Callback) -> Self {
        self.checkbox.toggled = clicked;
        self
    }

    pub fn finish(self) -> StyledWidget<Checkbox> {
        StyledWidget::from(self.checkbox)
    }
}

#[derive(Debug)]
pub enum InternalCheckboxEvent {
    Clicked,
}

#[derive(Debug)]
pub enum CheckboxCommand {
    Toggled,
    LabelChanged,
}

impl Widget for Checkbox {
    type Command = CheckboxCommand;
    type Event = InternalCheckboxEvent;

    const CLASS: &'static str = "gooey-checkbox";

    fn receive_event(&mut self, event: Self::Event, context: &Context<Self>) {
        let InternalCheckboxEvent::Clicked = event;
        self.set_checked(!self.checked, context);
        self.toggled.invoke(());
    }
}

#[derive(Debug)]
pub struct CheckboxTransmogrifier;
