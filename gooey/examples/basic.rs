use gooey::{
    core::{Context, StyledWidget},
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        container::Container,
    },
};
use gooey_core::{Transmogrifiers, WidgetStorage};
use harness::UserInterface;

mod harness;

impl UserInterface for Counter {
    type Root = Component<Self>;

    fn root_widget(storage: &WidgetStorage) -> StyledWidget<Self::Root> {
        Component::<Counter>::default_for(storage)
    }

    fn transmogrifiers(transmogrifiers: &mut Transmogrifiers<gooey::ActiveFrontend>) {
        transmogrifiers
            .register_transmogrifier(ComponentTransmogrifier::<Counter>::default())
            .unwrap();
    }
}

fn main() {
    Counter::run();
}

#[derive(Default, Debug)]
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Container> {
        StyledWidget::from(builder.register(
            CounterWidgets::Button,
            Button::new(
                "Click Me!",
                builder.map_event(|_| CounterEvent::ButtonClicked),
            ),
        ))
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        component.map_widget_mut(
            &CounterWidgets::Button,
            context,
            |button: &mut Button, context| {
                button.set_label(component.behavior.count.to_string(), context);
            },
        );
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
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
            let mut headless = Counter::headless();
            let mut recorder = headless.begin_recording(Size2D::new(320, 240), theme, true, 30);
            recorder.set_cursor(Point2D::new(100., 200.));
            recorder.render_frame(Duration::from_millis(100)).await?;
            recorder
                .move_cursor_to(Point2D::new(160., 130.), Duration::from_millis(300))
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
                .move_cursor_to(Point2D::new(200., 180.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(1000));

            recorder.save_mp4(harness::snapshot_path(
                "basic",
                &format!("Demo-{:?}.mp4", theme),
            )?)?;
        }
        Ok(())
    }
}
