use gooey::{
    core::{Context, Transmogrifiers},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentTransmogrifier},
    },
};
use gooey_core::WidgetId;
use gooey_widgets::{component::CallbackMapper, container::Container};

fn main() {
    #[cfg(target_arch = "wasm32")]
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Starting up");

    let mut transmogrifiers = Transmogrifiers::default();
    transmogrifiers
        .register_transmogrifier(ComponentTransmogrifier::<Counter>::default())
        .unwrap();
    gooey::main_with(transmogrifiers, |storage| {
        Component::<Counter>::new(storage)
    })
}

#[derive(Debug)]
struct Counter {
    button_id: WidgetId,
    count: u32,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;

    fn initialize(callbacks: CallbackMapper<Self>) -> Component<Self> {
        let button = callbacks.register(Button {
            label: String::from("Click Me!"),
            clicked: callbacks.map_event(|_| CounterEvent::ButtonClicked),
        });
        let button_id = button.id().clone();

        Component::initialized(
            Container::from(button),
            Self {
                button_id,
                count: 0,
            },
            callbacks,
        )
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        let button_state = context
            .frontend
            .storage()
            .widget_state(component.behavior.button_id.id)
            .unwrap();
        let button_channels = button_state.channels::<Button>().unwrap();

        button_channels.post_command(ButtonCommand::SetLabel(
            component.behavior.count.to_string(),
        ));
    }
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}
