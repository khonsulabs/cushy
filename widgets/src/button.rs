use gooey_core::{
    stylecs::{Points, Style},
    Widget,
};

#[derive(Debug)]
pub struct Button {
    pub label: String,
    pub style: Style<Points>,
}

pub enum ButtonEvent {
    Clicked,
}

impl Widget for Button {
    type TransmogrifierEvent = ButtonEvent;
}

pub struct ButtonTransmogrifier;
