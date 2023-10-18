use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, UnwindSafe};

use kludgine::app::winit::dpi::PhysicalPosition;
use kludgine::app::winit::error::EventLoopError;
use kludgine::app::winit::event::{DeviceId, ElementState, MouseButton};
use kludgine::app::winit::keyboard::KeyCode;
use kludgine::app::WindowBehavior as _;
use kludgine::figures::units::Px;
use kludgine::figures::Point;
use kludgine::render::Drawing;

use crate::context::Context;
use crate::graphics::Graphics;
use crate::tree::{ManagedWidget, Tree};
use crate::utils::ModifiersExt;
use crate::widget::{EventHandling, HANDLED, UNHANDLED};
use crate::window::sealed::WindowCommand;

pub type RunningWindow<'window> = kludgine::app::Window<'window, WindowCommand>;
pub type WindowAttributes = kludgine::app::WindowAttributes<WindowCommand>;

pub struct Window<Behavior>
where
    Behavior: WindowBehavior,
{
    context: Behavior::Context,
    pub attributes: WindowAttributes,
}

impl<Behavior> Default for Window<Behavior>
where
    Behavior: WindowBehavior,
    Behavior::Context: Default,
{
    fn default() -> Self {
        let context = Behavior::Context::default();
        Self::new(context)
    }
}

impl<Behavior> Window<Behavior>
where
    Behavior: WindowBehavior,
{
    pub fn new(context: Behavior::Context) -> Self {
        Self {
            attributes: WindowAttributes::default(),
            context,
        }
    }

    pub fn run(self) -> Result<(), EventLoopError> {
        GooeyWindow::<Behavior>::run_with(AssertUnwindSafe((
            self.context,
            RefCell::new(Some(self.attributes)),
        )))
    }
}

pub trait WindowBehavior: Sized + 'static {
    type Context: UnwindSafe + Send + 'static;

    fn initialize(window: &mut RunningWindow<'_>, context: Self::Context) -> Self;

    fn make_root(&mut self, tree: &Tree) -> ManagedWidget;

    #[allow(unused_variables)]
    fn close_requested(&self, window: &mut RunningWindow<'_>) -> bool {
        true
    }

    fn run() -> Result<(), EventLoopError>
    where
        Self::Context: Default,
    {
        Self::run_with(<Self::Context>::default())
    }

    fn run_with(context: Self::Context) -> Result<(), EventLoopError> {
        GooeyWindow::<Self>::run_with(AssertUnwindSafe((
            context,
            RefCell::new(Some(WindowAttributes {
                title: String::from("Gooey Application"),
                ..WindowAttributes::default()
            })),
        )))
    }
}

struct GooeyWindow<T> {
    behavior: T,
    root: ManagedWidget,
    contents: Drawing,
    should_close: bool,
    mouse_state: MouseState,
}

impl<T> GooeyWindow<T>
where
    T: WindowBehavior,
{
    fn request_close(&mut self, window: &mut RunningWindow<'_>) -> bool {
        self.should_close |= self.behavior.close_requested(window);

        self.should_close
    }
}

