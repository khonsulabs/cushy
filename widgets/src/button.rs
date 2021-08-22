use gooey_core::{
    styles::{
        style_sheet::Classes, BackgroundColor, ColorPair, FallbackComponent, Style, StyleComponent,
    },
    Callback, Context, StyledWidget, Widget, SOLID_WIDGET_CLASS,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug, Default)]
#[must_use]
pub struct Button {
    label: String,
    clicked: Callback,
}

impl Button {
    pub fn build() -> Builder {
        Builder::new()
    }

    pub fn new<S: ToString>(label: S, clicked: Callback) -> StyledWidget<Self> {
        StyledWidget::from(Self {
            label: label.to_string(),
            clicked,
        })
    }

    pub fn set_label(&mut self, label: impl Into<String>, context: &Context<Self>) {
        self.label = label.into();
        context.send_command(ButtonCommand::LabelChanged);
    }

    #[must_use]
    pub fn label(&self) -> &str {
        self.label.as_str()
    }
}

#[derive(Debug)]
pub enum InternalButtonEvent {
    Clicked,
}

#[derive(Debug)]
pub enum ButtonCommand {
    LabelChanged,
}

impl Widget for Button {
    type Command = ButtonCommand;
    type Event = InternalButtonEvent;

    const CLASS: &'static str = "gooey-button";
    const FOCUSABLE: bool = true;

    fn classes() -> Classes {
        Classes::from(vec![Self::CLASS, SOLID_WIDGET_CLASS])
    }

    fn receive_event(&mut self, event: Self::Event, _context: &gooey_core::Context<Self>) {
        let InternalButtonEvent::Clicked = event;
        self.clicked.invoke(());
    }

    fn background_color(style: &Style) -> Option<&'_ ColorPair> {
        style.get_with_fallback::<ButtonColor>()
    }
}

#[derive(Debug)]
#[must_use]
pub struct Builder {
    button: Button,
}

impl Builder {
    fn new() -> Self {
        Self {
            button: Button::default(),
        }
    }

    pub fn labeled<S: Into<String>>(mut self, label: S) -> Self {
        self.button.label = label.into();
        self
    }

    pub fn on_clicked(mut self, callback: Callback) -> Self {
        self.button.clicked = callback;
        self
    }

    pub fn finish(self) -> StyledWidget<Button> {
        StyledWidget::from(self.button)
    }
}

#[derive(Debug)]
pub struct ButtonTransmogrifier;

/// The button's background color.
#[derive(Debug, Clone)]
pub struct ButtonColor(pub ColorPair);
impl StyleComponent for ButtonColor {}

impl From<ButtonColor> for ColorPair {
    fn from(color: ButtonColor) -> Self {
        color.0
    }
}

impl FallbackComponent for ButtonColor {
    type Fallback = BackgroundColor;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}
