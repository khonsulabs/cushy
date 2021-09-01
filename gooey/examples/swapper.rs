//! An example showing how to swap between two components with a parent component.

use gooey::{
    core::{
        assets::{Asset, Image},
        Context, DefaultWidget, StyledWidget,
    },
    widgets::{
        button::Button,
        component::{Behavior, Component, Content, EventMapper},
        container::Container,
    },
    App,
};
use gooey_core::{Callback, WidgetRegistration, WidgetStorage};
use gooey_rasterizer::WidgetRasterizer;
use gooey_widgets::layout::{Layout, WidgetLayout};

#[cfg(test)]
mod harness;

fn app() -> App {
    App::from_root(|storage| Component::<Counter>::default_for(storage))
        .with_component::<Counter>()
        .with_component::<AppPage>()
        .with_component::<Settings>()
}

fn main() {
    app().run();
}

#[derive(Clone, Default, Debug)]
struct Counter {
    page: Page,
}

#[derive(Clone, Debug)]
enum Page {
    App,
    Settings,
}

impl Default for Page {
    fn default() -> Self {
        Self::App
    }
}

impl Behavior for Counter {
    type Content = Layout;
    type Event = SwapperEvent;
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Layout> {
        let page = self.page.build_content(builder.storage(), events);
        builder
            .with_registration(
                (),
                page,
                WidgetLayout::build().fill_width().fill_height().finish(),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let SwapperEvent::ChangePage(page) = event;
        component.map_content_mut(context, |layout: &mut Layout, context| {
            layout.insert_registration(
                Some(()),
                page.build_content(&context.frontend().storage(), &component.event_mapper()),
                WidgetLayout::build().fill_width().fill_height().finish(),
                context,
            );
        });
        component.behavior.page = page;
    }
}

#[derive(Debug)]
enum SwapperEvent {
    ChangePage(Page),
}

impl Page {
    fn build_content(
        &self,
        storage: &WidgetStorage,
        events: &EventMapper<Counter>,
    ) -> WidgetRegistration {
        match self {
            Page::App => storage.register(Component::<AppPage>::new(
                AppPage {
                    swap_to_settings: events.map(|_| SwapperEvent::ChangePage(Page::Settings)),
                },
                storage,
            )),
            Page::Settings => storage.register(Component::<Settings>::new(
                Settings {
                    swap_to_app: events.map(|_| SwapperEvent::ChangePage(Page::App)),
                },
                storage,
            )),
        }
    }
}

#[derive(Debug)]
struct AppPage {
    swap_to_settings: Callback,
}

#[derive(Debug)]
enum AppEvent {
    SwapToSettings,
}

impl Behavior for AppPage {
    type Content = Container;
    type Event = AppEvent;
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content> {
        builder
            .child(
                None,
                Button::new("Settings", events.map(|_| AppEvent::SwapToSettings)),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let AppEvent::SwapToSettings = event;
        component.behavior.swap_to_settings.invoke(());
    }
}

#[derive(Debug)]
struct Settings {
    swap_to_app: Callback,
}

#[derive(Debug)]
enum SettingsEvent {
    SwapToApp,
}

impl Behavior for Settings {
    type Content = Container;
    type Event = SettingsEvent;
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Self::Content> {
        builder
            .child(
                None,
                Button::new("App", events.map(|_| SettingsEvent::SwapToApp)),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let SettingsEvent::SwapToApp = event;
        component.behavior.swap_to_app.invoke(());
    }
}
