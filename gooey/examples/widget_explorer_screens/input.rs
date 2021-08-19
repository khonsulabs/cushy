use gooey::{
    core::styles::{Alignment, VerticalAlignment},
    widgets::{
        component::{Behavior, Content, EventMapper},
        input::Input,
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
    },
};
use gooey_core::styles::Autofocus;

#[derive(Debug, Default)]
pub struct Demo {}

impl Behavior for Demo {
    type Content = Layout;
    type Event = ();
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        builder
            .with(
                None,
                Label::new(
                    "This is a text input widget demo. You can try typing in the field. Copy and \
                     paste should also work correctly.",
                )
                .with(Alignment::Center)
                .with(VerticalAlignment::Top),
                WidgetLayout::build()
                    .top(Dimension::exact(16.))
                    .fill_width()
                    .bottom(Dimension::percent(50.))
                    .finish(),
            )
            .with(
                None,
                Input::build().value("Lorem Ipsum").finish().with(Autofocus),
                WidgetLayout::build()
                    .left(Dimension::percent(0.1))
                    .width(Dimension::percent(0.8))
                    .bottom(Dimension::percent(0.25))
                    .finish(),
            )
            .finish()
    }

    fn receive_event(
        _component: &mut gooey_widgets::component::Component<Self>,
        _event: Self::Event,
        _context: &gooey_core::Context<gooey_widgets::component::Component<Self>>,
    ) {
    }
}
