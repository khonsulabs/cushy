//! A [`Frontend`](gooey_core::Frontend) for `Gooey` that rasterizes widgets
//! using a [`Renderer`](gooey_renderer::Renderer).

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // TODO missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::if_not_else,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc, // TODO clippy::missing_errors_doc
    clippy::missing_panics_doc, // TODO clippy::missing_panics_doc
)]
#![cfg_attr(doc, warn(rustdoc::all))]

use std::{collections::HashSet, sync::Arc};

use events::{InputEvent, WindowEvent};
use gooey_core::{
    assets::{self, Configuration, Image},
    euclid::{Point2D, Rect},
    styles::{
        style_sheet::{self},
        Style, SystemTheme,
    },
    AnyTransmogrifierContext, Callback, Gooey, Points, TransmogrifierContext, WidgetId,
};
use image::{ImageFormat, RgbaImage};
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};

pub mod events;
mod state;
mod transmogrifier;

#[doc(hidden)]
pub use gooey_renderer::Renderer;
use state::State;
pub use winit;

pub use self::transmogrifier::*;

pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    state: State,
    refresh_callback: Option<Arc<dyn RefreshCallback>>,
    renderer: Option<R>,
}

impl<R: Renderer> std::fmt::Debug for Rasterizer<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rasterizer")
            .field("ui", &self.ui)
            .field("state", &self.state)
            .field("renderer", &self.renderer)
            .field(
                "refresh_callback",
                &self.refresh_callback.as_ref().map(|_| "installed"),
            )
            .finish()
    }
}

impl<R: Renderer> Clone for Rasterizer<R> {
    /// This implementation ignores the `renderer` field, as it's temporary
    /// state only used during the render method. It shouldn't ever be accessed
    /// outside of that context.
    fn clone(&self) -> Self {
        Self {
            ui: self.ui.clone(),
            state: self.state.clone(),
            refresh_callback: self.refresh_callback.clone(),
            renderer: None,
        }
    }
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyTransmogrifier = RegisteredTransmogrifier<R>;
    type Context = Self;

    fn gooey(&self) -> &'_ Gooey<Self> {
        &self.ui
    }

    fn ui_state_for(&self, widget_id: &WidgetId) -> style_sheet::State {
        self.state.ui_state_for(widget_id)
    }

    fn set_widget_has_messages(&self, widget: WidgetId) {
        self.gooey().set_widget_has_messages(widget);
        // If we're not inside of a render
        if !self.gooey().is_managed_code() {
            self.gooey().process_widget_messages(self);
        }
    }

    fn exit_managed_code(&self) {
        if self.needs_redraw() {
            if let Some(callback) = &self.refresh_callback {
                callback.refresh();
            }
        }
    }

    fn theme(&self) -> SystemTheme {
        self.state.system_theme()
    }

    fn load_image(&self, image: &Image, completed: Callback<Image>, error: Callback<String>) {
        let image = image.clone();
        let mut asset_path = self.state.assets_path();
        // TODO load this in a separate dedicated thread or async if enabled -- but we don't know about async at this layer
        // spawning a thread to make this happen asynchronously
        std::thread::spawn(move || {
            for part in image.asset.path() {
                asset_path = asset_path.join(part.as_ref());
            }
            match std::fs::read(asset_path) {
                Ok(data) => match load_image(&image, data) {
                    Ok(_) => {
                        completed.invoke(image);
                    }
                    Err(err) => {
                        error.invoke(err);
                    }
                },
                // TODO fallback to HTTP if the file can't be found
                Err(err) => {
                    error.invoke(format!("io error: {:?}", err));
                }
            };
        });
    }

    fn asset_configuration(&self) -> &assets::Configuration {
        self.state.configuration()
    }
}

fn load_image(image: &Image, data: Vec<u8>) -> Result<(), String> {
    let format = ImageFormat::from_path(image.asset.path().last().unwrap().as_ref())
        .map_err(|err| format!("unknown image format: {:?}", err))?;
    let loaded_image = image::load_from_memory_with_format(&data, format)
        .map_err(|err| format!("error parsing image: {:?}", err))?;
    image.set_data(Arc::new(loaded_image.to_rgba8()));
    Ok(())
}

impl<R: Renderer> Rasterizer<R> {
    #[must_use]
    pub fn new(ui: Gooey<Self>, configuration: Configuration) -> Self {
        Self {
            ui: Arc::new(ui),
            state: State::new(configuration),
            refresh_callback: None,
            renderer: None,
        }
    }

    pub fn render(&self, renderer: R) {
        let _guard = self.ui.enter_managed_code(self);
        // Process messages after new_frame,
        self.ui.process_widget_messages(self);

        self.state.new_frame();

        let size = renderer.size();

        Self {
            ui: self.ui.clone(),
            state: self.state.clone(),
            refresh_callback: self.refresh_callback.clone(),
            renderer: Some(renderer),
        }
        .with_transmogrifier(self.ui.root_widget().id(), |transmogrifier, mut context| {
            transmogrifier.render_within(
                &mut context,
                Rect::new(Point2D::default(), size),
                &Style::default(),
            );
        });
    }

