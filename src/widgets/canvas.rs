use std::fmt::Debug;

use figures::Size;

use crate::context::{GraphicsContext, LayoutContext};
use crate::reactive::value::Dynamic;
use crate::widget::{Widget, WidgetLayout};
use crate::{ConstraintLimit, Tick};

/// A 2d drawable surface.
#[must_use]
pub struct Canvas {
    render: Box<dyn RenderFunction>,
    tick: Option<Tick>,
    redraw: Dynamic<()>,
}

impl Canvas {
    /// Returns a new canvas that draws its contents by invoking `render`.
    pub fn new<F>(render: F) -> Self
    where
        F: for<'clip, 'gfx, 'pass, 'context> FnMut(
                &mut GraphicsContext<'context, 'clip, 'gfx, 'pass>,
            ) + Send
            + 'static,
    {
        Self {
            render: Box::new(render),
            tick: None,
            redraw: Dynamic::new(()),
        }
    }

    /// Associates a [`Tick`] with this widget and returns self.
    pub fn tick(mut self, tick: Tick) -> Self {
        self.tick = Some(tick);
        self
    }
}

impl Widget for Canvas {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        context.redraw_when_changed(&self.redraw);
        self.render.render(context);
        if let Some(tick) = &self.tick {
            tick.rendered(context);
        }
    }

    fn layout(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        _context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        available_space.map(ConstraintLimit::max).into()
    }
}

impl Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas").finish_non_exhaustive()
    }
}

trait RenderFunction: Send + 'static {
    fn render(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>);
}

impl<F> RenderFunction for F
where
    F: for<'clip, 'gfx, 'pass, 'context> FnMut(&mut GraphicsContext<'context, 'clip, 'gfx, 'pass>)
        + Send
        + 'static,
{
    fn render(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        self(context);
    }
}
