use gooey_core::Widget;

#[derive(Eq, PartialEq, Clone)]
pub struct Button {
    pub label: String,
    pub disabled: bool,
}

pub enum ButtonEvent {
    Clicked,
}

impl Widget for Button {
    type TransmogrifierEvent = ButtonEvent;
}

pub struct ButtonTransmogrifier;
