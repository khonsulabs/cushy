use gooey::{
    core::{Context, DefaultWidget, LocalizationParameters, StyledWidget},
    fluent::{
        fluent::{bundle::FluentBundle, FluentResource},
        FluentLocalizer,
    },
    widgets::{
        component::{Behavior, Component, Content, ContentBuilder, EventMapper},
        container::Container,
        label::Label,
    },
    App,
};

#[cfg(test)]
mod harness;

fn app() -> App {
    let mut english = FluentBundle::new_concurrent(vec!["en-US".parse().unwrap()]);
    english
        .add_resource(
            FluentResource::try_new(String::from(
                r#"hello = Hello, {$user -> 
                        [friend] Friend 
                        *[other] {$user}
                    }!"#,
            ))
            .unwrap(),
        )
        .unwrap();
    let mut spanish = FluentBundle::new_concurrent(vec!["es-MX".parse().unwrap()]);
    spanish
        .add_resource(
            FluentResource::try_new(String::from(
                r#"hello = Hola, {$user -> 
                        [friend] Amigo 
                        *[other] {$user}
                    }!"#,
            ))
            .unwrap(),
        )
        .unwrap();
    let mut german = FluentBundle::new_concurrent(vec!["de-DE".parse().unwrap()]);
    german
        .add_resource(
            FluentResource::try_new(String::from(
                r#"hello = Hallo, {$user -> 
                        [friend] Freund 
                        *[other] {$user}
                    }!"#,
            ))
            .unwrap(),
        )
        .unwrap();
    App::from_root(|storage| Component::<Localization>::default_for(storage))
        .with_component::<Localization>()
        .localizer(FluentLocalizer::new(
            "en-US".parse().unwrap(),
            vec![english, spanish, german],
        ))
}

fn main() {
    app().run();
}

#[derive(Clone, Default, Debug)]
struct Localization;

impl Behavior for Localization {
    type Content = Container;
    type Event = ();
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> StyledWidget<Container> {
        let user = std::env::var("USER")
            .ok()
            .filter(|username| !username.is_empty())
            .unwrap_or_else(|| String::from("friend"));
        let label =
            Label::new(builder.localize("hello", LocalizationParameters::new().with("user", user)));
        builder.child(None, label).finish()
    }

    fn receive_event(
        _component: &mut Component<Self>,
        _event: Self::Event,
        _context: &Context<Component<Self>>,
    ) {
    }
}
