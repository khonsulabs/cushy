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

#[cfg(test)]
mod harness;

fn app() -> App {
    App::from_root(|storage| Component::<Counter>::default_for(storage)).with_component::<Counter>()
}

fn main() {
    app().run();
}

#[derive(Clone, Default, Debug)]
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Container> {
        builder
            .child(
                CounterWidgets::Button,
                Button::build()
                    .labeled("Click the gooey cinnamon rolls!")
                    .on_clicked(events.map(|_| CounterEvent::ButtonClicked))
                    .image(Image::from(Asset::build().path(vec!["rolls.jpg"]).finish()))
                    .finish(),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.count += 1;

        let button_state = component
            .widget_state(&CounterWidgets::Button, context)
            .unwrap();
        let mut button = button_state.lock::<Button>(context.frontend()).unwrap();
        button
            .widget
            .set_label(component.count.to_string(), &button.context);
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Button,
}

#[derive(Debug)]
enum CounterEvent {
    ButtonClicked,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gooey::{
        core::{figures::Size, styles::SystemTheme},
        HeadlessError,
    };

    use super::*;

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = app().headless();
            let mut recorder = headless.begin_recording(Size::new(320, 240), theme, true, 30);
            recorder.set_cursor((100., 200.));
            recorder.render_frame(Duration::from_millis(100)).await?;
            recorder
                .move_cursor_to((160., 130.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(250));
            recorder.left_click().await?;

            assert_eq!(
                "1",
                &recorder
                    .map_root_widget(|component: &mut Component<Counter>, context| {
                        component
                            .map_widget(
                                &CounterWidgets::Button,
                                &context,
                                |button: &Button, _context| button.label().to_owned(),
                            )
                            .unwrap()
                    })
                    .unwrap()
            );

            recorder
                .move_cursor_to((200., 180.), Duration::from_millis(300))
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
