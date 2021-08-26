//! This is a simple demonstration of the List widget.
//!
//! The list widget is *not* a table view. It is the equivalent of
//! unordered/ordered lists in html.
//!
//! This example is more of a testbed and not a good example -- it will be
//! replaced with something better eventually.

use gooey::{
    core::{Callback, Context, DefaultWidget, StyledWidget},
    widgets::{
        button::Button,
        component::{Behavior, Component, Content, EventMapper},
        label::Label,
        list::{List, OrderedListKind},
    },
    App,
};

fn app() -> App {
    App::from_root(|storage| Component::<Lists>::default_for(storage)).with_component::<Lists>()
}

fn main() {
    app().run();
}

#[derive(Clone, Default, Debug)]
struct Lists {
    count: u32,
}

impl Behavior for Lists {
    type Content = List;
    type Event = ();
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> StyledWidget<List> {
        let sub_list = List::unadorned(builder.storage())
            .with(Label::new("Sub List"))
            .with(
                List::build(builder.storage())
                    .ordered(OrderedListKind::RomanLower)
                    .with(Label::new("Sub 1"))
                    .with(Label::new("Sub 2"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .with(Label::new("Test"))
                    .finish(),
            )
            .finish();
        builder
            .ordered(OrderedListKind::RomanUpper)
            .with(Label::new("Test"))
            .with(sub_list)
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Label::new("Test"))
            .with(Button::new("Test", Callback::default()))
            .finish()
    }

    fn receive_event(
        _component: &mut Component<Self>,
        _event: Self::Event,
        _context: &Context<Component<Self>>,
    ) {
    }
}
