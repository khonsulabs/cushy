use std::fmt::Debug;

use figures::units::{Px, UPx};
use figures::{Point, Size};
use intentional::Cast;
use kludgine::app::winit::event::{DeviceId, ElementState, KeyEvent, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
use kludgine::tilemap;
use kludgine::tilemap::TileMapFocus;

use crate::context::{EventContext, GraphicsContext, LayoutContext};
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
            focus: Value::default(),
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
        // TODO this needs to be updated to support being placed in side of a scroll view.
        let redraw_after = match &mut self.layers {
            Value::Constant(layers) => tilemap::draw(
                layers,
                focus,
                self.zoom,
                context.elapsed(),
                context.gfx.inner_graphics(),
            ),
            Value::Dynamic(layers) => {
                let mut layers = layers.lock();
                layers.prevent_notifications();
                tilemap::draw(
                    &mut *layers,
                    focus,
                    self.zoom,
                    context.elapsed(),
                    context.gfx.inner_graphics(),
                )
            }
        };

        context.draw_focus_ring();

        if let Some(tick) = &self.tick {
            // When we are driven by a tick, we ignore all other sources of
            // refreshes.
            tick.rendered(context);
        } else {
            if let Some(redraw_after) = redraw_after {
                context.redraw_in(redraw_after);
            }
            self.focus.redraw_when_changed(context);
            self.layers.redraw_when_changed(context);
        }
    }

    fn accept_focus(&mut self, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn hit_test(
        &mut self,
        _location: figures::Point<figures::units::Px>,
        _context: &mut EventContext<'_, '_>,
    ) -> bool {
        true
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        _context: &mut LayoutContext<'_, '_, '_, '_, '_>,
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
            MouseScrollDelta::PixelDelta(px) => px.y.cast::<f32>() / 16.0,
        };

        self.zoom += self.zoom * 0.1 * amount;

        context.set_needs_redraw();
        HANDLED
    }

    fn hover(
        &mut self,
        local: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Option<CursorIcon> {
        if let Some(tick) = &self.tick {
            let Some(size) = context.last_layout().map(|rect| rect.size) else {
                return None;
            };

            let world =
                tilemap::translate_coordinates(local, context.kludgine.scale(), self.zoom, size);
            let offset = self
                .layers
                .map(|layers| self.focus.get().world_coordinate(layers));

            tick.set_cursor_position(Some(world + offset));
        }

        None
    }

    fn unhover(&mut self, _context: &mut EventContext<'_, '_>) {
        if let Some(tick) = &self.tick {
            tick.set_cursor_position(None);
        }
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

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: DeviceId,
        button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        if let Some(tick) = &self.tick {
            tick.mouse_button(button, ElementState::Pressed);
            context.focus();
            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: DeviceId,
        button: kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        if let Some(tick) = &self.tick {
            tick.mouse_button(button, ElementState::Released);
        }
    }
}
