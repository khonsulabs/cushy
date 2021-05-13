use crate::Widget;

#[derive(Eq, PartialEq, Clone)]
pub struct Button {
    pub label: String,
    pub disabled: bool,
}

pub enum ButtonEvent {
    Clicked,
}

impl Widget for Button {
    type MaterializerEvent = ButtonEvent;
    type State = Self;
    type Layout = ();

    fn state(&self) -> Self::State {
        self.clone()
    }

    fn layout(&self) -> Self::Layout {}
}

pub struct ButtonMaterializer;
