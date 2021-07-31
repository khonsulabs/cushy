use std::marker::PhantomData;

use gooey_core::{
    styles::{style_sheet::Classes, Alignment, FontSize, VerticalAlignment},
    Context, StyledWidget, WeakWidgetRegistration, Widget, WidgetStorage,
};

use super::{Location, Navigator};
use crate::{
    button::Button,
    component::{Behavior, Component, Content, EventMapper},
    label::Label,
    layout::{Dimension, Layout, WidgetLayout},
};

pub trait NavigatorBar: Widget {
    fn new(navigator: WeakWidgetRegistration, storage: &WidgetStorage) -> StyledWidget<Self>;
    fn set_back_button(&mut self, caption: Option<&str>, context: &Context<Self>);
    fn set_title(&mut self, title: &str, context: &Context<Self>);
}

pub type DefaultBar<Loc> = Component<DefaultBarBehavior<Loc>>;

#[derive(Debug)]
pub struct DefaultBarBehavior<Loc: Location> {
    navigator: WeakWidgetRegistration,
    _loc: PhantomData<Loc>,
}

impl<Loc: Location> NavigatorBar for Component<DefaultBarBehavior<Loc>> {
    fn new(navigator: WeakWidgetRegistration, storage: &WidgetStorage) -> StyledWidget<Self> {
        Self::new(
            DefaultBarBehavior {
                navigator,
                _loc: PhantomData::default(),
            },
            storage,
        )
    }

    #[allow(clippy::blocks_in_if_conditions)]
    fn set_back_button(&mut self, label: Option<&str>, context: &Context<Self>) {
        if let Some(label) = label {
            if self
                .map_widget_mut(
                    &Widgets::BackButton,
                    context,
                    |button: &mut Button, context| {
                        button.set_label(label, context);
                    },
                )
                .is_none()
            {
                // Back button doesn't exist
                let button = context.register(Button::new(label, self.map_event(|_| Event::Back)));

                self.map_content_mut(context, |layout, context| {
                    layout.insert_registration(
                        Some(Widgets::BackButton),
                        button,
                        WidgetLayout::build()
                            .left(Dimension::exact(10.))
                            .top(Dimension::exact(10.))
                            .bottom(Dimension::exact(10.))
                            .width(Dimension::exact(150.)) // TODO need a "minimal" dimension for width
                            .finish(),
                        context,
                    );
                });
            }
        } else {
            self.map_content_mut(context, |layout, context| {
                layout.remove_child(&Widgets::BackButton, context);
            });
        }
    }

    fn set_title(&mut self, title: &str, context: &Context<Self>) {
        self.map_widget_mut(&Widgets::Title, context, |label: &mut Label, context| {
            label.set_label(title, context);
        });
    }
}

#[derive(Debug)]
pub enum Event {
    Back,
}

impl<Loc: Location> Behavior for DefaultBarBehavior<Loc> {
    type Content = Layout;
    type Event = Event;
    type Widgets = Widgets;

    fn classes() -> Option<Classes> {
        Some(Classes::from("gooey-navigator-bar"))
    }

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content> {
        builder
            .with(
                Widgets::Title,
                Label::new("")
                    .with(Alignment::Center)
                    .with(VerticalAlignment::Center)
                    .with(FontSize::new(18.)),
                WidgetLayout::fill(),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &gooey_core::Context<Component<Self>>,
    ) {
        let Event::Back = event;

        if let Some(navigator) = component.navigator.upgrade() {
            context.map_widget_mut(navigator.id(), |navigator: &mut Navigator<Loc>, context| {
                navigator.behavior.pop(context);
            });
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Widgets {
    Title,
    BackButton,
}
