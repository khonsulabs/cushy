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

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use events::{InputEvent, WindowEvent};
use gooey_core::{
    assets::{self, Asset, Configuration, FrontendImage, Image},
    figures::{Point, Rect, Size},
    styles::{
        style_sheet::{self},
        Autofocus, Intent, Style, SystemTheme, TabIndex,
    },
    AnyFrontend, AnyTransmogrifierContext, AnyWindowBuilder, AppContext, Callback, Gooey, Pixels,
    Scaled, Timer, TransmogrifierContext, WidgetId, Window, WindowRef,
};
use image::{ImageFormat, RgbaImage};
use platforms::{target::OS, TARGET_OS};
use timer::ThreadTimer;
use winit::event::{
    ElementState, ModifiersState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase,
    VirtualKeyCode,
};

pub mod events;
mod state;
mod timer;
mod transmogrifier;

#[doc(hidden)]
pub use gooey_renderer::Renderer;
use state::State;
pub use winit;

pub use self::transmogrifier::*;
use crate::state::FocusEvent;

pub type WindowCreator = dyn Fn(AppContext, &mut dyn AnyWindowBuilder) + Send + Sync + 'static;

pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    state: State,
    window_creator: Option<Arc<WindowCreator>>,
    refresh_callback: Option<Arc<dyn RefreshCallback>>,
    renderer: Option<R>,
    window: Option<WindowRef>,
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
            window: self.window.clone(),
            window_creator: self.window_creator.clone(),
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

    fn widget_initialized(&self, widget: &WidgetId, style: &Style) {
        if style.get::<Autofocus>().is_some() {
            self.focus_on(widget);
        }
    }

    fn schedule_timer(&self, callback: Callback, duration: Duration, repeating: bool) -> Timer {
        ThreadTimer::schedule(callback, duration, repeating)
    }

    fn window(&self) -> Option<&dyn gooey_core::Window> {
        self.window.as_deref()
    }

    fn open(&self, mut window: Box<dyn AnyWindowBuilder>) -> bool {
        self.window_creator
            .as_ref()
            .map_or(false, |window_creator| {
                window_creator(self.storage().app().clone(), window.as_mut());
                true
            })
    }
}

fn load_image(image: &Image, data: Vec<u8>) -> Result<(), String> {
    let format = ImageFormat::from_path(image.asset.path().last().unwrap().as_ref())
        .map_err(|err| format!("unknown image format: {:?}", err))?;
    let loaded_image = image::load_from_memory_with_format(&data, format)
        .map_err(|err| format!("error parsing image: {:?}", err))?;
    image.set_data(RasterizerImage(Arc::new(loaded_image.to_rgba8())));
    Ok(())
}

#[derive(Clone, Debug)]
struct RasterizerImage(Arc<RgbaImage>);

impl FrontendImage for RasterizerImage {
    fn size(&self) -> Option<Size<u32, Pixels>> {
        Some(Size::new(self.0.width(), self.0.height()))
    }
}

impl<R: Renderer> Rasterizer<R> {
    #[must_use]
    pub fn new(ui: Gooey<Self>, configuration: Configuration) -> Self {
        Self {
            ui: Arc::new(ui),
            state: State::new(configuration),
            window_creator: None,
            refresh_callback: None,
            renderer: None,
            window: None,
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
            window_creator: self.window_creator.clone(),
            renderer: Some(renderer),
            window: None,
        }
        .with_transmogrifier(self.ui.root_widget().id(), |transmogrifier, mut context| {
            transmogrifier.render_within(&mut context, Rect::from(size), None, &Style::default());
        });
    }

    pub fn clipped_to(&self, clip: Rect<f32, Scaled>) -> Option<Self> {
        self.renderer()
            .map(|renderer| self.with_renderer(renderer.clip_to(clip)))
    }

