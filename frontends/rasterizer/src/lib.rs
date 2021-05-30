use std::{collections::HashSet, sync::Arc};

use events::{InputEvent, WindowEvent};
use gooey_core::{
    euclid::{Point2D, Rect},
    styles::{style_sheet::StyleSheet, Style},
    Gooey, Points, WidgetId,
};

pub const CONTROL_CLASS: &str = "gooey-widgets.control";
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase, VirtualKeyCode,
};

mod context;
pub mod events;
mod state;
mod transmogrifier;

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use state::State;
pub use winit;

pub use self::{context::*, transmogrifier::*};

#[derive(Debug)]
pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    pub theme: Arc<StyleSheet>,
    state: State,
    renderer: Option<R>,
}

impl<R: Renderer> Clone for Rasterizer<R> {
    /// This implementation ignores the `renderer` field, as it's temporary
    /// state only used during the render method. It shouldn't ever be accessed
    /// outside of that context.
    fn clone(&self) -> Self {
        Self {
            ui: self.ui.clone(),
            state: self.state.clone(),
            theme: self.theme.clone(),
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
}

impl<R: Renderer> Rasterizer<R> {
    pub fn new(ui: Gooey<Self>, stylesheet: StyleSheet) -> Self {
        Self {
            theme: Arc::new(stylesheet.merge_with(ui.stylesheet())),
            ui: Arc::new(ui),
            state: State::default(),
            renderer: None,
        }
    }

    pub fn render(&self, scene: R) {
        self.state.new_frame();
        let size = scene.size();

        Rasterizer {
            ui: self.ui.clone(),
            state: self.state.clone(),
            theme: self.theme.clone(),
            renderer: Some(scene),
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
            theme: self.theme.clone(),
            renderer: Some(renderer.clip_to(clip)),
        })
    }

    pub fn handle_event(&self, event: WindowEvent) -> EventResult {
        match event {
            WindowEvent::Input(input_event) => match input_event {
                InputEvent::Keyboard {
                    scancode,
                    key,
                    state,
                } => self.handle_keyboard_input(scancode, key, state),
                InputEvent::MouseButton { button, state } => self.handle_mouse_input(state, button),
                InputEvent::MouseMoved { position } => self.handle_cursor_moved(position),
                InputEvent::MouseWheel { delta, touch_phase } =>
                    self.handle_mouse_wheel(delta, touch_phase),
            },
            WindowEvent::RedrawRequested => EventResult::redraw(),
            WindowEvent::ReceiveCharacter(_) => EventResult::ignored(),
            WindowEvent::ModifiersChanged(_) => EventResult::ignored(),
            WindowEvent::LayerChanged { .. } => EventResult::ignored(),
            WindowEvent::SystemThemeChanged(_) => EventResult::ignored(),
        }
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

    fn invoke_drag_events(&self, _position: Option<Point2D<f32, Points>>) {
        // TODO
    }

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
                    .widget_bounds(id)
                    .and_then(|bounds| {
                        self.with_transmogrifier(id, |transmogrifier, mut context| {
                            transmogrifier.mouse_move(&mut context, position, bounds.size)
                        })
                    })
                    .unwrap_or_default()
            })
            .collect::<HashSet<_>>();

        for (button, handler) in self.state.mouse_button_handlers() {
            self.state.widget_bounds(&handler).and_then(|bounds| {
                self.with_transmogrifier(&handler, |transmogrifier, mut context| {
                    transmogrifier.mouse_drag(
                        &mut context,
                        button,
                        position - bounds.origin.to_vector(),
                        bounds.size,
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
                if let Some(bounds) = self.state.widget_bounds(&hovered) {
                    let handled = self
                        .with_transmogrifier(&hovered, |transmogrifier, mut context| {
                            let position = last_mouse_position - bounds.origin.to_vector();
                            let hit = transmogrifier.hit_test(&mut context, position, bounds.size);
                            let handled = hit
                                && transmogrifier.mouse_down(
                                    &mut context,
                                    button,
                                    position,
                                    bounds.size,
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
            .take_mouse_button_handler(&button)
            .and_then(|handler| {
                self.state.widget_bounds(&handler).and_then(|bounds| {
                    self.with_transmogrifier(&handler, |transmogrifier, mut context| {
                        transmogrifier.mouse_up(
                            &mut context,
                            button,
                            self.state
                                .last_mouse_position()
                                .map(|pos| pos - bounds.origin.to_vector()),
                            bounds.size,
                        );
                        EventResult::processed()
                    })
                })
            })
            .unwrap_or_else(EventResult::ignored)
    }

    fn handle_mouse_wheel(&self, _delta: MouseScrollDelta, _phase: TouchPhase) -> EventResult {
        // TODO forward mouse wheel events
        EventResult::ignored()
    }

    pub fn renderer(&self) -> Option<&R> {
        self.renderer.as_ref()
    }

    pub fn rasterizerd_widget(&self, widget: WidgetId, bounds: Rect<f32, Points>) {
        self.state.widget_rendered(widget, bounds);
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn with_transmogrifier<
        TResult,
        C: FnOnce(&'_ dyn AnyWidgetRasterizer<R>, AnyRasterContext<'_, R>) -> TResult,
    >(
        &self,
        widget_id: &WidgetId,
        callback: C,
    ) -> Option<TResult> {
        self.ui
            .with_transmogrifier(widget_id, self, |transmogrifier, state, widget| {
                let widget_state = self.ui.widget_state(widget_id.id).unwrap();
                let style = widget_state.style.lock().unwrap();
                callback(
                    transmogrifier.as_ref(),
                    AnyRasterContext::new(
                        widget_state.registration().unwrap(),
                        state,
                        self,
                        widget,
                        &style,
                        &self.state.ui_state_for(widget_id),
                    ),
                )
            })
    }

    pub fn set_needs_redraw(&self) {
        self.state.set_needs_redraw();
    }

    pub fn needs_redraw(&self) -> bool {
        self.state.needs_redraw()
    }
}

pub struct EventResult {
    pub status: EventStatus,
    pub needs_redraw: bool,
}

impl EventResult {
    pub const fn ignored() -> Self {
        Self {
            status: EventStatus::Ignored,
            needs_redraw: false,
        }
    }

    pub const fn processed() -> Self {
        Self {
            status: EventStatus::Processed,
            needs_redraw: false,
        }
    }

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
