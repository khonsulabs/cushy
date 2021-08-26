use std::time::Duration;

use gooey::{
    core::{Context, DefaultWidget, StyledWidget, Timer},
    widgets::{
        component::{Behavior, Component, Content, EventMapper},
        container::Container,
        label::Label,
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
    timer: Option<Timer>,
}

impl Behavior for Counter {
    type Content = Container;
    type Event = CounterEvent;
    type Widgets = CounterWidgets;

    fn initialize(component: &mut Component<Self>, context: &Context<Component<Self>>) {
        component.timer = Some(
            context
                .timer(
                    Duration::from_secs(1),
                    component.map_event(|_| CounterEvent::TimerFired),
                )
                .repeating()
                .schedule(),
        );
    }

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> StyledWidget<Container> {
        builder
            .child(CounterWidgets::Label, Label::new("0"))
            .finish()
    }

    fn receive_event(
        component: &mut Component<Self>,
        event: Self::Event,
        context: &Context<Component<Self>>,
    ) {
        let CounterEvent::TimerFired = event;
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
    TimerFired,
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
            let mut recorder = headless.begin_recording(Size::new(320, 240), theme, false, 30);
            const ONE_SECOND: Duration = Duration::from_millis(1000);
            recorder.render_frame(ONE_SECOND).await?;
            for _ in 0..5 {
                tokio::time::sleep(ONE_SECOND).await;
                recorder.render_frame(ONE_SECOND).await?;
            }

            assert_eq!(
                5,
                recorder
                    .map_root_widget(|component: &mut Component<Counter>, _context| {
                        component.behavior.count
                    })
                    .unwrap()
            );

            recorder.save_apng(harness::snapshot_path(
                "timer",
                &format!("Demo-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}
