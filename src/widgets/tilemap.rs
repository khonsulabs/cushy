use std::fmt::Debug;

use kludgine::figures::utils::lossy_f64_to_f32;

use crate::context::{EventContext, GraphicsContext};
use crate::kludgine::app::winit::event::{DeviceId, KeyEvent, MouseScrollDelta, TouchPhase};
use crate::kludgine::figures::units::UPx;
use crate::kludgine::figures::Size;
use crate::kludgine::tilemap;
use crate::kludgine::tilemap::TileMapFocus;
use crate::tick::Tick;
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{EventHandling, Widget, HANDLED, IGNORED};
use crate::ConstraintLimit;

/// A layered tile-based 2d game surface.
#[derive(Debug)]
#[must_use]
pub struct TileMap<Layers> {
    layers: Value<Layers>,
    focus: Value<TileMapFocus>,
    zoom: f32,
    tick: Option<Tick>,
}

impl<Layers> TileMap<Layers> {
    fn construct(layers: Value<Layers>) -> Self {
        Self {
            layers,
            focus: Value::Constant(TileMapFocus::default()),
            zoom: 1.,
            tick: None,
        }
    }

    /// Returns a new tilemap that contains dynamic layers.
    pub fn dynamic(layers: Dynamic<Layers>) -> Self {
        Self::construct(Value::Dynamic(layers))
    }

    /// Returns a new tilemap that renders `layers`.
    pub fn new(layers: Layers) -> Self {
        Self::construct(Value::Constant(layers))
    }

    /// Sets the camera's focus and returns self.
    ///
    /// The tilemap will ensure that `focus` is centered.
    // TODO how do we allow the camera to "lag" for juice effects?
    pub fn focus_on(mut self, focus: impl IntoValue<TileMapFocus>) -> Self {
        self.focus = focus.into_value();
        self
    }

    /// Associates a [`Tick`] with this widget and returns self.
    pub fn tick(mut self, tick: Tick) -> Self {
        self.tick = Some(tick);
        self
    }
}

impl<Layers> Widget for TileMap<Layers>
where
    Layers: tilemap::Layers,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let focus = self.focus.get();
        self.layers
            .map(|layers| tilemap::draw(layers, focus, self.zoom, &mut context.graphics));

        if let Some(tick) = &self.tick {
            tick.rendered(context);
        } else {
            self.focus.redraw_when_changed(context);
            self.layers.redraw_when_changed(context);
        }
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
        _context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        if let Some(tick) = &self.tick {
            tick.key_input(&input)?;
        }

        IGNORED
    }
}
