use gooey::{
    core::Context,
    widgets::{
        button::{Button, ButtonCommand},
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};

fn main() {
    App::default()
        .with(ComponentTransmogrifier::<Counter>::default())
        .run(|storage| Component::<Counter>::default_for(storage))
}

#[derive(Default, Debug)]
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn initialize(builder: &mut ComponentBuilder<Self>) -> Container {
        Container::from(builder.register_widget(
            CounterWidgets::Button,
            Button {
                label: String::from("Click Me!"),
                clicked: builder.map_event(|_| CounterEvent::ButtonClicked),
            },
        ))
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
