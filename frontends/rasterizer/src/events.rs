use gooey_core::{
    euclid::{Point2D, Scale},
    styles::{Pixels, Points, SystemTheme},
};
use winit::event::{
    ElementState, ModifiersState, MouseButton, MouseScrollDelta, ScanCode, TouchPhase,
    VirtualKeyCode, WindowEvent as WinitWindowEvent,
};

/// An input Event
#[derive(Copy, Clone, Debug)]
pub enum InputEvent {
    /// A keyboard event
    Keyboard {
        scancode: ScanCode,
        key: Option<VirtualKeyCode>,
        state: ElementState,
    },
    /// A mouse button event
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    /// Mouse cursor event
    MouseMoved {
        position: Option<Point2D<f32, Points>>,
    },
    /// Mouse wheel event
    MouseWheel {
        delta: MouseScrollDelta,
        touch_phase: TouchPhase,
    },
}

#[derive(Debug)]
pub enum WindowEvent {
    Input(InputEvent),
    ReceiveCharacter(char),
    ModifiersChanged(ModifiersState),
    LayerChanged { is_focused: bool },
    RedrawRequested,
    SystemThemeChanged(SystemTheme),
}

impl WindowEvent {
    pub fn from_winit_event(
        value: WinitWindowEvent<'_>,
        scale: Scale<f32, Points, Pixels>,
    ) -> Result<Self, WinitWindowEvent<'_>> {
        match value {
            WinitWindowEvent::ReceivedCharacter(c) => Ok(Self::ReceiveCharacter(c)),
            WinitWindowEvent::Focused(is_focused) => Ok(Self::LayerChanged { is_focused }),
            WinitWindowEvent::KeyboardInput { input, .. } =>
                Ok(Self::Input(InputEvent::Keyboard {
                    key: input.virtual_keycode,
                    scancode: input.scancode,
                    state: input.state,
                })),
            WinitWindowEvent::ModifiersChanged(state) => Ok(Self::ModifiersChanged(state)),
            WinitWindowEvent::CursorMoved { position, .. } =>
                Ok(Self::Input(InputEvent::MouseMoved {
                    position: Some(
                        Point2D::<f64, Pixels>::new(position.x, position.y).to_f32() / scale,
                    ),
                })),
            WinitWindowEvent::CursorLeft { .. } =>
                Ok(Self::Input(InputEvent::MouseMoved { position: None })),
            WinitWindowEvent::MouseWheel { delta, phase, .. } =>
                Ok(Self::Input(InputEvent::MouseWheel {
                    delta,
                    touch_phase: phase,
                })),
            WinitWindowEvent::MouseInput { state, button, .. } =>
                Ok(Self::Input(InputEvent::MouseButton { state, button })),
            WinitWindowEvent::ThemeChanged(theme) => Ok(Self::SystemThemeChanged(match theme {
                winit::window::Theme::Light => SystemTheme::Light,
                winit::window::Theme::Dark => SystemTheme::Dark,
            })),

            // Ignored
            ignored => Err(ignored),
        }
    }
}
