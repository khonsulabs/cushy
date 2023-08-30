use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use gooey_core::graphics::{Drawable, Options, TextMetrics};
use gooey_core::math::units::UPx;
use gooey_core::math::{IntoSigned, Point, Rect, ScreenUnit};
use gooey_core::window::{NewWindow, Window, WindowAttributes, WindowLevel};
use gooey_core::{Context, IntoNewWidget, NewWidget, Runtime, Widgets};
use gooey_raster::{DrawableState, RasterContext, Rasterizable, RasterizedApp, Surface};
use kludgine::app::winit::dpi::{PhysicalPosition, PhysicalSize};
use kludgine::app::winit::event::ElementState;
use kludgine::app::{winit, WindowBehavior};
use kludgine::render::Drawing;
use kludgine::shapes::Shape;
use kludgine::text::TextOrigin;
use kludgine::{Clipped, Color};

pub fn run<Widget>(widgets: Arc<Widgets<RasterizedApp<Kludgine>>>, init: NewWindow<Widget>) -> !
where
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

    fn size(&self) -> gooey_core::math::Size<UPx> {
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
        first_baseline_origin: gooey_core::math::Point<Unit>,
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
    WindowTitleChanged,
    WindowLocationChanged,
    WindowSizeChanged,
    Invalidate,
    // InvalidateRect(Rect<UPx>),
}

struct GooeyWindow<Widget> {
    _root: NewWidget<Widget>,
    rasterizable: Rasterizable,
    context: RasterContext<Kludgine>,
    drawing: Drawing,
    window: Window,
}

#[derive(Debug)]
struct Handle(kludgine::app::WindowHandle<SurfaceEvent>);

impl gooey_raster::SurfaceHandle for Handle {
    fn invalidate(&self) {
        let _result = self.0.send(SurfaceEvent::Invalidate);
    }

    fn window_title_set(&self) {
        let _result = self.0.send(SurfaceEvent::WindowTitleChanged);
    }

    fn window_location_set(&self) {
        let _result = self.0.send(SurfaceEvent::WindowLocationChanged);
    }

    fn window_size_set(&self) {
        let _result = self.0.send(SurfaceEvent::WindowSizeChanged);
    }
    // fn invalidate_rect(&self, rect: Rect<UPx>) {
    //     let _result = self.0.send(SurfaceEvent::InvalidateRect(rect));
    // }
}

impl<Widget> kludgine::app::WindowBehavior<SurfaceEvent> for GooeyWindow<Widget>
where
    Widget: gooey_core::Widget,
{
    type Context = (Arc<Widgets<RasterizedApp<Kludgine>>>, NewWindow<Widget>);

    fn initialize(
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _graphics: &mut kludgine::Graphics<'_>,
        (widgets, window_init): Self::Context,
    ) -> Self {
        let runtime = Runtime::default();
        let handle = Arc::new(Handle(window.handle()));
        let context = Context::root(RasterizedApp::<Kludgine>::new(handle.clone(), runtime));
        let running_window = Window {
            inner_size: context.new_dynamic(window.inner_size()),
            location: context.new_dynamic(window.location()),
            title: context.new_dynamic(window.title()),
        };
        let root = (window_init.init)(&context, &running_window).into_new(&context);
        let context = RasterContext::new(widgets, (), handle);
        let rasterizable = context
            .widgets()
            .instantiate(&root.widget, *root.style, &context);

        root.style.for_each({
            let handle = context.handle().clone();
            move |_| handle.invalidate()
        });

        running_window.title.for_each({
            let handle = context.handle().clone();
            move |_| handle.window_title_set()
        });

        let drawing = Drawing::default();
        Self {
            _root: root,
            rasterizable,
            context,
            drawing,
            window: running_window,
        }
    }

    fn prepare(
        &mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        graphics: &mut kludgine::Graphics<'_>,
    ) {
        let renderer = self.drawing.new_frame(graphics);

        self.rasterizable.draw(
            &mut KludgineRenderer {
                renderer,
                options: Options::default(),
            },
            &mut self.context,
        );
    }

    fn initial_window_attributes(
        context: &Self::Context,
    ) -> kludgine::app::WindowAttributes<SurfaceEvent> {
        let WindowAttributes {
            inner_size,
            location,
            resizable,
            title,
            window_level,
            ..
            // min_inner_size,
            // max_inner_size,
            // enabled_buttons,
            // maximized,
            // visible,
            // transparent,
            // decorations,
            // resize_increments,
            // content_protected,
            // active,
        } = &context.1.attributes;

        kludgine::app::WindowAttributes {
            inner_size: inner_size.map(|inner_size| {
                winit::dpi::Size::Physical(PhysicalSize::new(
                    inner_size.width.0,
                    inner_size.height.0,
                ))
            }),
            location: location.map(|position| {
                winit::dpi::Position::Physical(PhysicalPosition::new(position.x.0, position.y.0))
            }),
            resizable: *resizable,
            window_level: match window_level {
                WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
                WindowLevel::Normal => winit::window::WindowLevel::Normal,
                WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
            },
            title: title.clone(),
            ..kludgine::app::WindowAttributes::default()
        }
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
        match event {
            SurfaceEvent::WindowTitleChanged => {
                self.window.title.map_ref(|title| window.set_title(title));
            }
            SurfaceEvent::WindowLocationChanged => {
                if let Some(location) = self.window.location.get() {
                    window.set_location(location);
                }
            }
            SurfaceEvent::WindowSizeChanged => {
                if let Some(size) = self.window.inner_size.get() {
                    window.set_inner_size(size);
                }
            }
            SurfaceEvent::Invalidate => {
                window.set_needs_redraw();
            }
        }
    }

    fn mouse_input(
        &mut self,
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: winit::event::DeviceId,
        state: ElementState,
        _button: winit::event::MouseButton,
    ) {
        match state {
            ElementState::Pressed => self.rasterizable.mouse_down(
                window
                    .cursor_position()
                    .expect("mouse down with no cursor position"),
                &mut self.context,
            ),
            ElementState::Released => self
                .rasterizable
                .mouse_up(window.cursor_position(), &mut self.context),
        }
    }

    fn cursor_left(
        &mut self,
        _window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: winit::event::DeviceId,
    ) {
        self.rasterizable.cursor_moved(None, &mut self.context);
    }

    fn cursor_moved(
        &mut self,
        window: kludgine::app::Window<'_, SurfaceEvent>,
        _device_id: winit::event::DeviceId,
        position: winit::dpi::PhysicalPosition<f64>,
    ) {
        let position = position.into();
        if Rect::from(window.inner_size())
            .into_signed()
            .contains(position)
        {
            self.rasterizable
                .cursor_moved(Some(position), &mut self.context);
        } else {
            self.rasterizable.cursor_moved(None, &mut self.context);
        }
    }
}

fn gooey_color_to_kludgine(color: &gooey_core::style::Color) -> Color {
    let (r, g, b, a) = color.into_rgba();
    Color::new(r, g, b, a)
}
