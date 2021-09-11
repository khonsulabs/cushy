use gooey::{
    core::{
        figures::Figure,
        styles::{Alignment, FontSize, VerticalAlignment},
        Context, DefaultWidget, StyledWidget,
    },
    widgets::{
        button::Button,
        component::{Behavior, Component, Content, EventMapper},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
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
struct Counter {
    count: u32,
}

impl Behavior for Counter {
    type Content = Layout;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> StyledWidget<Layout> {
        builder
            .with(
                None,
                Button::new("Click Me!", events.map(|_| CounterEvent::ButtonClicked)),
                WidgetLayout::build()
                    .left(Dimension::Exact(Figure::new(0.)))
                    .top(Dimension::Percent(0.4))
                    .height(Dimension::Percent(0.2))
                    .width(Dimension::Percent(0.5))
                    .finish(),
            )
            .with(
                CounterWidgets::Label,
                Label::new("0")
                    .with(FontSize::new(36.))
                    .with(Alignment::Center)
                    .with(VerticalAlignment::Center),
                WidgetLayout::build()
                    .right(Dimension::Exact(Figure::new(0.)))
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

        let label_state = component
            .widget_state(&CounterWidgets::Label, context)
            .unwrap();
        let mut label = label_state.lock::<Label>(context.frontend()).unwrap();
        label
            .widget
            .set_label(component.count.to_string(), &label.context);
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
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
                .move_cursor_to((80., 120.), Duration::from_millis(300))
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
                .move_cursor_to((200., 180.), Duration::from_millis(300))
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
