use gooey::{
    core::{Context, Transmogrifiers},
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, CallbackMapper, Component, ComponentTransmogrifier},
        container::Container,
    },
};

fn main() {
    let mut transmogrifiers = Transmogrifiers::default();
    transmogrifiers
        .register_transmogrifier(ComponentTransmogrifier::<Counter>::default())
        .unwrap();
    gooey::main_with(transmogrifiers, |storage| {
        Component::<Counter>::new(storage)
    })
}

#[derive(Default, Debug)]
struct Counter {
    count: u32,
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Button,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn initialize(mut callbacks: CallbackMapper<Self>) -> Component<Self> {
        Component::initialized(
            Container::from(callbacks.register_with_id(
                CounterWidgets::Button,
                Button {
                    label: String::from("Click Me!"),
                    clicked: callbacks.map_event(|_| CounterEvent::ButtonClicked),
                },
            )),
            Self::default(),
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

        let button = component
            .registered_widget(&CounterWidgets::Button)
            .unwrap();
        let button_state = context.widget_state(button.id().id).unwrap();
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
