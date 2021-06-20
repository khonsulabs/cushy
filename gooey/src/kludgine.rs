use gooey_core::StyledWidget;

use crate::{
    core::{Frontend, Gooey, Transmogrifiers, Widget, WidgetStorage},
    frontends::{
        rasterizer::{events::InputEvent as GooeyInputEvent, Rasterizer},
        renderers::kludgine::{
            kludgine::{self, prelude::*},
            Kludgine,
        },
    },
    style::default_stylesheet,
    widgets::rasterized::{default_transmogrifiers, register_transmogrifiers},
};

/// Runs a `Kludgine`-based [`App`](crate::app::App) with `transmogrifiers` and
/// the root widget from `initializer`. Unless overriden by `transmogrifier`,
/// all widgets from [`gooey::widget`](crate::widgets) will use the built-in
/// transmogrifiers.
pub fn kludgine_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
    initializer: C,
) {
    register_transmogrifiers(&mut transmogrifiers);
    let ui = Gooey::with(transmogrifiers, default_stylesheet(), initializer);
    let ui = Rasterizer::<Kludgine>::new(ui);
    ui.gooey().process_widget_messages(&ui);

    SingleWindowApplication::run(GooeyWindow { ui, redrawer: None });
}

/// Runs a `Kludgine`-based [`App`](crate::app::App) with the root widget from
/// `initializer`. All widgets from [`gooey::widget`](crate::widgets) will be
/// usable. If you wish to use other widgets, use `browser_main_with` and
/// provide the transmogrifiers for the widgets you wish to use.
pub fn kludgine_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    initializer: C,
) {
    kludgine_main_with(default_transmogrifiers(), initializer)
}

struct GooeyWindow {
    ui: Rasterizer<Kludgine>,
    redrawer: Option<RedrawRequester>,
}

impl WindowCreator for GooeyWindow {
    fn window_title() -> String {
        "Gooey - Kludgine".to_owned()
    }
}

impl Window for GooeyWindow {
    fn initialize(&mut self, _scene: &Target, redrawer: RedrawRequester) -> kludgine::Result<()>
    where
        Self: Sized,
    {
        self.redrawer = Some(redrawer.clone());
        self.ui.set_refresh_callback(move || {
            redrawer.awaken();
        });
        Ok(())
    }

    fn render(&mut self, scene: &Target) -> kludgine::Result<()> {
        self.ui.render(Kludgine::from(scene));
        self.ui.gooey().process_widget_messages(&self.ui);
        if self.ui.needs_redraw() {
            self.redrawer.as_ref().unwrap().request_redraw();
        }

        Ok(())
    }

    fn update(&mut self, _scene: &Target, status: &mut RedrawStatus) -> kludgine::Result<()> {
        self.ui.gooey().process_widget_messages(&self.ui);
        if self.ui.needs_redraw() {
            status.set_needs_redraw();
        }

        Ok(())
    }

    fn process_input(
        &mut self,
        input: InputEvent,
        status: &mut RedrawStatus,
    ) -> kludgine::Result<()> {
        let input = match input.event {
            Event::Keyboard {
                scancode,
                key,
                state,
            } => GooeyInputEvent::Keyboard {
                scancode,
                key,
                state,
            },
            Event::MouseButton { button, state } => GooeyInputEvent::MouseButton { button, state },
            Event::MouseMoved { position } => GooeyInputEvent::MouseMoved {
                position: position.map(Point::cast_unit),
            },
            Event::MouseWheel { delta, touch_phase } =>
                GooeyInputEvent::MouseWheel { delta, touch_phase },
        };
        let result = self
            .ui
            .handle_event(gooey_rasterizer::events::WindowEvent::Input(input));
        self.ui.gooey().process_widget_messages(&self.ui);
        if result.needs_redraw || self.ui.needs_redraw() {
            status.set_needs_redraw();
        }
        Ok(())
    }
}
