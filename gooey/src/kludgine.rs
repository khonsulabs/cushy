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

pub fn kludgine_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
    initializer: C,
) {
    register_transmogrifiers(&mut transmogrifiers);
    let ui = Gooey::with(transmogrifiers, default_stylesheet(), initializer);
    let ui = Rasterizer::<Kludgine>::new(ui);
    ui.process_widget_messages();

    SingleWindowApplication::run(GooeyWindow { ui });
}

pub fn kludgine_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    initializer: C,
) {
    kludgine_main_with(default_transmogrifiers(), initializer)
}

struct GooeyWindow {
    ui: Rasterizer<Kludgine>,
}

impl WindowCreator for GooeyWindow {
    fn window_title() -> String {
        "Gooey - Kludgine".to_owned()
    }
}

impl Window for GooeyWindow {
    fn render(&mut self, scene: &Target) -> kludgine::Result<()> {
        self.ui.render(Kludgine::from(scene));
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
                position: position.map(|p| p.cast_unit()),
            },
            Event::MouseWheel { delta, touch_phase } => {
                GooeyInputEvent::MouseWheel { delta, touch_phase }
            }
        };
        let result = self
            .ui
            .handle_event(gooey_rasterizer::events::WindowEvent::Input(input));
        self.ui.process_widget_messages();
        if result.needs_redraw || self.ui.needs_redraw() {
            status.set_needs_redraw();
        }
        Ok(())
    }
}