    pub fn clipped_to(&self, clip: Rect<f32, Points>) -> Option<Self> {
        self.renderer().map(|renderer| Self {
            ui: self.ui.clone(),
            state: self.state.clone(),
            refresh_callback: self.refresh_callback.clone(),
            renderer: Some(renderer.clip_to(clip)),
        })
    }

    pub fn handle_event(&mut self, event: WindowEvent) -> EventResult {
        let _guard = self.ui.enter_managed_code(self);
        match event {
            WindowEvent::Input(input_event) => match input_event {
                InputEvent::Keyboard {
                    scancode,
                    key,
                    state,
                } => self.handle_keyboard_input(scancode, key, state),
                InputEvent::MouseButton { button, state } => self.handle_mouse_input(state, button),
                InputEvent::MouseMoved { position } => self.handle_cursor_moved(position),
                InputEvent::MouseWheel { delta, touch_phase } => {
                    self.handle_mouse_wheel(delta, touch_phase)
                }
            },
            WindowEvent::SystemThemeChanged(theme) => {
                self.state.set_system_theme(theme);
                EventResult::ignored()
            }
            WindowEvent::RedrawRequested => EventResult::redraw(),
            WindowEvent::ReceiveCharacter(_)
            | WindowEvent::ModifiersChanged(_)
            | WindowEvent::LayerChanged { .. } => EventResult::ignored(),
        }
    }

    pub fn set_refresh_callback<F: RefreshCallback>(&mut self, callback: F) {
        self.refresh_callback = Some(Arc::new(callback));
    }

    pub fn set_system_theme(&self, theme: SystemTheme) {
        self.state.set_system_theme(theme);
    }

    fn handle_cursor_moved(&self, position: Option<Point2D<f32, Points>>) -> EventResult {
        self.state.set_last_mouse_position(position);
        self.invoke_drag_events(position);
        if let Some(position) = position {
            self.update_hover(position)
        } else if self.state.clear_hover().is_empty() {
            EventResult::processed()
        } else {
            EventResult::redraw()
        }
    }

    #[allow(clippy::unused_self)] // TODO needs implementing
    fn invoke_drag_events(&self, _position: Option<Point2D<f32, Points>>) {}

    #[allow(clippy::unused_self)] // TODO needs implementing
    fn handle_keyboard_input(
        &self,
        _scancode: ScanCode,
        _keycode: Option<VirtualKeyCode>,
        _state: ElementState,
    ) -> EventResult {
        EventResult::ignored()
    }

    fn update_hover(&self, position: Point2D<f32, Points>) -> EventResult {
        let new_hover = self
            .state
            .widgets_under_point(position)
            .into_iter()
            .filter(|id| {
                self.state
                    .widget_area(id)
                    .and_then(|bounds| {
                        self.with_transmogrifier(id, |transmogrifier, mut context| {
                            transmogrifier.mouse_move(&mut context, position, &bounds)
                        })
                    })
                    .unwrap_or_default()
            })
            .collect::<HashSet<_>>();

        for (button, handler) in self.state.mouse_button_handlers() {
            self.state.widget_area(&handler).and_then(|bounds| {
                self.with_transmogrifier(&handler, |transmogrifier, mut context| {
                    transmogrifier.mouse_drag(
                        &mut context,
                        button,
                        position - bounds.location.to_vector(),
                        &bounds,
                    );
                })
            });
        }

        let last_hover = self.state.hover();
        if new_hover != last_hover {
            for unhovered_id in last_hover.difference(&new_hover) {
                self.with_transmogrifier(unhovered_id, |transmogrifier, mut context| {
                    transmogrifier.unhovered(&mut context);
                });
            }
            for newly_hovered_id in new_hover.difference(&last_hover) {
                self.with_transmogrifier(newly_hovered_id, |transmogrifier, mut context| {
                    transmogrifier.hovered(&mut context);
                });
            }
            self.state.set_hover(new_hover);
            EventResult::redraw()
        } else {
            EventResult::processed()
        }
    }

    fn handle_mouse_input(&self, state: ElementState, button: MouseButton) -> EventResult {
        match state {
            ElementState::Pressed => self.handle_mouse_down(button),
            ElementState::Released => self.handle_mouse_up(button),
        }
    }

