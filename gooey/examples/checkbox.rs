use gooey::{
    core::{Context, StyledWidget},
    widgets::{
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
    App,
};
use gooey_core::styles::FontFamily;
use gooey_widgets::checkbox::Checkbox;

fn main() {
    App::default()
        .with(ComponentTransmogrifier::<Counter>::default())
        .run(|storage| Component::<Counter>::default_for(storage))
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
                    .finish()
                    .with(FontFamily::from("Comic Sans")),
            ),
        )
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.map_widget(
            &CounterWidgets::Button,
            context,
            |button: &Checkbox, _context| {
                println!("Checkbox toggled: {:?}", button.checked());
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
