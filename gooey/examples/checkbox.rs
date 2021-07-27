use gooey::{
    core::{Context, StyledWidget},
    widgets::{
        checkbox::Checkbox,
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
struct Counter;

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Container> {
        StyledWidget::from(
            builder.register(
                CounterWidgets::Button,
                Checkbox::build()
                    .labeled("I'm a checkbox. Hear me roar.")
                    .on_clicked(builder.map_event(|_| CounterEvent::ButtonClicked))
                    .finish(),
            ),
        )
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
                .move_cursor_to(Point2D::new(160., 120.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;

            assert_eq!(
                true,
                recorder
                    .map_root_widget(|component: &mut Component<Counter>, context| {
                        component
                            .map_widget(
                                &CounterWidgets::Button,
                                &context,
                                |button: &Checkbox, _context| button.checked(),
                            )
                            .unwrap()
                    })
                    .unwrap()
            );
            recorder
                .move_cursor_to(Point2D::new(150., 140.), Duration::from_millis(100))
                .await?;
            recorder.pause(Duration::from_millis(500));
            recorder
                .move_cursor_to(Point2D::new(160., 120.), Duration::from_millis(200))
                .await?;

            recorder.left_click().await?;

            assert_eq!(
                false,
                recorder
                    .map_root_widget(|component: &mut Component<Counter>, context| {
                        component
                            .map_widget(
                                &CounterWidgets::Button,
                                &context,
                                |button: &Checkbox, _context| button.checked(),
                            )
                            .unwrap()
                    })
                    .unwrap()
            );

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
