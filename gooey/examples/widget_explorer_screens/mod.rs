use std::{borrow::Cow, fmt::Debug};

use gooey::{
    core::{
        styles::{Alignment, VerticalAlignment},
        WeakWidgetRegistration,
    },
    widgets::{
        button::Button,
        component::{Behavior, Component, Content, EventMapper},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
        navigator::{DefaultBar, Location, Navigator},
        url::Url,
    },
};

use crate::widget_explorer_screens::main_menu::MainMenu;

pub mod main_menu;
pub mod navigator;

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum Page {
    MainMenu,
    Navigator { level: usize },
}

impl Default for Page {
    fn default() -> Self {
        Self::MainMenu
    }
}

impl Location for Page {
    type Bar = DefaultBar<Self>;

    fn title(&self) -> Cow<'_, str> {
        match self {
            Page::MainMenu => Cow::from("Main Menu"),
            Page::Navigator { level } => {
                if *level > 0 {
                    Cow::from(format!("Navigator Demo - {}", level + 1))
                } else {
                    Cow::from("Navigator Demo")
                }
            }
        }
    }

    fn materialize(
        &self,
        storage: &gooey_core::WidgetStorage,
        navigator: WeakWidgetRegistration,
    ) -> gooey_core::WidgetRegistration {
        match self {
            Page::MainMenu => storage.register(Component::new(MainMenu::new(navigator), storage)),
            Page::Navigator { level } => storage.register(Component::new(
                navigator::Demo::new(navigator, *level),
                storage,
            )),
        }
    }

    fn serialize(&self) -> Url {
        todo!()
    }

    fn deserialize(url: Url) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct InfoPage {
    navigator: WeakWidgetRegistration,
    text: String,
    buttons: Vec<Page>,
}

impl Behavior for InfoPage {
    type Content = Layout;
    type Event = usize;
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        let mut layout = builder.with(
            None,
            Label::new(&self.text)
                .with(Alignment::Center)
                .with(VerticalAlignment::Center),
            WidgetLayout::build()
                .top(Dimension::zero())
                .right(Dimension::zero())
                .left(Dimension::zero())
                .height(Dimension::percent(80.))
                .finish(),
        );

        let button_width = 1. / self.buttons.len() as f32;
        for (index, button) in self.buttons.iter().enumerate() {
            let callback = events.map(move |_| index);
            layout = layout.with(
                None,
                Button::new(button.title(), callback),
                WidgetLayout::build()
                    .bottom(Dimension::zero())
                    .left(Dimension::percent(index as f32 * button_width))
                    .right(Dimension::percent(1. - (index + 1) as f32 * button_width))
                    .height(Dimension::exact(44.))
                    // .width(Dimension::percent(button_width))
                    .finish(),
            );
        }

        layout.finish()
    }

    fn receive_event(
        component: &mut gooey_widgets::component::Component<Self>,
        event: Self::Event,
        context: &gooey_core::Context<gooey_widgets::component::Component<Self>>,
    ) {
        let button = &component.buttons[event];
        if let Some(navigator) = component.navigator.upgrade() {
            context.map_widget_mut(
                navigator.id(),
                |navigator: &mut Navigator<Page>, context| {
                    navigator.push(button.clone(), context);
                },
            );
        }
    }
}
