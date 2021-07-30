#![allow(clippy::zero_sized_map_values)]

use std::{borrow::Cow, fmt::Debug, hash::Hash};

use gooey_core::{
    styles::style_sheet::Classes, Context, DefaultWidget, StyledWidget, WeakWidgetRegistration,
    WidgetRegistration, WidgetStorage,
};
use url::Url;

use crate::{
    component::{
        Behavior, Component, ComponentBuilder, ComponentCommand, ContentBuilder, EventMapper,
    },
    layout::{self, Dimension, Layout, WidgetLayout},
};

mod bar;

pub use bar::{DefaultBar, DefaultBarBehavior, NavigatorBar};

pub type Navigator<Loc> = Component<NavigatorBehavior<Loc>>;

#[derive(Debug)]
pub struct NavigatorBehavior<Loc: Location> {
    back_stack: Vec<Loc>,
}

impl<Loc: Location> NavigatorBehavior<Loc> {
    pub fn default_for(storage: &WidgetStorage) -> StyledWidget<Component<Self>> {
        Component::default_for(storage)
    }

    pub fn push(&mut self, location: Loc, context: &Context<Component<Self>>) {
        self.back_stack.push(location);
        context.send_command(ComponentCommand::Behavior(Event::ContentChanged));
    }

    pub fn swap_to(&mut self, location: Loc, context: &Context<Component<Self>>) {
        *self.back_stack.last_mut().unwrap() = location;
        context.send_command(ComponentCommand::Behavior(Event::ContentChanged));
    }

    pub fn pop_to_root(&mut self, context: &Context<Component<Self>>) {
        self.back_stack.truncate(1);
        context.send_command(ComponentCommand::Behavior(Event::ContentChanged));
    }

    pub fn pop(&mut self, context: &Context<Component<Self>>) -> Option<Loc> {
        if self.back_stack.len() <= 1 {
            None
        } else {
            let location = self.back_stack.pop();
            context.send_command(ComponentCommand::Behavior(Event::ContentChanged));
            location
        }
    }

    fn replace_content(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        let location = component.back_stack.last().unwrap();
        let new_widget = location.materialize(context, context.registration().clone());
        println!("Registering widget");
        component.register_widget(Widgets::Content, &new_widget);
        println!("mapping content");
        component.map_content_mut(context, |layout, context| {
            println!("inserting registation");
            layout.insert_registration(
                Some(Widgets::Content),
                new_widget,
                content_layout(),
                context,
            );
        });
        Self::update_bar(component, context);
    }

    fn update_bar(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        component.map_widget_mut(&Widgets::Bar, context, |bar: &mut Loc::Bar, context| {
            bar.set_title(
                component
                    .behavior
                    .back_stack
                    .last()
                    .unwrap()
                    .title()
                    .as_ref(),
                context,
            );

            if component.back_stack.len() > 1 {
                let back_title = component.back_stack[component.back_stack.len() - 2].title();
                bar.set_back_button(Some(&back_title), context);
            } else {
                bar.set_back_button(None, context);
            }
        });
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Widgets {
    Bar,
    Content,
}

pub trait Location:
    Clone + Default + Hash + PartialEq + Send + Sync + Sized + Debug + 'static
{
    type Bar: NavigatorBar;
    fn title(&self) -> Cow<'_, str>;

    fn materialize(
        &self,
        storage: &WidgetStorage,
        navigator: WeakWidgetRegistration,
    ) -> WidgetRegistration;

    fn serialize(&self) -> Url;

    fn deserialize(url: Url) -> Self;

    fn navigator(storage: &WidgetStorage) -> StyledWidget<Component<NavigatorBehavior<Self>>> {
        Component::default_for(storage)
    }
}

impl<Loc: Location> Default for NavigatorBehavior<Loc> {
    fn default() -> Self {
        Self {
            back_stack: vec![Loc::default()],
        }
    }
}

#[derive(Debug)]
pub enum Event {
    ContentChanged,
}

impl<Loc: Location> Behavior for NavigatorBehavior<Loc> {
    type Content = Layout;
    type Event = Event;
    type Widgets = Widgets;

    fn classes() -> Option<Classes> {
        Some(Classes::from("gooey-navigator"))
    }

    fn initialize(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        Self::update_bar(component, context);
    }

    fn build_content(
        &mut self,
        builder: layout::Builder<'_, Widgets, Event, ComponentBuilder<Self>>,
        _events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content> {
        let initial_content = self.back_stack.last().unwrap();
        let navigator = builder.component().unwrap();
        let initial_widget = initial_content.materialize(builder.storage(), navigator.clone());
        let bar = Loc::Bar::new(navigator, builder.storage());
        builder
            .with(
                Widgets::Bar,
                bar,
                WidgetLayout::build()
                    .top(Dimension::zero())
                    .height(Dimension::exact(44.))
                    .left(Dimension::zero())
                    .right(Dimension::zero())
                    .finish(),
            )
            .with_registration(Widgets::Content, initial_widget, content_layout())
            .finish()
    }

    fn receive_event(
        component: &mut crate::component::Component<Self>,
        event: Self::Event,
        context: &gooey_core::Context<crate::component::Component<Self>>,
    ) {
        let Event::ContentChanged = event;
        Self::replace_content(component, context);
    }
}

fn content_layout() -> WidgetLayout {
    WidgetLayout::build()
        .top(Dimension::exact(44.))
        .left(Dimension::zero())
        .right(Dimension::zero())
        .finish()
}
