use std::ops::{Deref, DerefMut};

use kludgine::figures::units::UPx;
use kludgine::figures::Rect;
use kludgine::render::Renderer;
use kludgine::ClipGuard;

/// A 2d graphics context
pub struct Graphics<'clip, 'gfx, 'pass> {
    renderer: RenderContext<'clip, 'gfx, 'pass>,
}

enum RenderContext<'clip, 'gfx, 'pass> {
    Renderer(Renderer<'gfx, 'pass>),
    Clipped(ClipGuard<'clip, Renderer<'gfx, 'pass>>),
}

impl<'clip, 'gfx, 'pass> Graphics<'clip, 'gfx, 'pass> {
    /// Returns a new graphics context for the given [`Renderer`].
    #[must_use]
    pub fn new(renderer: Renderer<'gfx, 'pass>) -> Self {
        Self {
            renderer: RenderContext::Renderer(renderer),
        }
    }

    /// Returns a context that has been clipped to `clip`.
    ///
    /// The new clipping rectangle is interpreted relative to the current
    /// clipping rectangle. As a side effect, this function can never expand the
    /// clipping rect beyond the current clipping rect.
    ///
    /// The returned context will report the clipped size, and all drawing
    /// operations will be relative to the origin of `clip`.
    pub fn clipped_to(&mut self, clip: Rect<UPx>) -> Graphics<'_, 'gfx, 'pass> {
        Graphics {
            renderer: RenderContext::Clipped(self.deref_mut().clipped_to(clip)),
        }
    }
}

impl<'gfx, 'pass> Deref for Graphics<'_, 'gfx, 'pass> {
    type Target = Renderer<'gfx, 'pass>;

    fn deref(&self) -> &Self::Target {
        match &self.renderer {
            RenderContext::Renderer(renderer) => renderer,
            RenderContext::Clipped(clipped) => clipped,
        }
    }
}

impl<'gfx, 'pass> DerefMut for Graphics<'_, 'gfx, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.renderer {
            RenderContext::Renderer(renderer) => renderer,
            RenderContext::Clipped(clipped) => &mut *clipped,
        }
    }
}
