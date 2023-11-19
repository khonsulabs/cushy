use std::ops::{Deref, DerefMut};

use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{
    self, Fraction, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, ScreenUnit, Size, Zero,
};
use kludgine::render::Renderer;
use kludgine::shapes::Shape;
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{
    cosmic_text, ClipGuard, Color, Drawable, Kludgine, ShaderScalable, ShapeSource, TextureSource,
};

/// A 2d graphics context
pub struct Graphics<'clip, 'gfx, 'pass> {
    renderer: RenderContext<'clip, 'gfx, 'pass>,
    region: Rect<Px>,
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
            region: renderer.clip_rect().into_signed(),
            renderer: RenderContext::Renderer(renderer),
        }
    }

    /// Returns the offset relative to the clipping rect that the graphics
    /// context renders at.
    ///
    /// This is used when rendering controls that are partially offscreen to the
    /// left or top of the window's origin.
    ///
    /// In general, this is handled automatically. This function should only be
    /// needed when using [`inner_graphics()`](Self::inner_graphics).
    #[must_use]
    pub fn translation(&self) -> Point<Px> {
        let clip_origin = self.renderer.clip_rect().origin.into_signed();
        -Point::new(
            if clip_origin.x <= self.region.origin.x {
                Px::ZERO
            } else {
                clip_origin.x - self.region.origin.x
            },
            if clip_origin.y <= self.region.origin.y {
                Px::ZERO
            } else {
                clip_origin.y - self.region.origin.y
            },
        )
    }

    /// Returns the underlying renderer.
    ///
    /// Note: Kludgine graphics contexts only support clipping. This type adds
    /// [`self.translation()`](Self::translation) to the offset of each drawing
    /// call. When using the underlying renderer, any drawing calls will need
    /// this offset as well, otherwise the widget that is being rendered will
    /// not render correctly when placed in a [`Scroll`](crate::widgets::Scroll)
    /// widget.
    pub fn inner_graphics(&mut self) -> &mut Renderer<'gfx, 'pass> {
        &mut self.renderer
    }

    /// Returns a context that has been clipped to `clip`.
    ///
    /// The new clipping rectangle is interpreted relative to the current
    /// clipping rectangle. As a side effect, this function can never expand the
    /// clipping rect beyond the current clipping rect.
    ///
    /// The returned context will report the clipped size, and all drawing
    /// operations will be relative to the origin of `clip`.
    pub fn clipped_to(&mut self, clip: Rect<Px>) -> Graphics<'_, 'gfx, 'pass> {
        let region = clip + self.region.origin;
        let new_clip = self
            .renderer
            .clip_rect()
            .intersection(&region.into_unsigned())
            .map(|intersection| intersection - self.renderer.clip_rect().origin)
            .unwrap_or_default();

        Graphics {
            renderer: RenderContext::Clipped(self.renderer.clipped_to(new_clip)),
            region,
        }
    }

    /// Returns the current clipping rectangle.
    ///
    /// The clipping rectangle is represented in unsigned pixels in the window's
    /// coordinate system.
    #[must_use]
    pub fn clip_rect(&self) -> Rect<UPx> {
        self.renderer.clip_rect()
    }

    /// Returns the current region being rendered to.
    ///
    /// The rendering region utilizes signed pixels, which allows it to
    /// represent regions that are out of bounds of the window's visible region.
    #[must_use]
    pub fn region(&self) -> Rect<Px> {
        self.region
    }

    /// Returns the visible region of the graphics context.
    ///
    /// This is the intersection of [`Self::region()`] and
    /// [`Self::clip_rect()`].
    #[must_use]
    pub fn visible_rect(&self) -> Option<Rect<UPx>> {
        self.clip_rect().intersection(&self.region.into_unsigned())
    }

    /// Returns the size of the current region.
    ///
    /// This is `self.region().size` converted to unsigned pixels.
    #[must_use]
    pub fn size(&self) -> Size<UPx> {
        self.region.size.into_unsigned()
    }

    /// Returns the current DPI scaling factor applied to the window this
    /// context is attached to.
    #[must_use]
    pub fn scale(&self) -> Fraction {
        self.renderer.scale()
    }

    /// Fills the entire context with `color`.
    ///
    /// If the alpha channel of `color` is 0, this function does nothing.
    pub fn fill(&mut self, color: Color) {
        if color.alpha() > 0 {
            let rect = Rect::from(self.region.size);
            self.draw_shape(&Shape::filled_rect(rect, color));
        }
    }

    /// Draws a shape at the origin, rotating and scaling as needed.
    pub fn draw_shape<'a, Unit>(&mut self, shape: impl Into<Drawable<&'a Shape<Unit, false>, Unit>>)
    where
        Unit: Zero + ShaderScalable + figures::ScreenUnit + Copy,
    {
        let mut shape = shape.into();
        shape.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_shape(shape);
    }

    /// Draws `texture` at `destination`, scaling as necessary.
    pub fn draw_texture<Unit>(&mut self, texture: &impl TextureSource, destination: Rect<Unit>)
    where
        Unit: figures::ScreenUnit + ShaderScalable,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        let translate = Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_texture(texture, destination + translate);
    }

    /// Draws a shape that was created with texture coordinates, applying the
    /// provided texture.
    pub fn draw_textured_shape<'shape, Unit, Shape>(
        &mut self,
        shape: impl Into<Drawable<&'shape Shape, Unit>>,
        texture: &impl TextureSource,
    ) where
        Unit: Zero + ShaderScalable + figures::ScreenUnit + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
        Shape: ShapeSource<Unit, true> + 'shape,
    {
        let mut shape = shape.into();
        shape.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_textured_shape(shape, texture);
    }

    /// Measures `text` using the current text settings.
    ///
    /// `default_color` does not affect the
    pub fn measure_text<'a, Unit>(&mut self, text: impl Into<Text<'a, Unit>>) -> MeasuredText<Unit>
    where
        Unit: figures::ScreenUnit,
    {
        self.renderer.measure_text(text)
    }

    /// Draws `text` using the current text settings.
    pub fn draw_text<'a, Unit>(&mut self, text: impl Into<Drawable<Text<'a, Unit>, Unit>>)
    where
        Unit: ScreenUnit,
    {
        let mut text = text.into();
        text.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_text(text);
    }

    /// Prepares the text layout contained in `buffer` to be rendered.
    ///
    /// When the text in `buffer` has no color defined, `default_color` will be
    /// used.
    ///
    /// `origin` allows controlling how the text will be drawn relative to the
    /// coordinate provided in [`render()`](kludgine::PreparedGraphic::render).
    pub fn draw_text_buffer<'a, Unit>(
        &mut self,
        buffer: impl Into<Drawable<&'a cosmic_text::Buffer, Unit>>,
        default_color: Color,
        origin: TextOrigin<Px>,
    ) where
        Unit: ScreenUnit,
    {
        let mut buffer = buffer.into();
        buffer.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer
            .draw_text_buffer(buffer, default_color, origin);
    }

    /// Measures `buffer` and caches the results using `default_color` when
    /// the buffer has no color associated with text.
    pub fn measure_text_buffer<Unit>(
        &mut self,
        buffer: &cosmic_text::Buffer,
        default_color: Color,
    ) -> MeasuredText<Unit>
    where
        Unit: figures::ScreenUnit,
    {
        self.renderer.measure_text_buffer(buffer, default_color)
    }

    /// Prepares the text layout contained in `buffer` to be rendered.
    ///
    /// When the text in `buffer` has no color defined, `default_color` will be
    /// used.
    ///
    /// `origin` allows controlling how the text will be drawn relative to the
    /// coordinate provided in [`render()`](kludgine::PreparedGraphic::render).
    pub fn draw_measured_text<'a, Unit>(
        &mut self,
        text: impl Into<Drawable<&'a MeasuredText<Unit>, Unit>>,
        origin: TextOrigin<Unit>,
    ) where
        Unit: ScreenUnit,
    {
        let mut text = text.into();
        text.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_measured_text(text, origin);
    }
}

impl<'gfx, 'pass> Deref for Graphics<'_, 'gfx, 'pass> {
    type Target = Kludgine;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}

impl<'gfx, 'pass> DerefMut for Graphics<'_, 'gfx, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.renderer
    }
}

impl<'gfx, 'pass> Deref for RenderContext<'_, 'gfx, 'pass> {
    type Target = Renderer<'gfx, 'pass>;

    fn deref(&self) -> &Self::Target {
        match self {
            RenderContext::Renderer(renderer) => renderer,
            RenderContext::Clipped(clipped) => clipped,
        }
    }
}

impl<'gfx, 'pass> DerefMut for RenderContext<'_, 'gfx, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RenderContext::Renderer(renderer) => renderer,
            RenderContext::Clipped(clipped) => &mut *clipped,
        }
    }
}
