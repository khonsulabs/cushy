use gooey::{
    core::Context,
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        label::Label,
        layout::{Dimension, Layout},
    },
    App,
};
use gooey_core::{
    euclid::Length,
    styles::{Alignment, VerticalAlignment},
    StyledWidget,
};
use gooey_widgets::layout::WidgetLayout;

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
    type Content = Layout;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Layout> {
        Layout::build(builder)
            .with_registration(
                CounterWidgets::Button,
                builder.register_widget(
                    CounterWidgets::Button,
                    Button::new(
                        "Click Me!",
                        builder.map_event(|_| CounterEvent::ButtonClicked),
                    ),
                ),
                WidgetLayout::default()
                    .with_left(Dimension::Exact(Length::new(0.)))
                    .with_top(Dimension::Percent(0.4))
                    .with_height(Dimension::Percent(0.2))
                    .with_width(Dimension::Percent(0.5)),
            )
            .with_registration(
                CounterWidgets::Label,
                builder.register_widget(
                    CounterWidgets::Label,
                    Label::new("0")
                        .with(Alignment::Center)
                        .with(VerticalAlignment::Center),
                ),
                WidgetLayout::default()
                    .with_right(Dimension::Exact(Length::new(0.)))
                    .with_top(Dimension::Percent(0.4))
                    .with_height(Dimension::Percent(0.2))
                    .with_width(Dimension::Percent(0.5)),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        component.with_widget_mut(
            &CounterWidgets::Label,
            context,
            |label: &mut Label, context| {
                label.set_label(component.behavior.count.to_string(), context);
            },
        );
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Button,
    Label,
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}
