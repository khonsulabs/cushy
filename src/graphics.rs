use std::ops::{Deref, DerefMut};

use kludgine::figures::units::UPx;
use kludgine::figures::Rect;

pub struct Graphics<'clip, 'gfx, 'pass> {
    renderer: GraphicsContext<'clip, 'gfx, 'pass>,
}

enum GraphicsContext<'clip, 'gfx, 'pass> {
    Renderer(kludgine::render::Renderer<'gfx, 'pass>),
    Clipped(kludgine::ClipGuard<'clip, kludgine::render::Renderer<'gfx, 'pass>>),
}

impl<'clip, 'gfx, 'pass> Graphics<'clip, 'gfx, 'pass> {
    #[must_use]
    pub fn new(renderer: kludgine::render::Renderer<'gfx, 'pass>) -> Self {
        Self {
            renderer: GraphicsContext::Renderer(renderer),
        }
    }

    pub fn clipped_to(&mut self, clip: Rect<UPx>) -> Graphics<'_, 'gfx, 'pass> {
        Graphics {
            renderer: GraphicsContext::Clipped(self.deref_mut().clipped_to(clip)),
        }
    }
}

impl<'gfx, 'pass> Deref for Graphics<'_, 'gfx, 'pass> {
    type Target = kludgine::render::Renderer<'gfx, 'pass>;

    fn deref(&self) -> &Self::Target {
        match &self.renderer {
            GraphicsContext::Renderer(renderer) => renderer,
            GraphicsContext::Clipped(clipped) => clipped,
        }
    }
}

impl<'gfx, 'pass> DerefMut for Graphics<'_, 'gfx, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.renderer {
            GraphicsContext::Renderer(renderer) => renderer,
            GraphicsContext::Clipped(clipped) => &mut *clipped,
        }
    }
}