    pub fn handle_event(&self, event: WindowEvent, renderer: R) -> EventResult {
        let rasterizer = self.with_renderer(renderer);
        let _guard = rasterizer.ui.enter_managed_code(&rasterizer);
        let result = match event {
            WindowEvent::Input(input_event) => match input_event {
                InputEvent::Keyboard {
                    scancode,
                    key,
                    state,
                } => rasterizer.handle_keyboard_input(scancode, key, state),
                InputEvent::MouseButton { button, state } => {
                    rasterizer.handle_mouse_input(state, button)
                }
                InputEvent::MouseMoved { position } => rasterizer.handle_cursor_moved(position),
                InputEvent::MouseWheel { delta, touch_phase } => {
                    rasterizer.handle_mouse_wheel(delta, touch_phase)
                }
            },
            WindowEvent::SystemThemeChanged(theme) => {
                rasterizer.state.set_system_theme(theme);
                EventResult::ignored()
            }
            WindowEvent::RedrawRequested => EventResult::redraw(),
            WindowEvent::ReceiveCharacter(ch) => rasterizer.handle_receive_character(ch),
            WindowEvent::ModifiersChanged(modifiers) => {
                rasterizer.state.set_keyboard_modifiers(modifiers);
                // Report ignored since this isn't an event that we exclusively can handle.
                EventResult::ignored()
            }
            WindowEvent::LayerChanged { .. } => EventResult::ignored(),
        };

        for focus_event in self.state.focus_events() {
            self.with_transmogrifier(focus_event.widget(), |transmogrifier, mut context| {
                match focus_event {
                    FocusEvent::Focused(_) => transmogrifier.focused(&mut context),
                    FocusEvent::Blurred(_) => transmogrifier.blurred(&mut context),
                }
            });
        }

        result
    }

    pub fn set_window_creator<
        F: Fn(AppContext, &mut dyn AnyWindowBuilder) + Send + Sync + 'static,
    >(
        &mut self,
        window_creator: F,
    ) {
        self.window_creator = Some(Arc::new(window_creator));
    }

    pub fn set_window<W: Window>(&mut self, window: W) {
        self.window = Some(Arc::new(window));
    }

    pub fn set_refresh_callback<F: RefreshCallback>(&mut self, callback: F) {
        self.refresh_callback = Some(Arc::new(callback));
    }

    pub fn set_system_theme(&self, theme: SystemTheme) {
        self.state.set_system_theme(theme);
    }

