use gooey::{
    core::{
        styles::{Alignment, Border, BorderOptions, Color, Padding, VerticalAlignment},
        StyledWidget,
    },
    widgets::{
        component::{Behavior, Content, EventMapper},
        container::Container,
        label::Label,
        layout::{Dimension, Layout, WidgetLayout},
    },
};

#[derive(Debug, Default)]
pub struct Demo {}

fn centered_label(title: &str) -> StyledWidget<Label> {
    Label::new(title)
        .with(Alignment::Center)
        .with(VerticalAlignment::Center)
}

impl Behavior for Demo {
    type Content = Layout;
    type Event = ();
    type Widgets = ();

    fn build_content(
        &mut self,
        builder: <Self::Content as Content<Self>>::Builder,
        _events: &EventMapper<Self>,
    ) -> gooey_core::StyledWidget<Self::Content> {
        let border_only = Container::new(
            centered_label("Only Borders")
                .with(Border::uniform(BorderOptions::new(2., Color::RED))),
            builder.storage(),
        );
        let with_padding = Container::new(
            centered_label("With Padding")
                .with(Border::uniform(BorderOptions::new(2., Color::RED)))
                .with(Padding::uniform(10.)),
            builder.storage(),
        );
        builder
            .with(
                None,
                Label::new(
                    "Each widget can have Border and Padding components applied to them. Padding \
                     provides spacing between the edge of the widget and the content area.",
                )
                .with(Alignment::Center)
                .with(VerticalAlignment::Top),
                WidgetLayout::build()
                    .top(Dimension::exact(16.))
                    .fill_width()
                    .bottom(Dimension::exact(50.))
                    .finish(),
            )
            .with(
                None,
                border_only,
                WidgetLayout::build()
                    .left(Dimension::percent(0.))
                    .width(Dimension::percent(0.5))
                    .bottom(Dimension::percent(0.25))
                    .height(Dimension::percent(0.25))
                    .finish(),
            )
            .with(
                None,
                with_padding,
                WidgetLayout::build()
                    .left(Dimension::percent(0.5))
                    .width(Dimension::percent(0.5))
                    .bottom(Dimension::percent(0.25))
                    .height(Dimension::percent(0.25))
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
    use gooey_core::{euclid::Size2D, styles::SystemTheme};

    #[cfg(not(target_arch = "wasm32-unknown-unknown"))]
    #[tokio::test]
    async fn demo() -> Result<(), HeadlessError> {
        use gooey_core::euclid::Point2D;

        for theme in [SystemTheme::Dark, SystemTheme::Light] {
            let mut headless = crate::app().headless();

            // This isn't an interactive recorder. For the events to work, the widgets positions must be known, but they aren't known until the first render.
            headless
                .screenshot(Size2D::new(480, 320), theme, None)
                .await?;

            headless.set_cursor(Point2D::new(300., 300.));
            headless.left_click();

            headless
                .screenshot(Size2D::new(480, 320), theme, None)
                .await?
                .to_rgb8()
                .save(crate::harness::snapshot_path(
                    "widget-explorer",
                    &format!("Borders-{:?}.png", theme),
                )?)
                .unwrap();
        }
        Ok(())
    }
}
