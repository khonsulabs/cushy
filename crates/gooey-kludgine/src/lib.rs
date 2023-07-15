use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use gooey_core::graphics::{Options, Point, Rect, Renderer, TextMetrics};
use gooey_core::math::IntoSigned;
use gooey_core::style::{Px, UPx};
use gooey_core::{ActiveContext, IntoNewWidget, NewWidget, Runtime, Widgets};
use gooey_raster::{RasterContext, RasterizedApp, Surface, SurfaceHandle, WidgetRasterizer};
use kludgine::app::winit::event::ElementState;
use kludgine::app::WindowBehavior;
use kludgine::render::Drawing;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::Color;

pub fn run<Widget, Initializer>(widgets: Widgets<RasterizedApp<Kludgine>>, init: Initializer) -> !
where
    Initializer: FnOnce(&ActiveContext) -> Widget + UnwindSafe + Send + 'static,
    Widget: gooey_core::Widget,
{
    GooeyWindow::run_with((widgets, init))
}

#[derive(Debug)]
pub struct Kludgine;

impl Surface for Kludgine {
    type Context = ();
    type Rasterizable = Rasterizable;

    fn new_rasterizable<R>(rasterizable: R) -> Self::Rasterizable
    where
        R: WidgetRasterizer,
    {
        Rasterizable::new(rasterizable)
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

pub trait AnyWidgetRasterizer: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    fn draw(&mut self, renderer: &mut KludgineRenderer<'_, '_, '_>);
    fn mouse_down(&mut self, location: Point<Px>, surface: &dyn SurfaceHandle);
    fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle);
    fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle);
}

impl<T> AnyWidgetRasterizer for T
where
    T: WidgetRasterizer,
{
    fn draw(&mut self, renderer: &mut KludgineRenderer<'_, '_, '_>) {
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

#[derive(Debug)]
pub struct KludgineTransmogrifier;

#[derive(Debug)]
pub struct KludgineRenderer<'clip, 'r, 'gfx> {
    renderer: PossiblyClipped<'clip, 'r, 'gfx>,
    options: &'clip mut Options,
}

#[derive(Debug)]
enum PossiblyClipped<'clip, 'r, 'gfx> {
    Renderer(kludgine::render::Renderer<'r, 'gfx>),
    Clipped(kludgine::ClipGuard<'clip, kludgine::render::Renderer<'r, 'gfx>>),
}

impl<'clip, 'r, 'gfx> Deref for KludgineRenderer<'clip, 'r, 'gfx> {
    type Target = Options;

    fn deref(&self) -> &Self::Target {
        self.options
    }
}

impl<'clip, 'r, 'gfx> DerefMut for KludgineRenderer<'clip, 'r, 'gfx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.options
    }
}

impl<'r, 'gfx> Deref for PossiblyClipped<'_, 'r, 'gfx> {
    type Target = kludgine::render::Renderer<'r, 'gfx>;

    fn deref(&self) -> &Self::Target {
        match self {
            PossiblyClipped::Renderer(r) => r,
            PossiblyClipped::Clipped(r) => r,
        }
    }
}
impl<'r, 'gfx> DerefMut for PossiblyClipped<'_, 'r, 'gfx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PossiblyClipped::Renderer(r) => r,
            PossiblyClipped::Clipped(r) => r,
        }
    }
}

impl<'clip, 'r, 'gfx> Renderer for KludgineRenderer<'clip, 'r, 'gfx> {
    type Clipped<'newclip>
    = KludgineRenderer< 'newclip, 'r, 'gfx>
    where
        Self: 'newclip;

    fn fill_rect<Unit>(&mut self, rect: Rect<Unit>)
    where
        Unit: gooey_core::math::ScreenUnit,
    {
        self.renderer.draw_shape(
            &Shape::filled_rect(rect, gooey_color_to_kludgine(&self.options.fill.color)),
            Point::default(),
            None,
            None,
        )
    }

    fn draw_text<Unit>(
        &mut self,
        text: &str,
        first_baseline_origin: gooey_core::graphics::Point<Unit>,
        maximum_width: Option<Unit>,
    ) where
        Unit: gooey_core::math::ScreenUnit,
    {
        self.renderer.draw_text(
            text,
            gooey_color_to_kludgine(&self.options.fill.color),
            TextOrigin::FirstBaseline,
            first_baseline_origin,
            None,
            None,
        );
    }

    fn measure_text<Unit>(&mut self, text: &str, maximum_width: Option<Unit>) -> TextMetrics<Unit>
    where
        Unit: gooey_core::math::ScreenUnit,
    {
        let text = self
            .renderer
            .measure_text(text, gooey_color_to_kludgine(&self.options.fill.color));
        TextMetrics {
            ascent: text.ascent,
            descent: text.descent,
            size: text.size,
        }
    }

