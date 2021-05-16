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
    type State = Self;

    fn state(&self) -> Self::State {
        self.clone()
    }
}

pub struct ButtonTransmogrifier;
