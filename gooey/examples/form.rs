use gooey::{
    core::{Context, DefaultWidget, StyledWidget},
    widgets::{
        component::{Behavior, Component, Content, ContentBuilder, EventMapper},
        container::Container,
        form::{ChangeEvent, Form, FormWidget, Model, TextField},
    },
    App,
};
use gooey_widgets::form::FormKey;

#[cfg(test)]
mod harness;

fn app() -> App {
    App::from_root(|storage| Component::<Counter>::default_for(storage))
        .with_component::<Counter>()
        .with_form::<SignIn>()
}

fn main() {
    app().run();
}

#[derive(Clone, Default, Debug)]
struct Counter {
    count: u32,
}

#[derive(Debug, Default)]
struct SignIn {
    username: String,
    password: String,
}

impl Model for SignIn {
    type Fields = SignInFields;
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum SignInFields {
    Username,
    Password,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = FormEvent;
    type Widgets = Widgets;

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Container> {
        let form = Form::build(SignIn::default(), builder.storage())
            .field(
                SignInFields::Username,
                TextField::simple(|model: &mut SignIn| &mut model.username),
            )
            .field(
                SignInFields::Password,
                TextField::build_simple(|model: &mut SignIn| &mut model.password)
                    .password()
                    .finish(),
            )
            .on_changed(events.map(FormEvent::FormChanged))
            .finish();
        builder.child(Widgets::Form, form).finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let FormEvent::FormChanged(event) = event;

        component.map_widget_mut(
            &Widgets::Form,
            context,
            |form: &mut Component<Form<SignIn>>, _context| {
                let model = form.model.lock();
                println!("Model changed: {:?}, Event: {:?}", model, event);
            },
        );
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
enum Widgets {
    Form,
}

#[derive(Debug)]
enum FormEvent {
    FormChanged(ChangeEvent),
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gooey::{
        core::{
            figures::{Point, Size},
            styles::SystemTheme,
        },
        HeadlessError,
    };

    use super::*;

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = app().headless();
            let mut recorder = headless.begin_recording(Size::new(320, 240), theme, true, 30);
            recorder.set_cursor(Point::new(100., 200.));
            recorder.render_frame(Duration::from_millis(100)).await?;
            recorder
                .move_cursor_to(Point::new(160., 130.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(250));
            recorder.left_click().await?;

            assert_eq!(
                "1",
                &recorder
                    .map_root_widget(|component: &mut Component<Counter>, context| {
                        component
                            .map_widget(&Widgets::Form, &context, |button: &Button, _context| {
                                button.label().to_owned()
                            })
                            .unwrap()
                    })
                    .unwrap()
            );

            recorder
                .move_cursor_to(Point::new(200., 180.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(1000));

            recorder.save_apng(harness::snapshot_path(
                "basic",
                &format!("Demo-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}

impl FormKey for SignInFields {
    fn label(&self, context: &gooey_core::AppContext) -> Option<String> {
        Some(match self {
            SignInFields::Username => context.localize("Username", None),
            SignInFields::Password => context.localize("Password", None),
        })
    }
}
