use std::path::PathBuf;

use gooey_core::StyledWidget;
use gooey_rasterizer::winit::window::Theme;
use platforms::target::{OS, TARGET_OS};

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
    transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
    initializer: C,
) {
    kludgine_run(kludgine_app(transmogrifiers, initializer));
}

/// Runs a `Kludgine`-based [`App`](crate::app::App) with the root widget from
/// `initializer`. All widgets from [`gooey::widget`](crate::widgets) will be
/// usable. If you wish to use other widgets, use `browser_main_with` and
/// provide the transmogrifiers for the widgets you wish to use.
pub fn kludgine_main<W: Widget, C: Fn(&WidgetStorage) -> StyledWidget<W>>(initializer: C) {
    kludgine_main_with(default_transmogrifiers(), &initializer);
}

/// Returns an initialized frontend using the root widget returned from `initializer`.
pub fn kludgine_app<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
    initializer: C,
) -> Rasterizer<Kludgine> {
    register_transmogrifiers(&mut transmogrifiers);
    let ui = Gooey::with(transmogrifiers, default_stylesheet(), initializer);
    let ui = Rasterizer::<Kludgine>::new(ui);
    ui.gooey().process_widget_messages(&ui);
    ui
}

/// Runs an initialized frontend.
pub fn kludgine_run(ui: Rasterizer<Kludgine>) {
    SingleWindowApplication::run(GooeyWindow { ui, redrawer: None });
}

struct GooeyWindow {
    ui: Rasterizer<Kludgine>,
    redrawer: Option<RedrawRequester>,
}

impl WindowCreator for GooeyWindow {
    fn window_title() -> String {
        "Gooey - Kludgine".to_owned()
    }

    fn initial_system_theme() -> Theme {
        // winit doesn't have a way on linux to detect dark mode
        if TARGET_OS == OS::Linux {
            gtk3_preferred_theme().unwrap_or(Theme::Light)
        } else {
            Theme::Light
        }
    }
}

fn gtk3_preferred_theme() -> Option<Theme> {
    let settings_path = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config_home)
            .join("gtk-3.0")
            .join("settings.ini")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("gtk-3.0")
            .join("settings.ini")
    } else {
        return None;
    };
    let file_contents = std::fs::read_to_string(&settings_path).ok()?;
    // TODO make this more forgiving to whitespace
    if file_contents.contains("gtk-application-prefer-dark-theme=true") {
        Some(Theme::Dark)
    } else {
        Some(Theme::Light)
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
            Event::MouseWheel { delta, touch_phase } => {
                GooeyInputEvent::MouseWheel { delta, touch_phase }
            }
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