    fn handle_cursor_moved(&self, position: Option<Point<f32, Scaled>>) -> EventResult {
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

    fn with_renderer(&self, renderer: R) -> Self {
        Self {
            ui: self.ui.clone(),
            state: self.state.clone(),
            refresh_callback: self.refresh_callback.clone(),
            renderer: Some(renderer),
            window: self.window.clone(),
            window_creator: self.window_creator.clone(),
        }
    }

    #[allow(clippy::unused_self)] // TODO needs implementing
    fn invoke_drag_events(&self, _position: Option<Point<f32, Scaled>>) {}

    fn handle_keyboard_input(
        &self,
        scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventResult {
        if let Some(widget) = self.focused_widget() {
            // give the widget an opportunity to process the input. If the
            // widget doesn't handle it, pass the event up the render tree.
            let result = self.with_transmogrifier(&widget, |transmogrifier, mut context| {
                transmogrifier.keyboard(&mut context, scancode, keycode, state)
            });
            if matches!(result, Some(EventStatus::Processed)) {
                return EventResult::processed();
            }
        }

        match (keycode, state) {
            (Some(VirtualKeyCode::Tab), ElementState::Pressed) => {
                if self.state.keyboard_modifiers() == ModifiersState::SHIFT {
                    self.reverse_focus();
                    EventResult::processed()
                } else if self.state.keyboard_modifiers().is_empty() {
                    self.advance_focus();
                    EventResult::processed()
                } else {
                    EventResult::ignored()
                }
            }
            (Some(VirtualKeyCode::Return | VirtualKeyCode::NumpadEnter), state) => self
                .state
                .default_widget()
                .and_then(|submit| {
                    self.with_transmogrifier(&submit, |transmogrifier, mut context| {
                        EventResult::from(transmogrifier.keyboard(
                            &mut context,
                            scancode,
                            keycode,
                            state,
                        ))
                    })
                })
                .unwrap_or_else(EventResult::ignored),
            (Some(VirtualKeyCode::Escape), _) => self
                .state
                .cancel_widget()
                .and_then(|submit| {
                    // Give the cancel widget a chance to handle the key
                    self.with_transmogrifier(&submit, |transmogrifier, mut context| {
                        EventResult::from(transmogrifier.keyboard(
                            &mut context,
                            scancode,
                            keycode,
                            state,
                        ))
                    })
                })
                .unwrap_or_else(|| {
                    self.state.focus().map_or_else(EventResult::ignored, |_| {
                        self.blur();
                        EventResult::processed()
                    })
                }),
            _ => EventResult::ignored(),
        }
    }

    fn handle_receive_character(&self, character: char) -> EventResult {
        self.focused_widget()
            .map_or_else(EventResult::ignored, |widget| {
                EventResult::from(
                    self.with_transmogrifier(&widget, |transmogrifier, mut context| {
                        transmogrifier.receive_character(&mut context, character)
                    })
                    .unwrap_or(EventStatus::Ignored),
                )
            })
    }

    pub fn keyboard_modifiers(&self) -> ModifiersState {
        self.state.keyboard_modifiers()
    }

    fn update_hover(&self, position: Point<f32, Scaled>) -> EventResult {
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
                    transmogrifier.mouse_drag(&mut context, button, position, &bounds);
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
                            let hit =
                                transmogrifier.hit_test(&mut context, last_mouse_position, &bounds);
                            let handled = hit
                                && transmogrifier.mouse_down(
                                    &mut context,
                                    button,
                                    last_mouse_position,
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
                            self.state.last_mouse_position(),
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

    pub fn rasterized_widget(
        &self,
        widget: WidgetId,
        area: ContentArea,
        should_accept_focus: bool,
        parent_id: Option<&WidgetId>,
        tab_order: Option<TabIndex>,
        intent: Option<Intent>,
    ) {
        self.state.widget_rendered(
            widget,
            area,
            should_accept_focus,
            parent_id,
            tab_order,
            intent,
        );
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

    pub fn duration_until_next_redraw(&self) -> Option<Duration> {
        self.state.duration_until_next_redraw()
    }

    pub fn schedule_redraw_in(&self, duration: Duration) {
        self.state.schedule_redraw_in(duration);
    }

    pub fn schedule_redraw_at(&self, at: Instant) {
        self.state.schedule_redraw_at(at);
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

    pub fn blur_and_deactivate(&self) {
        self.state.blur_and_deactivate();
    }

    fn advance_focus(&self) {
        if let Some(next) = self.state.next_tab_entry() {
            self.focus_on(&next);
        }
    }

    fn reverse_focus(&self) {
        if let Some(previous) = self.state.previous_tab_entry() {
            self.focus_on(&previous);
        }
    }
}

pub struct EventResult {
    pub status: EventStatus,
    pub needs_redraw: bool,
}

impl From<EventStatus> for EventResult {
    fn from(status: EventStatus) -> Self {
        Self {
            status,
            needs_redraw: false,
        }
    }
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
    fn from_rgba_image(image: RgbaImage) -> Image;
}

impl ImageExt for Image {
    fn as_rgba_image(&self) -> Option<Arc<RgbaImage>> {
        self.map_data(|opt_data| {
            opt_data
                .and_then(|data| data.as_any().downcast_ref::<RasterizerImage>())
                .map(|img| img.0.clone())
        })
    }

    fn from_rgba_image(image: RgbaImage) -> Image {
        let asset = Image::from(Asset::build().finish());
        asset.set_data(RasterizerImage(Arc::new(image)));
        asset
    }
}

pub trait TransmogrifierContextExt {
    fn activate(&self);
    fn deactivate(&self);
    fn is_focused(&self) -> bool;
    fn focus(&self);
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

    fn is_focused(&self) -> bool {
        self.frontend.focused_widget().as_ref() == Some(self.registration.id())
    }

    fn focus(&self) {
        self.frontend.focus_on(self.registration.id());
    }

    fn blur(&self) {
        self.frontend.blur();
    }
}

pub trait ModifiersStateExt {
    /// Returns true if the primary OS modifier key is pressed. On Mac, this is
    /// the Logo key. On all other platforms, this is the control key.
    fn primary(&self) -> bool;
}

impl ModifiersStateExt for ModifiersState {
    fn primary(&self) -> bool {
        if TARGET_OS == OS::MacOS {
            self.logo()
        } else {
            self.ctrl()
        }
    }
}
