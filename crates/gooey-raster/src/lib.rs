use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use gooey_core::graphics::{Drawable, Options, Point};
use gooey_core::math::{Rect, Size};
use gooey_core::style::{Lp, Px, UPx};
use gooey_core::{AnyWidget, Frontend, Transmogrify, Widget, WidgetTransmogrifier, Widgets};
use gooey_reactor::Value;

pub struct RasterizedApp<Surface>
where
    Surface: crate::Surface,
{
    handle: Arc<dyn SurfaceHandle>,
    surface: PhantomData<Surface>,
}

impl<Surface> Debug for RasterizedApp<Surface>
where
    Surface: crate::Surface,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterizedApp")
            .field("handle", &self.handle)
            .field("surface", &self.surface)
            .finish()
    }
}

impl<Surface> RasterizedApp<Surface>
where
    Surface: crate::Surface,
{
    pub fn new(handle: Arc<dyn SurfaceHandle>) -> Self {
        Self {
            handle,
            surface: PhantomData,
        }
    }
}

pub trait Surface: RefUnwindSafe + UnwindSafe + Send + Sync + Sized + 'static {
    type Context: RefUnwindSafe + UnwindSafe;
}

pub trait AnyWidgetRasterizer: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    fn measure(
        &mut self,
        available_space: Size<Option<UPx>>,
        renderer: &mut dyn Renderer,
    ) -> Size<UPx>;
    fn draw(&mut self, renderer: &mut dyn Renderer);
    fn mouse_down(&mut self, location: Point<Px>, surface: &dyn SurfaceHandle);
    fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle);
    fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle);
}

impl<T> AnyWidgetRasterizer for T
where
    T: WidgetRasterizer,
{
    fn measure(
        &mut self,
        available_space: Size<Option<UPx>>,
        renderer: &mut dyn Renderer,
    ) -> Size<UPx> {
        T::measure(self, available_space, renderer)
    }

    fn draw(&mut self, renderer: &mut dyn Renderer) {
        T::draw(self, renderer)
    }

    fn mouse_down(&mut self, location: Point<Px>, surface: &dyn SurfaceHandle) {
        T::mouse_down(self, location, surface)
    }

    fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
        T::mouse_up(self, location, surface)
    }

    fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
        T::cursor_moved(self, location, surface)
    }
}

pub struct Rasterizable(Box<dyn AnyWidgetRasterizer>);

impl Rasterizable {
    pub fn new<R>(rasterizable: R) -> Self
    where
        R: WidgetRasterizer,
    {
        Self(Box::new(rasterizable))
    }
}

impl Deref for Rasterizable {
    type Target = dyn AnyWidgetRasterizer;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl DerefMut for Rasterizable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

pub trait Renderer:
    DrawableState
    + Drawable<Lp>
    + Drawable<Px>
    + Deref<Target = Options>
    + DerefMut<Target = Options>
    + Debug
{
}

pub trait DrawableState {
    fn clip_to(&mut self, clip: Rect<UPx>);
    fn pop_clip(&mut self);

    fn size(&self) -> Size<UPx>;
}

impl<T> Renderer for T where
    T: DrawableState
        + Drawable<Lp>
        + Drawable<Px>
        + Debug
        + Deref<Target = Options>
        + DerefMut<Target = Options>
{
}

impl<Surface> Frontend for RasterizedApp<Surface>
where
    Surface: crate::Surface,
{
    type Context = RasterContext<Surface>;
    type Instance = Rasterizable;
}

#[derive(Debug)]
pub struct RasterContext<Surface>
where
    Surface: crate::Surface,
{
    widgets: Widgets<RasterizedApp<Surface>>,
    surface: Surface::Context,
    handle: Arc<dyn SurfaceHandle>,
}

impl<Surface> RasterContext<Surface>
where
    Surface: crate::Surface,
{
    pub fn new(
        widgets: Widgets<RasterizedApp<Surface>>,
        surface: Surface::Context,
        handle: Arc<dyn SurfaceHandle>,
    ) -> Self {
        Self {
            widgets,
            surface,
            handle,
        }
    }

    pub const fn surface(&self) -> &Surface::Context {
        &self.surface
    }

    pub const fn widgets(&self) -> &Widgets<RasterizedApp<Surface>> {
        &self.widgets
    }

    pub const fn handle(&self) -> &Arc<dyn SurfaceHandle> {
        &self.handle
    }
}

pub struct AnyRasterizer<Surface>(Box<dyn Transmogrify<RasterizedApp<Surface>>>)
where
    Surface: crate::Surface;

impl<Surface> AnyRasterizer<Surface>
where
    Surface: crate::Surface,
{
    pub fn new<T>(transmogrifier: T) -> Self
    where
        T: WidgetTransmogrifier<RasterizedApp<Surface>>,
    {
        Self(Box::new(transmogrifier))
    }
}

impl<Surface> Transmogrify<RasterizedApp<Surface>> for AnyRasterizer<Surface>
where
    Surface: crate::Surface,
{
    fn transmogrify(
        &self,
        widget: &dyn AnyWidget,
        style: Value<gooey_core::style::Style>,
        context: &<RasterizedApp<Surface> as Frontend>::Context,
    ) -> <RasterizedApp<Surface> as Frontend>::Instance {
        self.0.transmogrify(widget, style, context)
    }
}

pub trait SurfaceHandle: Debug + RefUnwindSafe + UnwindSafe + Sync + Send + 'static {
    fn invalidate(&self);
    // fn invalidate_rect(&self, rect: Rect<UPx>);
}

pub trait WidgetRasterizer: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    type Widget: Widget;
    fn measure(
        &mut self,
        available_space: Size<Option<UPx>>,
        renderer: &mut dyn Renderer,
    ) -> Size<UPx>;
    fn draw(&mut self, renderer: &mut dyn Renderer);

    #[allow(unused_variables)]
    fn mouse_down(&mut self, location: Point<Px>, surface: &dyn SurfaceHandle) {}
    #[allow(unused_variables)]
    fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {}
    #[allow(unused_variables)]
    fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {}
}
