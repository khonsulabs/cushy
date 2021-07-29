use gooey::{
    core::{
        euclid::Length,
        styles::{Alignment, VerticalAlignment},
        Context, StyledWidget,
    },
    widgets::{
        button::Button,
        component::{Behavior, Component, ComponentBuilder},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
    },
    App,
};
use gooey_core::{
    styles::{BackgroundColor, Color},
    DefaultWidget,
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
                WidgetLayout::build()
                    .left(Dimension::Exact(Length::new(0.)))
                    .top(Dimension::Percent(0.4))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .with_registration(
                CounterWidgets::Label,
                builder.register(
                    CounterWidgets::Label,
                    Label::new("0")
                        .with(Alignment::Center)
                        .with(VerticalAlignment::Center)
                        .with(BackgroundColor(Color::new(1., 0., 0., 0.7).into())),
                ),
                WidgetLayout::build()
                    .right(Dimension::Exact(Length::new(0.)))
                    .top(Dimension::Percent(0.4))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
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

        component.map_widget_mut(
            &CounterWidgets::Label,
            context,
            |label: &mut Label, context| {
                label.set_label(component.count.to_string(), context);
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
            let mut headless = app().headless();
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
                            component.count
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
