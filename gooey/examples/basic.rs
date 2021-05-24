use gooey::{
    core::{Context, Transmogrifiers},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentTransmogrifier},
    },
};

fn main() {
    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Starting up");

    let mut transmogrifiers = Transmogrifiers::default();
    transmogrifiers
        .register_transmogrifier(ComponentTransmogrifier::<Counter>::default())
        .unwrap();
    gooey::main_with(transmogrifiers, |storage| {
        Component::with(storage, Counter::default(), |storage, callbacks| Button {
            label: String::from("Hello, World"),
            clicked: callbacks.map_event(|_| CounterEvent::ButtonClicked),
        })
    })
}

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Button;
    type Event = CounterEvent;

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        context.send_command(ButtonCommand::SetLabel(
            component.behavior.count.to_string(),
        ));
    }
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}
