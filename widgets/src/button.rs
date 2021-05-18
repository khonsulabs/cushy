use gooey_core::{
    styles::{Points, Style},
    Widget,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

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