    fn handle_mouse_down(&self, button: MouseButton) -> EventResult {
        self.state.blur();

        if let Some(last_mouse_position) = self.state.last_mouse_position() {
            for hovered in self.state.hover() {
                if let Some(bounds) = self.state.widget_area(&hovered) {
                    let handled = self
                        .with_transmogrifier(&hovered, |transmogrifier, mut context| {
                            let position = last_mouse_position - bounds.location.to_vector();
                            let hit = transmogrifier.hit_test(&mut context, position, &bounds);
                            let handled = hit
                                && transmogrifier.mouse_down(
                                    &mut context,
                                    button,
                                    position,
                                    &bounds,
                                ) == EventStatus::Processed;
                            if handled {
                                self.state.register_mouse_handler(button, hovered.clone());
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or_default();

                    if handled {
                        return EventResult::processed();
                    }
                }
            }
        }

        EventResult::ignored()
    }

    fn handle_mouse_up(&self, button: MouseButton) -> EventResult {
        self.state
            .take_mouse_button_handler(button)
            .and_then(|handler| {
                self.state.widget_area(&handler).and_then(|bounds| {
                    self.with_transmogrifier(&handler, |transmogrifier, mut context| {
                        transmogrifier.mouse_up(
                            &mut context,
                            button,
                            self.state
                                .last_mouse_position()
                                .map(|pos| pos - bounds.location.to_vector()),
                            &bounds,
                        );
                        EventResult::processed()
                    })
                })
            })
            .unwrap_or_else(EventResult::ignored)
    }

    #[allow(clippy::unused_self)] // TODO needs implementing
    fn handle_mouse_wheel(&self, _delta: MouseScrollDelta, _phase: TouchPhase) -> EventResult {
        // TODO forward mouse wheel events
        EventResult::ignored()
    }

    pub fn renderer(&self) -> Option<&R> {
        self.renderer.as_ref()
    }

    pub fn rasterized_widget(&self, widget: WidgetId, area: ContentArea) {
        self.state.widget_rendered(widget, area);
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn with_transmogrifier<
        TResult,
        C: FnOnce(&'_ dyn AnyWidgetRasterizer<R>, AnyTransmogrifierContext<'_, Self>) -> TResult,
    >(
        &self,
        widget_id: &WidgetId,
        callback: C,
    ) -> Option<TResult> {
        self.ui
            .with_transmogrifier(widget_id, self, |transmogrifier, context| {
                callback(transmogrifier.as_ref(), context)
            })
    }

    pub fn set_needs_redraw(&self) {
        self.state.set_needs_redraw();
        if !self.ui.is_managed_code() {
            if let Some(callback) = &self.refresh_callback {
                callback.refresh();
            }
        }
    }

    pub fn needs_redraw(&self) -> bool {
        self.state.needs_redraw()
    }

    pub fn activate(&self, widget: &WidgetId) {
        self.state.set_active(Some(widget.clone()));
    }

    pub fn deactivate(&self) {
        self.state.set_active(None);
    }

    pub fn active_widget(&self) -> Option<WidgetId> {
        self.state.active()
    }

    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.state.focus()
    }

    pub fn focus_on(&self, widget: &WidgetId) {
        self.state.set_focus(Some(widget.clone()));
    }

    pub fn blur(&self) {
        self.state.blur();
    }
}

pub struct EventResult {
    pub status: EventStatus,
    pub needs_redraw: bool,
}

impl EventResult {
    #[must_use]
    pub const fn ignored() -> Self {
        Self {
            status: EventStatus::Ignored,
            needs_redraw: false,
        }
    }

    #[must_use]
    pub const fn processed() -> Self {
        Self {
            status: EventStatus::Processed,
            needs_redraw: false,
        }
    }

    #[must_use]
    pub const fn redraw() -> Self {
        Self {
            status: EventStatus::Processed,
            needs_redraw: true,
        }
    }
}

impl std::ops::Add for EventResult {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl std::ops::AddAssign for EventResult {
    fn add_assign(&mut self, rhs: Self) {
        self.needs_redraw = self.needs_redraw || rhs.needs_redraw;
        self.status = if matches!(self.status, EventStatus::Processed)
            || matches!(rhs.status, EventStatus::Processed)
        {
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        };
    }
}

pub trait RefreshCallback: Send + Sync + 'static {
    fn refresh(&self);
}

impl<T> RefreshCallback for T
where
    T: Fn() + Send + Sync + 'static,
{
    fn refresh(&self) {
        self();
    }
}

pub trait ImageExt {
    fn as_rgba_image(&self) -> Option<Arc<RgbaImage>>;
}

impl ImageExt for Image {
    fn as_rgba_image(&self) -> Option<Arc<RgbaImage>> {
        self.map_data(|opt_data| {
            opt_data
                .and_then(|data| data.as_any().downcast_ref::<Arc<RgbaImage>>())
                .map(Arc::clone)
        })
    }
}

pub trait TransmogrifierContextExt {
    fn activate(&self);
    fn focus(&self);
    fn is_focused(&self) -> bool;
    fn deactivate(&self);
    fn blur(&self);
}

impl<'a, T: WidgetRasterizer<R>, R: Renderer> TransmogrifierContextExt
    for TransmogrifierContext<'a, T, Rasterizer<R>>
{
    fn activate(&self) {
        self.frontend.activate(self.registration.id());
    }

    fn deactivate(&self) {
        self.frontend.deactivate();
    }

    fn focus(&self) {
        self.frontend.focus_on(self.registration.id());
    }

    fn blur(&self) {
        self.frontend.blur();
    }

    fn is_focused(&self) -> bool {
        self.frontend.focused_widget().as_ref() == Some(self.registration.id())
    }
}
