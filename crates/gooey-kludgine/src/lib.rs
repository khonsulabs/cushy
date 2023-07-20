use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::UnwindSafe;
use std::sync::Arc;

use gooey_core::graphics::{Drawable, Options, Point, Rect, TextMetrics};
use gooey_core::math::{IntoSigned, ScreenUnit};
use gooey_core::style::UPx;
use gooey_core::{ActiveContext, IntoNewWidget, NewWidget, Runtime, Widgets};
use gooey_raster::{DrawableState, RasterContext, Rasterizable, RasterizedApp, Surface};
use kludgine::app::winit::event::ElementState;
use kludgine::app::WindowBehavior;
use kludgine::render::Drawing;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::{Clipped, Color};

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
}

#[derive(Debug)]
pub struct KludgineTransmogrifier;

#[derive(Debug)]
pub struct KludgineRenderer<'r, 'gfx> {
    renderer: kludgine::render::Renderer<'r, 'gfx>,
    options: Options,
}

impl<'r, 'gfx> Deref for KludgineRenderer<'r, 'gfx> {
    type Target = Options;

    fn deref(&self) -> &Self::Target {
        &self.options
    }
}

impl<'r, 'gfx> DerefMut for KludgineRenderer<'r, 'gfx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.options
    }
}

impl<'r, 'gfx> DrawableState for KludgineRenderer<'r, 'gfx> {
    fn clip_to(&mut self, clip: Rect<UPx>) {
        self.renderer.push_clip(clip);
    }

    fn size(&self) -> gooey_core::graphics::Size<UPx> {
        self.renderer.size()
    }

    fn pop_clip(&mut self) {
        self.renderer.pop_clip();
    }
}

impl<'r, 'gfx, Unit> Drawable<Unit> for KludgineRenderer<'r, 'gfx>
where
    Unit: ScreenUnit,
{
    fn fill_rect(&mut self, rect: Rect<Unit>) {
        self.renderer.draw_shape(
            &Shape::filled_rect(rect, gooey_color_to_kludgine(&self.options.fill.color)),
            Point::default(),
            None,
            None,
        )
    }

    fn draw_text(
        &mut self,
        text: &str,
        first_baseline_origin: gooey_core::graphics::Point<Unit>,
        _maximum_width: Option<Unit>,
    ) {
        // TODO honor maximium_width
        self.renderer.draw_text(
            text,
            gooey_color_to_kludgine(&self.options.fill.color),
            TextOrigin::FirstBaseline,
            first_baseline_origin,
            None,
            None,
        );
    }

    fn measure_text(&mut self, text: &str, _maximum_width: Option<Unit>) -> TextMetrics<Unit> {
        // TODO honor maximium_width
        let text = self
            .renderer
            .measure_text(text, gooey_color_to_kludgine(&self.options.fill.color));
        TextMetrics {
            ascent: text.ascent,
            descent: text.descent,
            size: text.size,
        }
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

        self.rasterizable.draw(&mut KludgineRenderer {
            renderer,
            options: Options::default(),
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
            ElementState::Pressed => self.rasterizable.mouse_down(
                window
                    .cursor_position()
                    .expect("mouse down with no cursor position"),
                &**self.context.handle(),
            ),
            ElementState::Released => self
                .rasterizable
                .mouse_up(window.cursor_position(), &**self.context.handle()),
        }
    }

    fn cursor_left(
        &mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: kludgine::app::winit::event::DeviceId,
    ) {
        self.rasterizable
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
                .cursor_moved(Some(position), &**self.context.handle());
        } else {
            self.rasterizable
                .cursor_moved(None, &**self.context.handle());
        }
    }
}

fn gooey_color_to_kludgine(color: &gooey_core::style::Color) -> Color {
    let (r, g, b, a) = color.into_rgba();
    Color::new(r, g, b, a)
}