impl<T> kludgine::app::WindowBehavior<WindowCommand> for GooeyWindow<T>
where
    T: WindowBehavior,
{
    type Context = AssertUnwindSafe<(T::Context, RefCell<Option<WindowAttributes>>)>;

    fn initialize(
        mut window: RunningWindow<'_>,
        _graphics: &mut kludgine::Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        let mut behavior = T::initialize(&mut window, context.0 .0);
        let root = behavior.make_root(&Tree::default());
        Self {
            behavior,
            root,
            contents: Drawing::default(),
            should_close: false,
            mouse_state: MouseState {
                location: None,
                widget: None,
                devices: HashMap::default(),
            },
        }
    }

    fn prepare(&mut self, mut window: RunningWindow<'_>, graphics: &mut kludgine::Graphics<'_>) {
        graphics.reset_text_attributes();
        self.root.tree.reset_render_order();
        let graphics = self.contents.new_frame(graphics);
        let mut context = Context::new(&self.root, &mut window);
        context.redraw(&mut Graphics::new(graphics));
    }

    fn render<'pass>(
        &'pass mut self,
        _window: RunningWindow<'_>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.contents.render(graphics);

        !self.should_close
    }

    fn initial_window_attributes(context: &Self::Context) -> WindowAttributes {
        context
            .1
            .borrow_mut()
            .take()
            .expect("called more than once")
    }

    fn close_requested(&mut self, mut window: RunningWindow<'_>) -> bool {
        self.request_close(&mut window)
    }

    // fn power_preference() -> wgpu::PowerPreference {
    //     wgpu::PowerPreference::default()
    // }

    // fn limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
    //     wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    // }

    // fn clear_color() -> Option<kludgine::Color> {
    //     Some(kludgine::Color::BLACK)
    // }

    // fn focus_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn occlusion_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn scale_factor_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn resized(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn theme_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn dropped_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file_cancelled(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn received_character(&mut self, window: kludgine::app::Window<'_, ()>, char: char) {}

    fn keyboard_input(
        &mut self,
        mut window: RunningWindow<'_>,
        _device_id: DeviceId,
        input: kludgine::app::winit::event::KeyEvent,
        _is_synthetic: bool,
    ) {
        if !input.state.is_pressed() {
            match input.physical_key {
                KeyCode::KeyW if window.modifiers().state().primary() => {
                    if self.request_close(&mut window) {
                        window.set_needs_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    // fn modifiers_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn ime(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     ime: kludgine::app::winit::event::Ime,
    // ) {
    // }

    fn cursor_moved(
        &mut self,
        mut window: RunningWindow<'_>,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        let location = Point::<Px>::from(position);
        self.mouse_state.location = Some(location);

        if let Some(state) = self.mouse_state.devices.get(&device_id) {
            // Mouse Drag
            for (button, handler) in state {
                let mut context = Context::new(handler, &mut window);
                let last_rendered_at = context.last_rendered_at().expect("passed hit test");
                context.mouse_drag(location - last_rendered_at.origin, device_id, *button);
            }
        } else {
            // Hover
            let mut context = Context::new(&self.root, &mut window);
            for widget in self.root.tree.widgets_at_point(location) {
                let mut widget_context = context.for_other(&widget);
                let relative = location
                    - widget_context
                        .last_rendered_at()
                        .expect("passed hit test")
                        .origin;

                if widget_context.hit_test(relative) {
                    widget_context.hover(relative);
                    drop(widget_context);
                    self.mouse_state.widget = Some(widget);
                    break;
                }
            }
        }
    }

    // fn cursor_entered(
    //     &mut self,
    //     window: RunningWindow<'_>,
    //     device_id: DeviceId,
    // ) {
    // }

    fn cursor_left(&mut self, mut window: RunningWindow<'_>, _device_id: DeviceId) {
        if self.mouse_state.widget.take().is_some() {
            let mut context = Context::new(&self.root, &mut window);
            context.clear_hover();
        }
    }

    // fn mouse_wheel(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    //     delta: kludgine::app::winit::event::MouseScrollDelta,
    //     phase: kludgine::app::winit::event::TouchPhase,
    // ) {
    // }

    fn mouse_input(
        &mut self,
        mut window: RunningWindow<'_>,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        match state {
            ElementState::Pressed => {
                Context::new(&self.root, &mut window).clear_focus();

                if let (ElementState::Pressed, Some(location), Some(hovered)) =
                    (state, &self.mouse_state.location, &self.mouse_state.widget)
                {
                    if let Some(handler) = recursively_handle_event(
                        &mut Context::new(hovered, &mut window),
                        |context| {
                            let relative = *location
                                - context.last_rendered_at().expect("passed hit test").origin;
                            context.mouse_down(relative, device_id, button)
                        },
                    ) {
                        self.mouse_state
                            .devices
                            .entry(device_id)
                            .or_default()
                            .insert(button, handler);
                    }
                }
            }
            ElementState::Released => {
                let Some(device_buttons) = self.mouse_state.devices.get_mut(&device_id) else {
                    return;
                };
                let Some(handler) = device_buttons.remove(&button) else {
                    return;
                };
                if device_buttons.is_empty() {
                    self.mouse_state.devices.remove(&device_id);
                }

                let mut context = Context::new(&handler, &mut window);

                let relative = if let (Some(last_rendered), Some(location)) =
                    (context.last_rendered_at(), self.mouse_state.location)
                {
                    Some(location - last_rendered.origin)
                } else {
                    None
                };

                context.mouse_up(relative, device_id, button);
            }
        }
    }

    // fn touchpad_pressure(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    //     pressure: f32,
    //     stage: i64,
    // ) {
    // }

    // fn axis_motion(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    //     axis: kludgine::app::winit::event::AxisId,
    //     value: f64,
    // ) {
    // }

    // fn touch(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     touch: kludgine::app::winit::event::Touch,
    // ) {
    // }

    // fn touchpad_magnify(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    //     delta: f64,
    //     phase: kludgine::app::winit::event::TouchPhase,
    // ) {
    // }

    // fn smart_magnify(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    // ) {
    // }

    // fn touchpad_rotate(
    //     &mut self,
    //     window: kludgine::app::Window<'_, ()>,
    //     device_id: kludgine::app::winit::event::DeviceId,
    //     delta: f32,
    //     phase: kludgine::app::winit::event::TouchPhase,
    // ) {
    // }

    fn event(&mut self, event: WindowCommand, mut window: RunningWindow<'_>) {
        match event {
            WindowCommand::Redraw => {
                window.set_needs_redraw();
            }
        }
    }
}

fn recursively_handle_event(
    context: &mut Context<'_, '_>,
    mut each_widget: impl FnMut(&mut Context<'_, '_>) -> EventHandling,
) -> Option<ManagedWidget> {
    match each_widget(context) {
        HANDLED => Some(context.widget().clone()),
        UNHANDLED => context.parent().and_then(|parent| {
            recursively_handle_event(&mut context.for_other(&parent), each_widget)
        }),
    }
}

#[derive(Default)]
struct MouseState {
    location: Option<Point<Px>>,
    widget: Option<ManagedWidget>,
    devices: HashMap<DeviceId, HashMap<MouseButton, ManagedWidget>>,
}

pub(crate) mod sealed {
    pub enum WindowCommand {
        Redraw,
        // RequestClose,
    }
}
