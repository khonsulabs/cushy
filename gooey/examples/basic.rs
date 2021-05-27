use gooey::{
    core::Context,
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentInitializer, ComponentTransmogrifier},
        container::Container,
    },
    App,
};

fn main() {
    App::default()
        .with(ComponentTransmogrifier::<Counter>::default())
        .run(|storage| Component::<Counter>::new(storage))
}

#[derive(Default, Debug)]
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn initialize(mut initializer: ComponentInitializer<Self>) -> Component<Self> {
        Component::initialized(
            Container::from(
                initializer.register_with_id(CounterWidgets::Button, Button {
                    label: String::from("Click Me!"),
                    clicked: initializer.map_event(|_| CounterEvent::ButtonClicked),
                }),
            ),
            Self::default(),
            initializer,
        )
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        component.send_command_to::<Button>(
            &CounterWidgets::Button,
            ButtonCommand::SetLabel(component.behavior.count.to_string()),
            context,
        );
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Button,
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}
