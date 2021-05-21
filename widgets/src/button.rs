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

#[derive(Debug)]
pub enum ButtonEvent {
    Clicked,
}

impl Widget for Button {
    type TransmogrifierCommand = ();
    type TransmogrifierEvent = ButtonEvent;

    fn receive_event(
        &mut self,
        event: Self::TransmogrifierEvent,
        context: &gooey_core::Context<Self>,
    ) where
        Self: Sized,
    {
        todo!()
    }
}

pub struct ButtonTransmogrifier;
