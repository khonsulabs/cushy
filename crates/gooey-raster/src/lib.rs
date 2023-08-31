use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use gooey_core::graphics::Drawable;
use gooey_core::math::units::{Lp, Px, UPx};
use gooey_core::math::{Point, Rect, Size};
use gooey_core::style::DynamicStyle;
use gooey_core::{
    AnyWidget, BoxedWidget, Frontend, Transmogrify, Widget, WidgetInstance, WidgetTransmogrifier,
    Widgets,
};

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
    type Context: RefUnwindSafe + UnwindSafe + Send + Sync;
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ConstraintLimit {
    Known(UPx),
    ClippedAfter(UPx),
}

impl ConstraintLimit {
    #[must_use]
    pub fn max(self) -> UPx {
        match self {
            ConstraintLimit::Known(v) | ConstraintLimit::ClippedAfter(v) => v,
        }
    }
}

pub trait AnyWidgetRasterizer: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        renderer: &mut dyn Renderer,
        context: &mut dyn AnyRasterContext,
    ) -> Size<UPx>;
    fn draw(&mut self, renderer: &mut dyn Renderer, context: &mut dyn AnyRasterContext);
    fn mouse_down(&mut self, location: Point<Px>, context: &mut dyn AnyRasterContext);
    fn mouse_up(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext);
    fn cursor_moved(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext);
}

impl<T> AnyWidgetRasterizer for T
where
    T: WidgetRasterizer,
{
    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        renderer: &mut dyn Renderer,
        context: &mut dyn AnyRasterContext,
    ) -> Size<UPx> {
        T::measure(self, available_space, renderer, context)
    }

    fn draw(&mut self, renderer: &mut dyn Renderer, context: &mut dyn AnyRasterContext) {
        T::draw(self, renderer, context);
    }

    fn mouse_down(&mut self, location: Point<Px>, context: &mut dyn AnyRasterContext) {
        T::mouse_down(self, location, context);
    }

    fn mouse_up(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {
        T::mouse_up(self, location, context);
    }

    fn cursor_moved(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {
        T::cursor_moved(self, location, context);
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

pub trait Renderer: DrawableState + Drawable<Lp> + Drawable<Px> + Debug {}

pub trait DrawableState {
    fn clip_to(&mut self, clip: Rect<UPx>);
    fn pop_clip(&mut self);

    fn size(&self) -> Size<UPx>;
}

impl<T> Renderer for T where T: DrawableState + Drawable<Lp> + Drawable<Px> + Debug {}

impl<Surface> Frontend for RasterizedApp<Surface>
where
    Surface: crate::Surface,
{
    type Context = RasterContext<Surface>;
    type Instance = Rasterizable;
}

pub struct RasterContext<Surface>
where
    Surface: crate::Surface,
{
    widgets: Arc<Widgets<RasterizedApp<Surface>>>,
    surface: Surface::Context,
    handle: Arc<dyn SurfaceHandle>,
}

impl<Surface> RasterContext<Surface>
where
    Surface: crate::Surface,
{
    pub fn new(
        widgets: Arc<Widgets<RasterizedApp<Surface>>>,
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

    pub fn widgets(&self) -> &Widgets<RasterizedApp<Surface>> {
        &self.widgets
    }

    pub const fn handle(&self) -> &Arc<dyn SurfaceHandle> {
        &self.handle
    }
}
impl<Surface> AnyRasterContext for RasterContext<Surface>
where
    Surface: crate::Surface,
{
    fn instantiate(&self, widget: &WidgetInstance<BoxedWidget>) -> Rasterizable {
        self.widgets
            .instantiate(&*widget.widget, widget.style, self)
    }
}

impl<Surface> SurfaceHandle for RasterContext<Surface>
where
    Surface: crate::Surface,
{
    fn invalidate(&self) {
        self.handle.invalidate();
    }

    fn window_title_set(&self) {
        self.handle.window_title_set();
    }

    fn window_position_set(&self) {
        self.handle.window_position_set();
    }

    fn window_size_set(&self) {
        self.handle.window_size_set();
    }
}

impl<Surface> Debug for RasterContext<Surface>
where
    Surface: crate::Surface,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterContext")
            .field("widgets", &self.widgets)
            .field("handle", &self.handle)
            .finish_non_exhaustive()
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
        style: DynamicStyle,
        context: &<RasterizedApp<Surface> as Frontend>::Context,
    ) -> <RasterizedApp<Surface> as Frontend>::Instance {
        self.0.transmogrify(widget, style, context)
    }
}

pub trait SurfaceHandle: Debug + RefUnwindSafe + UnwindSafe + Sync + Send + 'static {
    fn window_title_set(&self);
    fn window_position_set(&self);
    fn window_size_set(&self);
    fn invalidate(&self);
    // fn invalidate_rect(&self, rect: Rect<UPx>);
}

pub trait WidgetRasterizer: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    type Widget: Widget;
    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        renderer: &mut dyn Renderer,
        context: &mut dyn AnyRasterContext,
    ) -> Size<UPx>;
    fn draw(&mut self, renderer: &mut dyn Renderer, context: &mut dyn AnyRasterContext);

    #[allow(unused_variables)]
    fn mouse_down(&mut self, location: Point<Px>, context: &mut dyn AnyRasterContext) {}
    #[allow(unused_variables)]
    fn cursor_moved(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {}
    #[allow(unused_variables)]
    fn mouse_up(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {}
}

pub trait AnyRasterContext: SurfaceHandle {
    fn instantiate(&self, widget: &WidgetInstance<BoxedWidget>) -> Rasterizable;
}
