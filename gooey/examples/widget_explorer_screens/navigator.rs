use gooey::{
    core::{
        styles::{Alignment, VerticalAlignment},
        WeakWidgetRegistration,
    },
    widgets::{
        button::Button,
        component::{Behavior, Content, EventMapper},
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
        navigator::Navigator,
    },
};

use super::Page;

#[derive(Debug)]
pub struct Demo {
    navigator: WeakWidgetRegistration,
    level: usize,
}

#[derive(Debug)]
pub enum Event {
    Push,
    Replace,
    Home,
}
const BUTTON_HEIGHT: f32 = 44.;
const BUTTON_PADDING: f32 = 16.;

impl Demo {
    pub fn new(navigator: WeakWidgetRegistration, level: usize) -> Self {
        Self { navigator, level }
    }
}

impl Behavior for Demo {
    type Content = Layout;
    type Event = Event;
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        events: &EventMapper<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        builder
            .with(
                None,
                Label::new(
                    "This demo shows off the basic functionality of the Navigator. When clicking \
                     a button on the main menu, the widget-explorer example is 'pushing' \
                     `Page::Navigator { level: 0 }`. The bar at the top is called the navigator \
                     bar. When you are at the 'root', there is no button shown on the left side. \
                     When you push a Location to the navigator, the back button will show up, \
                     allowing the user to navigate to the previous Location.\n\nWhen you click \
                     'Push' below, a new location with `level + 1` will be pushed onto the \
                     navigator. The current `level` is shown in the title of the navigator \
                     bar.\n\nWhen you click 'Swap', instead of pushing a new location with `level \
                     + 1`, the top location is replaced with the new location. Notice when \
                     clicking 'Swap' how the back button doesn't change.\n\nWhen you click 'Go \
                     Home', the navigator is popped to the root location. This takes you to the \
                     main menu in this example.",
                )
                .with(Alignment::Center)
                .with(VerticalAlignment::Top),
                WidgetLayout::build()
                    .top(Dimension::exact(16.))
                    .fill_width()
                    .bottom(Dimension::exact(80.))
                    .finish(),
            )
            .with(
                None,
                Button::new("Push", events.map(|_| Event::Push)),
                WidgetLayout::build()
                    .left(Dimension::percent(0.1))
                    .width(Dimension::percent(0.2))
                    .bottom(Dimension::exact(BUTTON_PADDING))
                    .height(Dimension::exact(BUTTON_HEIGHT))
                    .finish(),
            )
            .with(
                None,
                Button::new("Replace", events.map(|_| Event::Replace)),
                WidgetLayout::build()
                    .left(Dimension::percent(0.4))
                    .width(Dimension::percent(0.2))
                    .bottom(Dimension::exact(BUTTON_PADDING))
                    .height(Dimension::exact(BUTTON_HEIGHT))
                    .finish(),
            )
            .with(
                None,
                Button::new("Go Home", events.map(|_| Event::Home)),
                WidgetLayout::build()
                    .left(Dimension::percent(0.7))
                    .width(Dimension::percent(0.2))
                    .bottom(Dimension::exact(BUTTON_PADDING))
                    .height(Dimension::exact(BUTTON_HEIGHT))
                    .finish(),
            )
            .finish()
    }

    fn receive_event(
        component: &mut gooey_widgets::component::Component<Self>,
        event: Self::Event,
        context: &gooey_core::Context<gooey_widgets::component::Component<Self>>,
    ) {
        let navigator = component.navigator.upgrade().expect("navigator not found");
        context.map_widget_mut(
            navigator.id(),
            |navigator: &mut Navigator<Page>, context| match event {
                Event::Push => {
                    navigator.push(
                        Page::Navigator {
                            level: component.level + 1,
                        },
                        context,
                    );
                }

                Event::Replace => {
                    navigator.swap_to(
                        Page::Navigator {
                            level: component.level + 1,
                        },
                        context,
                    );
                }
                Event::Home => {
                    navigator.pop_to_root(context);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gooey::HeadlessError;
    use gooey_core::{
        euclid::{Point2D, Size2D},
        styles::SystemTheme,
    };
    use gooey_widgets::navigator::Navigator;

    use crate::widget_explorer_screens::Page;

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = crate::app().headless();
            let mut recorder = headless.begin_recording(Size2D::new(480, 320), theme, true, 15);
            recorder.set_cursor(Point2D::new(100., 200.));

            // Open the navigator demo
            recorder
                .move_cursor_to(Point2D::new(150., 300.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(500));

            recorder.map_root_widget(|navigator: &mut Navigator<Page>, _context| {
                assert_eq!(navigator.location(), &Page::Navigator { level: 0 });
            });

            // Go back
            recorder
                .move_cursor_to(Point2D::new(30., 30.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(300));

            recorder.map_root_widget(|navigator: &mut Navigator<Page>, _context| {
                assert_eq!(navigator.location(), &Page::MainMenu);
            });

            // Enter back into the navigator demo
            recorder
                .move_cursor_to(Point2D::new(150., 300.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(500));

            // Push a few entries
            for i in 1_u8..3 {
                recorder
                    .move_cursor_to(
                        Point2D::new(130. + i as f32, 290.),
                        Duration::from_millis(10),
                    )
                    .await?;
                recorder.left_click().await?;
                recorder.pause(Duration::from_millis(100));
            }

            recorder.map_root_widget(|navigator: &mut Navigator<Page>, _context| {
                assert_eq!(navigator.back_stack(), &[
                    Page::MainMenu,
                    Page::Navigator { level: 0 },
                    Page::Navigator { level: 1 },
                    Page::Navigator { level: 2 },
                ]);
            });

            // Replace the top
            recorder
                .move_cursor_to(Point2D::new(240., 290.), Duration::from_millis(10))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(500));

            recorder.map_root_widget(|navigator: &mut Navigator<Page>, _context| {
                assert_eq!(navigator.back_stack(), &[
                    Page::MainMenu,
                    Page::Navigator { level: 0 },
                    Page::Navigator { level: 1 },
                    Page::Navigator { level: 3 },
                ]);
            });

            // Go home
            recorder
                .move_cursor_to(Point2D::new(420., 290.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(1000));

            recorder.map_root_widget(|navigator: &mut Navigator<Page>, _context| {
                assert_eq!(navigator.back_stack(), &[Page::MainMenu,]);
            });

            recorder.save_apng(crate::harness::snapshot_path(
                "widget-explorer",
                &format!("Navigator-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}
