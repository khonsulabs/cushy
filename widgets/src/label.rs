use gooey_core::{
    styles::{ColorPair, FallbackComponent, StyleComponent, TextColor},
    Context, StyledWidget, Widget,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Label {
    pub label: String,
}

impl Label {
    pub fn new<S: ToString>(label: S) -> StyledWidget<Self> {
        StyledWidget::default_for(Self {
            label: label.to_string(),
        })
    }
}

#[derive(Debug)]
pub enum InternalButtonEvent {
    Clicked,
}

#[derive(Debug)]
pub enum LabelCommand {
    SetLabel(String),
}

impl Widget for Label {
    type Command = LabelCommand;
    type TransmogrifierCommand = LabelCommand;
    type TransmogrifierEvent = ();

    const CLASS: &'static str = "gooey-label";

    /// Called when an `event` from the transmogrifier was received.
    #[allow(unused_variables)]
    fn receive_command(&mut self, command: Self::Command, context: &Context<Self>) {
        match &command {
            LabelCommand::SetLabel(label) => {
                self.label = label.clone();
            }
        }

        context.send_command(command);
    }
}

#[derive(Debug)]
pub struct LabelTransmogrifier;

/// The button's background color.
#[derive(Debug, Clone)]
pub struct LabelColor(pub ColorPair);
impl StyleComponent for LabelColor {}

impl From<LabelColor> for ColorPair {
    fn from(color: LabelColor) -> Self {
        color.0
    }
}

impl FallbackComponent for LabelColor {
    type Fallback = TextColor;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}
