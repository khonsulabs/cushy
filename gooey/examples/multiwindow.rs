//! This example shows off how to open multiple windows and how callbacks can be
//! used to communicate between components on separate windows safely.
//!
//! This example is contrived, but shows the flexibility (and fragility) of
//! using callbacks for communicating between windows. The "Count w/ Parent"
//! button sends and message to the window that created it to count with its
//! parents as well. This will recurse down to the original window that spawned
//! that series of windows. If the base window spawns two separate windows,
//! however, both of their "Count w/ Parent" buttons will not interact with each
//! other since they don't know about each other -- only about the window that
//! spawned them.

use std::sync::atomic::{AtomicU32, Ordering};

use gooey::{
    core::{
        figures::{Figure, Vector},
        styles::{Alignment, FontSize, VerticalAlignment},
        Callback, Context, DefaultWidget, StyledWidget, WindowBuilder,
    },
    widgets::{
        button::Button,
        component::{Behavior, Component, Content, EventMapper},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
    },
    App,
};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

fn app() -> App {
    App::from(
        WindowBuilder::new(|storage| Component::<Counter>::default_for(storage))
            .title("MultiWindow Demo - Window 0"),
    )
    .with_component::<Counter>()
}

fn main() {
    app().run();
}

#[derive(Default, Debug)]
struct Counter {
    id: u32,
    count: u32,
    previous_window_callback: Option<Callback>,
}

impl Behavior for Counter {
    type Content = Layout;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Layout> {
        builder
            .with(
                None,
                Button::new("New Window", events.map(|_| CounterEvent::NewWindow)),
                WidgetLayout::build()
                    .left(Dimension::Exact(Figure::new(0.)))
                    .top(Dimension::Percent(0.1))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .with(
                None,
                Button::new("Count Solo", events.map(|_| CounterEvent::CountSolo)),
                WidgetLayout::build()
                    .left(Dimension::Exact(Figure::new(0.)))
                    .top(Dimension::Percent(0.4))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .with(
                None,
                Button::new(
                    "Count w/ Parent",
                    events.map(|_| CounterEvent::CountWithParent),
                ),
                WidgetLayout::build()
                    .left(Dimension::Exact(Figure::new(0.)))
                    .top(Dimension::Percent(0.7))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .with(
                CounterWidgets::Label,
                Label::new(self.count.to_string())
                    .with(FontSize::new(36.))
                    .with(Alignment::Center)
                    .with(VerticalAlignment::Center),
                WidgetLayout::build()
                    .right(Dimension::Exact(Figure::new(0.)))
                    .top(Dimension::Percent(0.4))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        match event {
            CounterEvent::NewWindow => {
                let count = component.count;
                let parent_id = component.id;
                let new_id = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
                let previous_window_callback =
                    Some(component.map_event(|_| CounterEvent::CountWithParent));
                WindowBuilder::new(move |storage| {
                    Component::<Counter>::new(
                        Counter {
                            id: new_id,
                            count,
                            previous_window_callback,
                        },
                        storage,
                    )
                })
                .position(context.window().unwrap().inner_position() + Vector::new(32, 32))
                .title(format!("{} - Child of {}", new_id, parent_id))
                .open(context.frontend());
            }
            CounterEvent::CountSolo => {
                component.count += 1;
            }
            CounterEvent::CountWithParent => {
                component.count += 1;
                if let Some(callback) = &component.previous_window_callback {
                    callback.invoke(());
                }
            }
        }

        let label_state = component
            .widget_state(&CounterWidgets::Label, context)
            .unwrap();
        let mut label = label_state.lock::<Label>(context.frontend()).unwrap();
        label
            .widget
            .set_label(component.count.to_string(), &label.context);
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Label,
}

#[derive(Debug)]
enum CounterEvent {
    NewWindow,
    CountSolo,
    CountWithParent,
}
