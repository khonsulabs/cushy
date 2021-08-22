use gooey::{
    core::{
        styles::{Alignment, VerticalAlignment},
        WeakWidgetRegistration,
    },
    widgets::{
        button::Button,
        component::{Behavior, Content, EventMapper},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
        navigator::{Location, Navigator},
    },
};

use super::Page;

#[derive(Debug)]
pub struct MainMenu {
    navigator: WeakWidgetRegistration,
    buttons: Vec<Page>,
}

impl MainMenu {
    pub fn new(navigator: WeakWidgetRegistration) -> Self {
        Self {
            navigator,
            buttons: vec![Page::Navigator { level: 0 }, Page::Borders, Page::Focus],
        }
    }
}

impl Behavior for MainMenu {
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
            Label::new(
                "This is the Gooey widget explorer. Over time, it will grow to include many \
                 examples of how to use the widgets Gooey provides to build user \
                 interfaces.\n\nThe overall interface is powered by the Navigator widget, which \
                 provides the navigation bar at the top of this window.",
            )
            .with(Alignment::Center)
            .with(VerticalAlignment::Center),
            WidgetLayout::build()
                .top(Dimension::zero())
                .fill_width()
                .height(Dimension::percent(0.8))
                .finish(),
        );

        let button_width = 1. / self.buttons.len() as f32;
        for (index, button) in self.buttons.iter().enumerate() {
            layout = layout.with(
                None,
                Button::new(button.title(), events.map(move |_| index)),
                WidgetLayout::build()
                    .bottom(Dimension::zero())
                    .left(Dimension::percent(index as f32 * button_width))
                    .width(Dimension::percent(button_width))
                    .height(Dimension::exact(44.))
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
