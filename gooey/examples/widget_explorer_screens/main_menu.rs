use gooey_core::{
    styles::{Alignment, VerticalAlignment},
    WeakWidgetRegistration,
};
use gooey_widgets::{
    button::Button,
    component::Behavior,
    label::Label,
    layout::{Dimension, Layout, WidgetLayout},
    navigator::{Location, Navigator},
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
            buttons: vec![Page::Navigator { level: 0 }],
        }
    }
}

impl Behavior for MainMenu {
    type Content = Layout;
    type Event = usize;
    type Widgets = ();

    fn create_content(
        &mut self,
        builder: &mut gooey_widgets::component::ComponentBuilder<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        let mut layout = Layout::build::<()>(builder) // TODO having to specify the type here sucks
            .with(
                None,
                Label::new(
                    "This is the Gooey widget explorer. Over time, it will grow to include many \
                     examples of how to use the widgets Gooey provides to build user \
                     interfaces.\n\nThe overall interface is powered by the Navigator widget, \
                     which provides the navigation bar at the top of this window.",
                )
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
            layout = layout.with(
                None,
                Button::new(button.title(), builder.map_event(move |_| index)),
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
