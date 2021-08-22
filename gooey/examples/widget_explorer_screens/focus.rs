use gooey::{
    core::styles::{Alignment, Autofocus, TabIndex, VerticalAlignment},
    widgets::{
        button::Button,
        component::{Behavior, Content, EventMapper},
        input::Input,
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
    },
};

#[derive(Debug, Default)]
pub struct Demo {}

impl Behavior for Demo {
    type Content = Layout;
    type Event = ();
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        const BUTTON_WIDTH: f32 = 1. / 4.;
        builder
            .with(
                None,
                Label::new(
                    "This pane demonstrates tab ordering. The input field receives focus \
                     automatically, but has TabIndex(1). The buttons are labeled with their tab \
                     indexes.",
                )
                .with(Alignment::Center)
                .with(VerticalAlignment::Top),
                WidgetLayout::build()
                    .top(Dimension::exact(16.))
                    .fill_width()
                    .bottom(Dimension::percent(0.50))
                    .finish(),
            )
            .with(
                None,
                Input::build()
                    .value("Lorem Ipsum")
                    .finish()
                    .with(Autofocus)
                    .with(TabIndex(1)),
                WidgetLayout::build()
                    .left(Dimension::percent(0.05))
                    .width(Dimension::percent(0.9))
                    .bottom(Dimension::percent(0.25))
                    .finish(),
            )
            .with(
                None,
                Button::build()
                    .labeled("TabIndex(0)")
                    .finish()
                    .with(TabIndex(0)),
                WidgetLayout::build()
                    .left(Dimension::zero())
                    .width(Dimension::percent(BUTTON_WIDTH))
                    .bottom(Dimension::zero())
                    .finish(),
            )
            .with(
                None,
                Button::build().labeled("No TabIndex").finish(),
                WidgetLayout::build()
                    .left(Dimension::percent(BUTTON_WIDTH * 2.))
                    .width(Dimension::percent(BUTTON_WIDTH))
                    .bottom(Dimension::zero())
                    .finish(),
            )
            .with(
                None,
                Button::build().labeled("No TabIndex").finish(),
                WidgetLayout::build()
                    .left(Dimension::percent(BUTTON_WIDTH * 3.))
                    .width(Dimension::percent(BUTTON_WIDTH))
                    .bottom(Dimension::zero())
                    .finish(),
            )
            .with(
                None,
                Button::build()
                    .labeled("TabIndex(2)")
                    .finish()
                    .with(TabIndex(2)),
                WidgetLayout::build()
                    .left(Dimension::percent(BUTTON_WIDTH * 1.))
                    .width(Dimension::percent(BUTTON_WIDTH))
                    .bottom(Dimension::zero())
                    .finish(),
            )
            .finish()
    }

    fn receive_event(
        _component: &mut gooey_widgets::component::Component<Self>,
        _event: Self::Event,
        _context: &gooey_core::Context<gooey_widgets::component::Component<Self>>,
    ) {
    }
}

#[cfg(test)]
mod tests {

    use gooey::HeadlessError;
    use gooey_core::{figures::Size, styles::SystemTheme};

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        use std::time::Duration;

        use gooey_core::figures::Point;
        use gooey_rasterizer::winit::event::{ModifiersState, VirtualKeyCode};

        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = crate::app().headless();
            let mut recorder = headless.begin_recording(Size::new(480, 320), theme, true, 15);
            recorder.set_cursor(Point::new(100., 200.));

            // Open the focus demo
            recorder
                .move_cursor_to(Point::new(400., 300.), Duration::from_millis(300))
                .await?;
            recorder.left_click().await?;
            recorder.pause(Duration::from_millis(500));

            // Focus is on the input widget.
            recorder.press_key(VirtualKeyCode::Tab, None).await?;
            recorder.press_key(VirtualKeyCode::Tab, None).await?;
            recorder.press_key(VirtualKeyCode::Tab, None).await?;
            recorder.pause(Duration::from_millis(300));
            recorder
                .press_key(VirtualKeyCode::Tab, ModifiersState::SHIFT)
                .await?;
            recorder
                .press_key(VirtualKeyCode::Tab, ModifiersState::SHIFT)
                .await?;
            recorder
                .press_key(VirtualKeyCode::Tab, ModifiersState::SHIFT)
                .await?;
            recorder
                .press_key(VirtualKeyCode::Tab, ModifiersState::SHIFT)
                .await?;

            recorder.pause(Duration::from_millis(500));
            recorder.save_apng(crate::harness::snapshot_path(
                "widget-explorer",
                &format!("Focus-{:?}.png", theme),
            )?)?;
        }
        Ok(())
    }
}
