use gooey_core::{Callback, Context, Widget};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Button {
    pub label: String,
    pub clicked: Callback,
}

#[derive(Debug)]
pub enum InternalButtonEvent {
    Clicked,
}

#[derive(Debug)]
pub enum ButtonCommand {
    SetLabel(String),
}

impl Widget for Button {
    type Command = ButtonCommand;
    type TransmogrifierCommand = ButtonCommand;
    type TransmogrifierEvent = InternalButtonEvent;

    fn receive_event(
        &mut self,
        event: Self::TransmogrifierEvent,
        _context: &gooey_core::Context<Self>,
    ) {
        let InternalButtonEvent::Clicked = event;
        self.clicked.invoke(());
    }

    /// Called when an `event` from the transmogrifier was received.
    #[allow(unused_variables)]
    fn receive_command(&mut self, command: Self::Command, context: &Context<Self>) {
        match &command {
            ButtonCommand::SetLabel(label) => {
                self.label = label.clone();
            }
        }

        context.send_command(command);
    }
}

#[derive(Debug)]
pub struct ButtonTransmogrifier;
