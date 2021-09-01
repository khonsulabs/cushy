use std::{self, path::PathBuf, process::Command};

use gooey_core::{assets::Configuration, AppContext, StyledWidget};
use gooey_rasterizer::winit::{event::ModifiersState, window::Theme};
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
    context: AppContext,
) {
    kludgine_run(kludgine_app(transmogrifiers, initializer, context));
}

/// Runs a `Kludgine`-based [`App`](crate::app::App) with the root widget from
/// `initializer`. All widgets from [`gooey::widget`](crate::widgets) will be
/// usable. If you wish to use other widgets, use `browser_main_with` and
/// provide the transmogrifiers for the widgets you wish to use.
pub fn kludgine_main<W: Widget, C: Fn(&WidgetStorage) -> StyledWidget<W>>(
    initializer: C,
    context: AppContext,
) {
    kludgine_main_with(default_transmogrifiers(), &initializer, context);
}

/// Returns an initialized frontend using the root widget returned from `initializer`.
pub fn kludgine_app<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
    initializer: C,
    context: AppContext,
) -> Rasterizer<Kludgine> {
    register_transmogrifiers(&mut transmogrifiers);
    let ui = Gooey::with(transmogrifiers, default_stylesheet(), initializer, context);
    let ui = Rasterizer::<Kludgine>::new(ui, Configuration::default());
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
            gtk3_preferred_theme()
                .or_else(gtk2_theme)
                .unwrap_or(Theme::Light)
        } else {
            Theme::Light
        }
    }
}

fn gtk3_preferred_theme() -> Option<Theme> {
    let settings_path = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config_home)
            .join("gtk-3.window")
            .join("settings.ini")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("gtk-3.window")
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

fn gtk2_theme() -> Option<Theme> {
    let result = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
        .ok()?;
    if result.status.success() {
        let result = String::from_utf8(result.stdout).ok()?;
        // gsettings wraps the output in single quotes
        if result.trim().ends_with("-dark'") {
            Some(Theme::Dark)
        } else {
            Some(Theme::Light)
        }
    } else {
        None
    }
}

impl Window for GooeyWindow {
    fn initialize(
        &mut self,
        _scene: &Target,
        redrawer: RedrawRequester,
        window: WindowHandle,
    ) -> kludgine::Result<()>
    where
        Self: Sized,
    {
        self.redrawer = Some(redrawer.clone());
        self.ui.set_window(KludgineWindow { window });
        self.ui.set_refresh_callback(move || {
            redrawer.awaken();
        });
        Ok(())
    }

    fn render(
        &mut self,
        scene: &Target,
        status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        self.ui.render(Kludgine::from(scene));
        self.ui.gooey().process_widget_messages(&self.ui);
        if self.ui.needs_redraw() {
            status.set_needs_redraw();
        } else if let Some(duration) = self.ui.duration_until_next_redraw() {
            status.estimate_next_frame(duration);
        }

        Ok(())
    }

    fn update(
        &mut self,
        _scene: &Target,
        status: &mut RedrawStatus,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        self.ui.gooey().process_widget_messages(&self.ui);
        if self.ui.needs_redraw() {
            status.set_needs_redraw();
        } else if let Some(duration) = self.ui.duration_until_next_redraw() {
            status.estimate_next_frame(duration);
        }

        Ok(())
    }

    fn process_input(
        &mut self,
        input: InputEvent,
        status: &mut RedrawStatus,
        scene: &Target,
        _window: WindowHandle,
    ) -> kludgine::Result<()> {
        let input = match input.event {
            Event::Keyboard {
                scancode,
                key,
                state,
            } => {
                // When a keyboard event happens, refresh the modifiers.
                let mut modifiers = ModifiersState::default();
                let scene_modifiers = scene.modifiers_pressed();
                if scene_modifiers.alt {
                    modifiers |= ModifiersState::ALT;
                }
                if scene_modifiers.control {
                    modifiers |= ModifiersState::CTRL;
                }
                if scene_modifiers.operating_system {
                    modifiers |= ModifiersState::LOGO;
                }
                if scene_modifiers.shift {
                    modifiers |= ModifiersState::SHIFT;
                }
                self.ui.handle_event(
                    gooey_rasterizer::events::WindowEvent::ModifiersChanged(modifiers),
                    Kludgine::from(scene),
                );

                GooeyInputEvent::Keyboard {
                    scancode,
                    key,
                    state,
                }
            }
            Event::MouseButton { button, state } => GooeyInputEvent::MouseButton { button, state },
            Event::MouseMoved { position } => GooeyInputEvent::MouseMoved {
                position: position.map(|p| p.cast_unit()),
            },
            Event::MouseWheel { delta, touch_phase } => {
                GooeyInputEvent::MouseWheel { delta, touch_phase }
            }
        };
        let result = self.ui.handle_event(
            gooey_rasterizer::events::WindowEvent::Input(input),
            Kludgine::from(scene),
        );
        self.ui.gooey().process_widget_messages(&self.ui);
        if result.needs_redraw || self.ui.needs_redraw() {
            status.set_needs_redraw();
        }
        Ok(())
    }

    fn receive_character(
        &mut self,
        character: char,
        status: &mut RedrawStatus,
        scene: &Target,
        _window: WindowHandle,
    ) -> kludgine::app::Result<()>
    where
        Self: Sized,
    {
        let result = self.ui.handle_event(
            gooey_rasterizer::events::WindowEvent::ReceiveCharacter(character),
            Kludgine::from(scene),
        );
        self.ui.gooey().process_widget_messages(&self.ui);
        if result.needs_redraw || self.ui.needs_redraw() {
            status.set_needs_redraw();
        }
        Ok(())
    }
}

#[derive(Debug)]
struct KludgineWindow {
    window: WindowHandle,
}

impl gooey_core::Window for KludgineWindow {
    fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    fn inner_size(&self) -> gooey_core::figures::Size<u32, gooey_core::Pixels> {
        self.window.inner_size()
    }

    fn set_inner_size(&self, new_size: gooey_core::figures::Size<u32, gooey_core::Pixels>) {
        self.window.set_inner_size(new_size);
    }

    fn set_outer_position(
        &self,
        new_position: gooey_core::figures::Point<i32, gooey_core::Pixels>,
    ) {
        self.window.set_outer_position(new_position);
    }

    fn inner_position(&self) -> gooey_core::figures::Point<i32, gooey_core::Pixels> {
        self.window.inner_position()
    }

    fn set_always_on_top(&self, always: bool) {
        self.window.set_always_on_top(always);
    }

    fn maximized(&self) -> bool {
        self.window.maximized()
    }

    fn set_maximized(&self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    fn set_minimized(&self, minimized: bool) {
        self.window.set_minimized(minimized);
    }

    fn close(&self) {
        self.window.request_close();
    }

    fn outer_position(&self) -> gooey_core::figures::Point<i32, gooey_core::Pixels> {
        self.inner_position()
    }
}
