use std::fmt::Debug;
use std::panic::UnwindSafe;

use kludgine::figures::utils::lossy_f64_to_f32;

use crate::context::{EventContext, GraphicsContext};
use crate::dynamic::Dynamic;
use crate::kludgine::app::winit::event::{DeviceId, KeyEvent, MouseScrollDelta, TouchPhase};
use crate::kludgine::app::winit::keyboard::Key;
use crate::kludgine::figures::units::UPx;
use crate::kludgine::figures::Size;
use crate::kludgine::tilemap;
use crate::kludgine::tilemap::TileMapFocus;
use crate::widget::{Callback, EventHandling, IntoValue, Value, Widget, HANDLED, UNHANDLED};
use crate::ConstraintLimit;

#[derive(Debug)]
#[must_use]
pub struct TileMap<Layers> {
    layers: Value<Layers>,
    focus: Value<TileMapFocus>,
    key: Option<Callback<Key, EventHandling>>,
    zoom: f32,
}

impl<Layers> TileMap<Layers> {
    fn construct(layers: Value<Layers>) -> Self {
        Self {
            layers,
            focus: Value::Constant(TileMapFocus::default()),
            zoom: 1.,
            key: None,
        }
    }

    pub fn dynamic(layers: Dynamic<Layers>) -> Self {
        Self::construct(Value::Dynamic(layers))
    }

    pub fn new(layers: Layers) -> Self {
        Self::construct(Value::Constant(layers))
    }

    pub fn focus_on(mut self, focus: impl IntoValue<TileMapFocus>) -> Self {
        self.focus = focus.into_value();
        self
    }

    pub fn on_key<F>(mut self, key: F) -> Self
    where
        F: FnMut(Key) -> EventHandling + Send + UnwindSafe + 'static,
    {
        self.key = Some(Callback::new(key));
        self
    }
}

impl<Layers> Widget for TileMap<Layers>
where
    Layers: tilemap::Layers,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.focus.redraw_when_changed(context);
        self.layers.redraw_when_changed(context);

        let focus = self.focus.get();
        self.layers
            .map(|layers| tilemap::draw(layers, focus, self.zoom, &mut context.graphics));
    }

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        _context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        Size::new(available_space.width.max(), available_space.height.max())
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let amount = match delta {
            MouseScrollDelta::LineDelta(_, lines) => lines,
            MouseScrollDelta::PixelDelta(px) => lossy_f64_to_f32(px.y) / 16.0,
        };

        self.zoom += self.zoom * 0.1 * amount;

        context.set_needs_redraw();
        HANDLED
    }

    fn keyboard_input(
        &mut self,
        _device_id: DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        if !input.state.is_pressed() {
            return UNHANDLED;
        }
        if let Some(on_key) = &mut self.key {
            on_key.invoke(input.logical_key.clone())?;
        }
        self.focus.map_mut(|focus| {
            if let TileMapFocus::Point(focus) = focus {
                match input.logical_key {
                    Key::ArrowLeft => {
                        focus.x -= 1;
                    }
                    Key::ArrowRight => {
                        focus.x += 1;
                    }
                    Key::ArrowUp => {
                        focus.y -= 1;
                    }
                    Key::ArrowDown => {
                        focus.y += 1;
                    }
                    _ => {}
                }
            }
        });

        context.set_needs_redraw();
        HANDLED
    }
}
