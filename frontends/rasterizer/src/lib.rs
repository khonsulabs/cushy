use std::sync::{Arc, Mutex};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect},
    styles::Points,
    Gooey, WidgetId,
};
use winit::event::DeviceId;

mod context;
mod raster;
mod transmogrifier;

pub use self::{context::*, transmogrifier::*};

use raster::Raster;

#[derive(Debug)]
pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    last_raster: Arc<Mutex<Raster>>,
    renderer: Option<R>,
}

impl<R: Renderer> Clone for Rasterizer<R> {
    /// This implementation ignores the `renderer` field, as it's temporary
    /// state only used during the render method. It shouldn't ever be accessed
    /// outside of that context.
    fn clone(&self) -> Self {
        Self {
            ui: self.ui.clone(),
            last_raster: Arc::default(),
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
    pub fn new(ui: Gooey<Self>) -> Self {
        Self {
            ui: Arc::new(ui),
            last_raster: Arc::default(),
            renderer: None,
        }
    }

    pub fn render(&self, scene: R) {
        {
            let mut last_raster = self.last_raster.lock().unwrap();
            last_raster.reset();
        }
        let size = scene.size();

        self.ui.with_transmogrifier(
            self.ui.root_widget().id(),
            self,
            |transmogrifier, state, widget| {
                transmogrifier.render_within(
                    AnyRasterContext::new(
                        self.ui.root_widget().clone(),
                        state,
                        &Rasterizer {
                            ui: self.ui.clone(),
                            last_raster: self.last_raster.clone(),
                            renderer: Some(scene),
                        },
                        widget,
                    ),
                    Rect::new(Point2D::default(), size),
                );
            },
        );
    }

    pub fn clipped_to(&self, clip: Rect<f32, Points>) -> Option<Self> {
        self.renderer().map(|renderer| Self {
            ui: self.ui.clone(),
            last_raster: self.last_raster.clone(),
            renderer: Some(renderer.clip_to(clip)),
        })
    }

    pub fn handle_winit_event<'evt, T>(
        &self,
        scene: R,
        device: &DeviceId,
        event: &winit::event::Event<'evt, T>,
    ) {
        // TODO:
        // * The existing handling of focus/hover seemed fine from memory. The
        //   pain points were around styling, not the actual application of
        //   state. Actually there was a need of refactoring for code-reuse --
        //   each of the mouse event handlers were very similar.
    }

    pub fn renderer(&self) -> Option<&R> {
        self.renderer.as_ref()
    }

    pub fn rasterizerd_widget(&self, widget: WidgetId, bounds: Rect<f32, Points>) {
        let mut raster = self.last_raster.lock().unwrap();
        raster.widget_rendered(widget, bounds);
    }
}
