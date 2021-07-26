use gooey::{
    core::{Context, StyledWidget},
    widgets::{
        checkbox::Checkbox,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};

fn main() {
    App::from_root(|storage| Component::<Counter>::default_for(storage))
        .with(ComponentTransmogrifier::<Counter>::default())
        .run()
}

#[derive(Default, Debug)]
struct Counter;

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Container> {
        StyledWidget::from(
            builder.register(
                CounterWidgets::Button,
                Checkbox::build()
                    .labeled("I'm a checkbox. Hear me roar.")
                    .on_clicked(builder.map_event(|_| CounterEvent::ButtonClicked))
                    .finish(),
            ),
        )
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.map_widget_mut(
            &CounterWidgets::Button,
            context,
            |checkbox: &mut Checkbox, context| {
                if checkbox.checked() {
                    checkbox.set_label("I'm a checked checkbox now.", context);
                } else {
                    checkbox.set_label("I am no longer checked.", context);
                }
            },
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
