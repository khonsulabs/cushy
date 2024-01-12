use std::ops::{Deref, DerefMut};

use ahash::AHashSet;
use figures::units::{Px, UPx};
use figures::{
    self, Fraction, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, ScreenUnit, Size, Zero,
};
use kludgine::cosmic_text::FamilyOwned;
use kludgine::drawing::Renderer;
use kludgine::shapes::Shape;
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{
    cosmic_text, ClipGuard, Color, Drawable, Kludgine, ShaderScalable, ShapeSource, TextureSource,
};

use crate::animation::ZeroToOne;
use crate::styles::FontFamilyList;

/// A 2d graphics context
pub struct Graphics<'clip, 'gfx, 'pass> {
    renderer: RenderContext<'clip, 'gfx, 'pass>,
    region: Rect<Px>,
    font_state: &'clip mut FontState,
    pub(crate) opacity: ZeroToOne,
}

enum RenderContext<'clip, 'gfx, 'pass> {
    Renderer(Renderer<'gfx, 'pass>),
    Clipped(ClipGuard<'clip, Renderer<'gfx, 'pass>>),
}

impl<'clip, 'gfx, 'pass> Graphics<'clip, 'gfx, 'pass> {
    /// Returns a new graphics context for the given [`Renderer`].
    #[must_use]
    pub fn new(renderer: Renderer<'gfx, 'pass>, font_state: &'clip mut FontState) -> Self {
        Self {
            region: renderer.clip_rect().into_signed(),
            renderer: RenderContext::Renderer(renderer),
            font_state,
            opacity: ZeroToOne::ONE,
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

    /// Returns the first font family in `list` that is currently in the font
    /// system, or None if no font families match.
    pub fn find_available_font_family(&mut self, list: &FontFamilyList) -> Option<FamilyOwned> {
        self.font_state.find_available_font_family(list)
    }

    /// Sets the font family to the first family in `list`.
    pub fn set_available_font_family(&mut self, list: &FontFamilyList) {
        if self.font_state.current_font_family.as_ref() != Some(list) {
            self.font_state.current_font_family = Some(list.clone());
            if let Some(family) = self.find_available_font_family(list) {
                self.set_font_family(family);
            }
        }
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
            font_state: &mut *self.font_state,
            opacity: self.opacity,
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
        shape.opacity = Some(
            shape
                .opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
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
        self.renderer
            .draw_texture(texture, destination + translate, *self.opacity);
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
        shape.opacity = Some(
            shape
                .opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
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
        text.opacity = Some(
            text.opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
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
        buffer.opacity = Some(
            buffer
                .opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
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
        text.opacity = Some(
            text.opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
        text.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_measured_text(text, origin);
    }

    /// Returns this renderer as a
    /// [`DrawingArea`](plotters::drawing::DrawingArea) compatible with the
    /// [plotters](https://github.com/plotters-rs/plotters) crate.
    #[cfg(feature = "plotters")]
    pub fn as_plot_area(
        &mut self,
    ) -> plotters::drawing::DrawingArea<
        kludgine::drawing::PlotterBackend<'_, 'gfx, 'pass>,
        plotters::coord::Shift,
    > {
        self.renderer.as_plot_area()
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

pub struct FontState {
    fonts: AHashSet<String>,
    current_font_family: Option<FontFamilyList>,
}

impl FontState {
    pub fn new(db: &cosmic_text::fontdb::Database) -> Self {
        let faces = db.faces();
        let mut fonts = AHashSet::with_capacity(faces.size_hint().0);
        for (family, _) in faces.filter_map(|f| f.families.first()) {
            fonts.insert(family.clone());
        }
        Self {
            fonts,
            current_font_family: None,
        }
    }

    pub fn next_frame(&mut self) {
        self.current_font_family = None;
    }

    pub fn find_available_font_family(&self, list: &FontFamilyList) -> Option<FamilyOwned> {
        list.iter()
            .find(|family| match family {
                FamilyOwned::Name(name) => self.fonts.contains(name),
                _ => true,
            })
            .cloned()
    }

    pub fn apply_font_family_list(
        &self,
        family: &FontFamilyList,
        fallback: impl FnOnce() -> Option<FamilyOwned>,
        apply: impl FnOnce(String),
    ) {
        if let Some(FamilyOwned::Name(name)) =
            self.find_available_font_family(family).or_else(fallback)
        {
            apply(name);
        }
    }
}
