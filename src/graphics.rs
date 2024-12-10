use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use figures::units::{Px, UPx};
use figures::{
    self, Fraction, IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, ScreenUnit, Size, Zero
};
use kempt::{map, Map};
use kludgine::cosmic_text::{fontdb, FamilyOwned, FontSystem};
use kludgine::drawing::Renderer;
use kludgine::shapes::Shape;
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{
    cosmic_text, ClipGuard, Color, Drawable, Kludgine, RenderingGraphics, ShaderScalable,
    ShapeSource, TextureSource,
};

use crate::animation::ZeroToOne;
use crate::fonts::{FontCollection, LoadedFontFace, LoadedFontId};
use crate::styles::FontFamilyList;
use crate::value::{DynamicRead, Generation, Source};

/// A 2d graphics context
pub struct Graphics<'clip, 'gfx, 'pass> {
    renderer: RenderContext<'clip, 'gfx, 'pass>,
    region: Rect<Px>,
    pub(crate) opacity: ZeroToOne,
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
        self.region.origin - clip_origin
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

        // If the current region has a negative component, we need to adjust the
        // clipped rect before we perform an intersection in unsigned space.
        let mut effective_region = region;
        if region.origin.x < 0 {
            effective_region.size.width += region.origin.x;
            effective_region.origin.x = Px::ZERO;
        }
        if region.origin.y < 0 {
            effective_region.size.height += region.origin.y;
            effective_region.origin.y = Px::ZERO;
        }
        let new_clip = self
            .renderer
            .clip_rect()
            .intersection(&effective_region.into_unsigned())
            .map(|intersection| intersection - self.renderer.clip_rect().origin)
            .unwrap_or_default();

        Graphics {
            renderer: RenderContext::Clipped(self.renderer.clipped_to(new_clip)),
            region,
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
    pub fn draw_texture<Unit>(
        &mut self,
        texture: &impl TextureSource,
        destination: Rect<Unit>,
        opacity: ZeroToOne,
    ) where
        Unit: figures::ScreenUnit + ShaderScalable,
        i32: From<<Unit as IntoSigned>::Signed>,
    {
        let translate = Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer
            .draw_texture(texture, destination + translate, *(self.opacity * opacity));
    }

    /// Draws a shape that was created with texture coordinates, applying the
    /// provided texture.
    pub fn draw_textured_shape<'shape, Unit, Shape>(
        &mut self,
        shape: impl Into<Drawable<&'shape Shape, Unit>>,
        texture: &impl TextureSource,
        opacity: ZeroToOne,
    ) where
        Unit: Zero + ShaderScalable + figures::ScreenUnit + Copy,
        i32: From<<Unit as IntoSigned>::Signed>,
        Shape: ShapeSource<Unit, true> + 'shape,
    {
        let mut shape = shape.into();
        let effective_opacity = self.opacity * opacity;
        shape.opacity = Some(
            shape
                .opacity
                .map_or(*effective_opacity, |opacity| opacity * *effective_opacity),
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
        Unit: ScreenUnit + Round,
    {
        let mut text = text.into();
        text.opacity = Some(
            text.opacity
                .map_or(*self.opacity, |opacity| opacity * *self.opacity),
        );
        text.translation += Point::<Unit>::from_px(self.translation(), self.scale());
        self.renderer.draw_measured_text(text, origin);
    }

    /// Draws the custom rendering operation when this graphics is presented to
    /// the screen.
    ///
    /// The rendering operation will be clipped automatically, but the rendering
    /// operation will need to position and size itself accordingly.
    pub fn draw<Op: RenderOperation>(&mut self)
    where
        Op::DrawInfo: Default,
    {
        let region = self.region;
        self.renderer
            .draw::<CushyRenderOp<Op>>((<Op::DrawInfo>::default(), region));
    }

    /// Draws `Op` with the given context.
    pub fn draw_with<Op: RenderOperation>(&mut self, context: Op::DrawInfo) {
        let region = self.region;
        self.renderer.draw::<CushyRenderOp<Op>>((context, region));
    }

    /// Returns a reference to the font system used to render.
    pub fn font_system(&mut self) -> &mut FontSystem {
        self.renderer.font_system()
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

#[derive(Debug)]
struct Prepared<T> {
    region: Rect<Px>,
    data: T,
}

struct CushyRenderOp<Op>(Op);

impl<Op> kludgine::drawing::RenderOperation for CushyRenderOp<Op>
where
    Op: RenderOperation,
{
    type DrawInfo = (Op::DrawInfo, Rect<Px>);
    type Prepared = Prepared<Op::Prepared>;

    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self {
        Self(Op::new(graphics))
    }

    fn prepare(
        &mut self,
        (context, region): Self::DrawInfo,
        graphics: &mut kludgine::Graphics<'_>,
    ) -> Self::Prepared {
        Prepared {
            region,
            data: self.0.prepare(context, region, graphics),
        }
    }

    fn render<'pass>(
        &'pass self,
        prepared: &Self::Prepared,
        opacity: f32,
        graphics: &mut RenderingGraphics<'_, 'pass>,
    ) {
        self.0
            .render(&prepared.data, prepared.region, opacity, graphics);
    }
}

pub(crate) struct LoadedFontIds {
    generation: usize,
    pub(crate) faces: Vec<LoadedFontFace>,
}

pub struct FontState {
    app_fonts: FontCollection,
    app_font_generation: Generation,
    window_fonts: FontCollection,
    window_font_generation: Option<Generation>,
    pub(crate) loaded_fonts: Map<LoadedFontId, LoadedFontIds>,
    font_generation: usize,
    fonts: Map<String, usize>,
    pub(crate) current_font_family: Option<FontFamilyList>,
}

impl FontState {
    pub fn new(
        db: &mut cosmic_text::fontdb::Database,
        window_fonts: FontCollection,
        app_fonts: FontCollection,
    ) -> Self {
        let mut fonts = Map::new();
        Self::gather_available_family_names(&mut fonts, 0, db);
        let mut state = Self {
            fonts,
            current_font_family: None,
            window_font_generation: None,
            window_fonts,
            app_font_generation: app_fonts.0.generation(),
            app_fonts,
            font_generation: 0,
            loaded_fonts: Map::new(),
        };

        state.update_fonts(db);

        state
    }

    fn gather_available_family_names(
        families: &mut Map<String, usize>,
        generation: usize,
        db: &cosmic_text::fontdb::Database,
    ) {
        for (family, _) in db.faces().filter_map(|f| f.families.first()) {
            families
                .entry(family)
                .and_modify(|gen| *gen = generation)
                .or_insert(generation);
        }

        let mut i = 0;
        while i < families.len() {
            if families.field(i).expect("length checked").value == generation {
                i += 1;
            } else {
                families.remove_by_index(i);
            }
        }
    }

    pub fn update_fonts(&mut self, db: &mut cosmic_text::fontdb::Database) -> bool {
        let new_app_generation = self.app_fonts.0.generation();
        let app_fonts_changed = if self.app_font_generation == new_app_generation {
            false
        } else {
            self.app_font_generation = new_app_generation;
            true
        };
        let new_window_generation = self.window_fonts.0.generation();
        let window_fonts_changed = if self.window_font_generation == Some(new_window_generation) {
            false
        } else {
            self.window_font_generation = Some(new_window_generation);
            true
        };

        let changed = app_fonts_changed || window_fonts_changed;
        if changed {
            self.font_generation += 1;

            if app_fonts_changed {
                Self::synchronize_font_list(
                    &mut self.loaded_fonts,
                    self.font_generation,
                    &self.app_fonts,
                    db,
                );
            }
            if window_fonts_changed {
                Self::synchronize_font_list(
                    &mut self.loaded_fonts,
                    self.font_generation,
                    &self.window_fonts,
                    db,
                );
            }

            // Remove all fonts that didn't have their generation touched.
            let mut i = 0;
            while i < self.loaded_fonts.len() {
                let field = self.loaded_fonts.field(i).expect("length checked");
                let check_if_changed = (app_fonts_changed
                    && self.app_fonts.0.as_ptr() == field.key().collection)
                    || (window_fonts_changed
                        && self.window_fonts.0.as_ptr() == field.key().collection);
                if !check_if_changed || field.value.generation == self.font_generation {
                    i += 1;
                } else {
                    for face in self.loaded_fonts.remove_by_index(i).value.faces {
                        db.remove_face(face.id);
                    }
                }
            }

            Self::gather_available_family_names(&mut self.fonts, self.font_generation, db);
        }

        changed
    }

    fn synchronize_font_list(
        loaded_fonts: &mut Map<LoadedFontId, LoadedFontIds>,
        generation: usize,
        collection: &FontCollection,
        db: &mut cosmic_text::fontdb::Database,
    ) {
        for (font_id, data) in collection.0.read().fonts(collection) {
            match loaded_fonts.entry(font_id) {
                map::Entry::Occupied(mut entry) => {
                    entry.generation = generation;
                }
                map::Entry::Vacant(entry) => {
                    let faces = db
                        .load_font_source(fontdb::Source::Binary(data.clone()))
                        .into_iter()
                        .filter_map(|id| {
                            db.face(id).map(|face| LoadedFontFace {
                                id,
                                families: face.families.clone(),
                                weight: face.weight,
                                style: face.style,
                                stretch: face.stretch,
                            })
                        })
                        .collect();
                    entry.insert(LoadedFontIds { generation, faces });
                }
            }
        }
    }

    #[must_use]
    pub fn next_frame(&mut self, db: &mut cosmic_text::fontdb::Database) -> bool {
        self.current_font_family = None;
        self.update_fonts(db)
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

/// A custom wgpu-powered rendering operation.
///
/// # How custom rendering ops work
///
/// When [`Graphics::draw`]/[`Graphics::draw_with`] are invoked for the first
/// time for a given `RenderOperation` implementor, [`new()`](Self::new) is
/// called to create a shared instance of this rendering operation. The new
/// function is a good location to put any initialization that can be shared
/// amongst all drawing calls, as it is invoked only once for each surface.
///
/// After an existing or newly-created operation is located, `prepare()` is
/// invoked passing through the `DrawInfo` provided to the draw call. The result
/// of this function is stored.
///
/// When the graphics is presented, `render()` is invoked providing the data
/// returned from the `prepare()` function.
pub trait RenderOperation: Send + Sync + 'static {
    /// Data that is provided to the `prepare()` function. This is passed
    /// through from the `draw/draw_with` invocation.
    type DrawInfo;
    /// Data that is created in the `prepare()` function, and provided to the
    /// `render()` function.
    type Prepared: Debug + Send + Sync + 'static;

    /// Returns a new instance of this render operation.
    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self;

    /// Prepares this operation to be rendered in `region` in `graphics`.
    ///
    /// This operation's  will automatically be clipped to the available space
    /// for the context it is being drawn to. The render operation should
    /// project itself into `region` and only use the clip rect as an
    /// optimization. To test that this is handled correctly, try placing
    /// whatever is being rendered in a `Scroll` widget and ensure that as the
    /// contents are clipped, the visible area shows the correct contents.
    fn prepare(
        &mut self,
        context: Self::DrawInfo,
        region: Rect<Px>,
        graphics: &mut kludgine::Graphics<'_>,
    ) -> Self::Prepared;

    /// Render `preprared` to `graphics` at `region` with `opacity`.
    ///
    /// This operation's  will automatically be clipped to the available space
    /// for the context it is being drawn to. The render operation should
    /// project itself into `region` and only use the clip rect as an
    /// optimization. To test that this is handled correctly, try placing
    /// whatever is being rendered in a `Scroll` widget and ensure that as the
    /// contents are clipped, the visible area shows the correct contents.
    fn render(
        &self,
        prepared: &Self::Prepared,
        region: Rect<Px>,
        opacity: f32,
        graphics: &mut RenderingGraphics<'_, '_>,
    );
}

/// A [`RenderOperation`] with no per-drawing-call state.
pub trait SimpleRenderOperation: Send + Sync + 'static {
    /// Returns a new instance of this render operation.
    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self;

    /// Render to `graphics` at `rect` with `opacity`.
    ///
    /// This operation's  will automatically be clipped to the available space
    /// for the context it is being drawn to. The render operation should
    /// project itself into `region` and only use the clip rect as an
    /// optimization. To test that this is handled correctly, try placing
    /// whatever is being rendered in a `Scroll` widget and ensure that as the
    /// contents are clipped, the visible area shows the correct contents.
    fn render(&self, region: Rect<Px>, opacity: f32, graphics: &mut RenderingGraphics<'_, '_>);
}

impl<T> RenderOperation for T
where
    T: SimpleRenderOperation,
{
    type DrawInfo = ();
    type Prepared = ();

    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self {
        Self::new(graphics)
    }

    fn prepare(
        &mut self,
        _context: Self::DrawInfo,
        _origin: Rect<Px>,
        _graphics: &mut kludgine::Graphics<'_>,
    ) -> Self::Prepared {
    }

    fn render(
        &self,
        _prepared: &Self::Prepared,
        region: Rect<Px>,
        opacity: f32,
        graphics: &mut RenderingGraphics<'_, '_>,
    ) {
        self.render(region, opacity, graphics);
    }
}