    fn clip_to(&mut self, clip: Rect<UPx>) -> Self::Clipped<'_> {
        KludgineRenderer {
            renderer: PossiblyClipped::Clipped(self.renderer.clipped_to(clip)),
            options: self.options,
        }
    }

    fn size(&self) -> gooey_core::graphics::Size<UPx> {
        self.renderer.size()
    }
}

#[derive(Debug)]
pub enum SurfaceEvent {
    Invalidate,
    // InvalidateRect(Rect<UPx>),
}

struct GooeyWindow<Initializer, Widget> {
    _root: NewWidget<Widget>,
    rasterizable: Rasterizable,
    context: RasterContext<Kludgine>,
    _runtime: Runtime,
    drawing: Drawing,
    widget: PhantomData<Initializer>,
}

#[derive(Debug)]
struct Handle(kludgine::app::WindowHandle<SurfaceEvent>);

impl gooey_raster::SurfaceHandle for Handle {
    fn invalidate(&self) {
        let _result = self.0.send(SurfaceEvent::Invalidate);
    }

    // fn invalidate_rect(&self, rect: Rect<UPx>) {
    //     let _result = self.0.send(SurfaceEvent::InvalidateRect(rect));
    // }
}

impl<Initializer, Widget> kludgine::app::WindowBehavior<SurfaceEvent>
    for GooeyWindow<Initializer, Widget>
where
    Initializer: FnOnce(&ActiveContext) -> Widget + UnwindSafe + Send + 'static,
    Widget: gooey_core::Widget,
{
    type Context = (Widgets<RasterizedApp<Kludgine>>, Initializer);

    fn initialize(
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _graphics: &mut kludgine::Graphics<'_>,
        (widgets, init): Self::Context,
    ) -> Self {
        let runtime = Runtime::default();
        let handle = Arc::new(Handle(window.handle()));
        let context = ActiveContext::root(RasterizedApp::<Kludgine>::new(handle.clone()), &runtime);
        let root = init(&context).into_new(&context);
        let context = RasterContext::new(widgets, (), handle);
        let rasterizable = context
            .widgets()
            .instantiate(&root.widget, *root.style, &context);

        root.style.for_each({
            let handle = context.handle().clone();
            move |_| handle.invalidate()
        });

        // let root = app.instantiate(&root.widget, *root.style, &RasterContext);
        let drawing = Drawing::default();
        Self {
            _root: root,
            rasterizable,
            context,
            _runtime: runtime,
            drawing,
            widget: PhantomData,
        }
    }

    fn prepare(
        &mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        graphics: &mut kludgine::Graphics<'_>,
    ) {
        let renderer = self.drawing.new_frame(graphics);

        self.rasterizable.0.draw(&mut KludgineRenderer {
            renderer: PossiblyClipped::Renderer(renderer),
            options: &mut Options::default(),
        });
    }

    fn render<'pass>(
        &'pass mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.drawing.render(graphics);
        true
    }

    fn clear_color() -> Option<Color> {
        Some(Color::WHITE)
    }

    fn event(&mut self, event: SurfaceEvent, mut window: kludgine::app::Window<'_, SurfaceEvent>) {
        let SurfaceEvent::Invalidate = event;
        window.set_needs_redraw();
    }

    fn mouse_input(
        &mut self,
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: kludgine::app::winit::event::DeviceId,
        state: ElementState,
        _button: kludgine::app::winit::event::MouseButton,
    ) {
        match state {
            ElementState::Pressed => self.rasterizable.0.mouse_down(
                window
                    .cursor_position()
                    .expect("mouse down with no cursor position"),
                &**self.context.handle(),
            ),
            ElementState::Released => self
                .rasterizable
                .0
                .mouse_up(window.cursor_position(), &**self.context.handle()),
        }
    }

    fn cursor_left(
        &mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: kludgine::app::winit::event::DeviceId,
    ) {
        self.rasterizable
            .0
            .cursor_moved(None, &**self.context.handle());
    }

    fn cursor_moved(
        &mut self,
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: kludgine::app::winit::event::DeviceId,
        position: kludgine::app::winit::dpi::PhysicalPosition<f64>,
    ) {
        let position = position.into();
        if Rect::from(window.inner_size())
            .into_signed()
            .contains(position)
        {
            self.rasterizable
                .0
                .cursor_moved(Some(position), &**self.context.handle());
        } else {
            self.rasterizable
                .0
                .cursor_moved(None, &**self.context.handle());
        }
    }
}

fn gooey_color_to_kludgine(color: &gooey_core::style::Color) -> Color {
    let (r, g, b, a) = color.into_rgba();
    Color::new(r, g, b, a)
}
