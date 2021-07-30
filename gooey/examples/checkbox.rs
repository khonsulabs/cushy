use gooey::{
    core::{Context, DefaultWidget, StyledWidget},
    widgets::{
        checkbox::Checkbox,
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

#[derive(Default, Debug)]
struct Counter;

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
                Checkbox::build()
                    .labeled("I'm a checkbox. Hear me roar.")
                    .on_clicked(events.map(|_| CounterEvent::ButtonClicked))
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
        component.map_widget_mut(
            &CounterWidgets::Button,
            context,
            |checkbox: &mut Checkbox, context| {
                if checkbox.checked() {
                    checkbox.set_label("I'm a checked checkbox now.", context);
                } else {
                    checkbox.set_label("I am no longer checked.", context);
                }
            },
        );
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

    use gooey::HeadlessError;
    use gooey_core::{
        euclid::{Point2D, Size2D},
        styles::SystemTheme,
    };

    use super::*;

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = app().headless();
            let mut recorder = headless.begin_recording(Size2D::new(320, 240), theme, true, 30);
            recorder.set_cursor(Point2D::new(100., 200.));
            recorder.render_frame(Duration::from_millis(100)).await?;
            recorder
                .move_cursor_to(Point2D::new(160., 120.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;

            assert!(recorder
                .map_root_widget(|component: &mut Component<Counter>, context| {
                    component
                        .map_widget(
                            &CounterWidgets::Button,
                            &context,
                            |button: &Checkbox, _context| button.checked(),
                        )
                        .unwrap()
                })
                .unwrap());

            // Wiggle the cursor to make the second click seem like a click.
            recorder
                .move_cursor_to(Point2D::new(150., 140.), Duration::from_millis(100))
                .await?;
            recorder.pause(Duration::from_millis(00));
            recorder
                .move_cursor_to(Point2D::new(160., 120.), Duration::from_millis(200))
                .await?;

            recorder.left_click().await?;

            assert!(!recorder
                .map_root_widget(|component: &mut Component<Counter>, context| {
                    component
                        .map_widget(
                            &CounterWidgets::Button,
                            &context,
                            |button: &Checkbox, _context| button.checked(),
                        )
                        .unwrap()
                })
                .unwrap());

            recorder
                .move_cursor_to(Point2D::new(200., 180.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(1000));

            recorder.save_apng(harness::snapshot_path(
                "checkbox",
                &format!("Demo-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}
