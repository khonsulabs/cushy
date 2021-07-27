use gooey::{
    core::Context,
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder, ComponentTransmogrifier},
        label::Label,
        layout::{Dimension, Layout},
    },
};
use gooey_core::{
    euclid::Length,
    styles::{Alignment, VerticalAlignment},
    StyledWidget, Transmogrifiers, WidgetStorage,
};
use gooey_widgets::layout::WidgetLayout;
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
    type Content = Layout;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn create_content(&mut self, builder: &mut ComponentBuilder<Self>) -> StyledWidget<Layout> {
        Layout::build(builder)
            .with(
                None,
                Button::new(
                    "Click Me!",
                    builder.map_event(|_| CounterEvent::ButtonClicked),
                ),
                WidgetLayout::default()
                    .with_left(Dimension::Exact(Length::new(0.)))
                    .with_top(Dimension::Percent(0.4))
                    .with_height(Dimension::Percent(0.2))
                    .with_width(Dimension::Percent(0.5)),
            )
            .with_registration(
                Some(CounterWidgets::Label),
                builder.register(
                    CounterWidgets::Label,
                    Label::new("0")
                        .with(Alignment::Center)
                        .with(VerticalAlignment::Center),
                ),
                WidgetLayout::default()
                    .with_right(Dimension::Exact(Length::new(0.)))
                    .with_top(Dimension::Percent(0.4))
                    .with_height(Dimension::Percent(0.2))
                    .with_width(Dimension::Percent(0.5)),
            )
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::ButtonClicked = event;
        component.behavior.count += 1;

        component.map_widget_mut(
            &CounterWidgets::Label,
            context,
            |label: &mut Label, context| {
                label.set_label(component.behavior.count.to_string(), context);
            },
        );
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum CounterWidgets {
    Label,
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
                .move_cursor_to(Point2D::new(80., 120.), Duration::from_millis(300))
                .await?;

            for i in 1_u32..5 {
                recorder.left_click().await?;
                assert_eq!(
                    i,
                    recorder
                        .map_root_widget(|component: &mut Component<Counter>, _context| {
                            component.behavior.count
                        })
                        .unwrap()
                );
            }

            recorder
                .move_cursor_to(Point2D::new(200., 180.), Duration::from_millis(300))
                .await?;
            recorder.pause(Duration::from_millis(1000));

            recorder.save_apng(harness::snapshot_path(
                "layout",
                &format!("Demo-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}
