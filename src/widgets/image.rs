//! A widget that displays an image/texture.

use figures::units::{Px, UPx};
use figures::{FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, Size, Zero};
use kludgine::{AnyTexture, CollectedTexture, LazyTexture, SharedTexture, Texture, TextureRegion};

use crate::animation::ZeroToOne;
use crate::context::LayoutContext;
use crate::value::{IntoValue, Source, Value};
use crate::widget::Widget;
use crate::ConstraintLimit;

/// A widget that displays an image/texture.
#[derive(Debug)]
pub struct Image {
    /// The texture to render.
    pub contents: Value<AnyTexture>,
    /// The scaling strategy to apply.
    pub scaling: Value<ImageScaling>,
}

impl Image {
    /// Returns a new image widget that renders `contents`, using the default
    /// [`ImageScaling`] strategy.
    pub fn new(contents: impl IntoValue<AnyTexture>) -> Self {
        Self {
            contents: contents.into_value(),
            scaling: Value::default(),
        }
    }

    /// Applies the `scaling` strategies and returns self.
    #[must_use]
    pub fn scaling(mut self, scaling: impl IntoValue<ImageScaling>) -> Self {
        self.scaling = scaling.into_value();
        self
    }

    /// Applies the aspect-fit scaling strategy and returns self.
    ///
    /// The aspect-fit scaling strategy scales the image to be the largest size
    /// it can be without clipping. Any remaining whitespace will be at the
    /// right or bottom edge.
    ///
    /// To apply a different orientation for the whitespace, use
    /// [`Self::aspect_fit_around`].
    #[must_use]
    pub fn aspect_fit(self) -> Self {
        self.aspect_fit_around(Size::ZERO)
    }

    /// Applies the aspect-fit scaling strategy and returns self.
    ///
    /// The aspect-fit scaling strategy scales the image to be the largest size
    /// it can be without clipping. Any remaining whitespace will be divided
    /// using the ratio `orientation`.
    #[must_use]
    pub fn aspect_fit_around(self, orientation: Size<ZeroToOne>) -> Self {
        self.scaling(ImageScaling::Aspect {
            mode: Aspect::Fit,
            orientation,
        })
    }

    /// Applies the aspect-fill scaling strategy and returns self.
    ///
    /// The aspect-fill scaling strategy scales the image to be the smallest
    /// size it can be to cover the entire surface. The bottom or right sides of
    /// the image will be clipped.
    ///
    /// To apply a different orientation for the clipping, use
    /// [`Self::aspect_fill_around`].
    #[must_use]
    pub fn aspect_fill(self) -> Self {
        self.aspect_fill_around(Size::ZERO)
    }

    /// Applies the aspect-fill scaling strategy and returns self.
    ///
    /// The aspect-fill scaling strategy scales the image to be the smallest
    /// size it can be to cover the entire surface. The side that is cropped
    /// will be positioned using `orientation`.
    #[must_use]
    pub fn aspect_fill_around(self, orientation: Size<ZeroToOne>) -> Self {
        self.scaling(ImageScaling::Aspect {
            mode: Aspect::Fill,
            orientation,
        })
    }

    /// Applies the stretch scaling strategy and returns self.
    ///
    /// The stretch scaling strategy stretches the image to fill the surface,
    /// ignoring the aspect ratio.
    #[must_use]
    pub fn stretch(self) -> Self {
        self.scaling(ImageScaling::Stretch)
    }

    /// Applies a scaling factor strategy and returns self.
    ///
    /// The image will be displayed at a scaling factor of `amount`. In this
    /// mode, the widget will request that its size be the size of the contained
    /// image.
    #[must_use]
    pub fn scaled(self, amount: impl IntoValue<f32>) -> Self {
        self.scaling(match amount.into_value() {
            Value::Constant(amount) => Value::Constant(ImageScaling::Scale(amount)),
            Value::Dynamic(amount) => Value::Dynamic(amount.map_each_cloned(ImageScaling::Scale)),
        })
    }

    fn calculate_image_rect(
        &self,
        texture: &AnyTexture,
        within_size: Size<UPx>,
        context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>,
    ) -> Rect<Px> {
        let within_size = within_size.into_signed();
        let size = texture.size().into_signed();
        match self.scaling.get_tracking_invalidate(context) {
            ImageScaling::Aspect { mode, orientation } => {
                let scale_width = within_size.width.into_float() / size.width.into_float();
                let scale_height = within_size.height.into_float() / size.height.into_float();

                let effective_scale = match mode {
                    Aspect::Fill => scale_width.max(scale_height),
                    Aspect::Fit => scale_width.min(scale_height),
                };
                let scaled = size * effective_scale;

                let x = (within_size.width - scaled.width) * *orientation.width;
                let y = (within_size.height - scaled.height) * *orientation.height;

                Rect::new(Point::new(x, y), scaled)
            }
            ImageScaling::Stretch => within_size.into(),
            ImageScaling::Scale(factor) => {
                let size = size.map(|px| px * factor);
                size.into()
            }
        }
    }
}

impl Widget for Image {
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        self.contents.map(|texture| {
            let rect = self.calculate_image_rect(texture, context.gfx.size(), context);
            context.gfx.draw_texture(texture, rect);
        });
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let rect = self.contents.map(|texture| {
            self.calculate_image_rect(texture, available_space.map(ConstraintLimit::max), context)
        });
        rect.size.into_unsigned()
    }
}

/// A scaling strategy for an [`Image`] widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageScaling {
    /// Scales the image keeping the aspect ratio the same.
    Aspect {
        /// The strategy to use to pick a scaling factor.
        mode: Aspect,
        /// The orientation to either crop or align using.
        orientation: Size<ZeroToOne>,
    },

    /// The stretch scaling strategy stretches the image to fill the surface,
    /// ignoring the aspect ratio.
    Stretch,

    /// The image will be displayed at a scaling factor of the contained `f32`.
    /// In this mode, the widget will request that its size be the size of the
    /// contained image.
    Scale(f32),
}

impl Default for ImageScaling {
    /// Returns `ImageScaling::Scale(1.)`.
    fn default() -> Self {
        Self::Scale(1.)
    }
}

impl IntoValue<AnyTexture> for Texture {
    fn into_value(self) -> Value<AnyTexture> {
        Value::Constant(AnyTexture::from(self))
    }
}

impl IntoValue<AnyTexture> for LazyTexture {
    fn into_value(self) -> Value<AnyTexture> {
        Value::Constant(AnyTexture::from(self))
    }
}

impl IntoValue<AnyTexture> for SharedTexture {
    fn into_value(self) -> Value<AnyTexture> {
        Value::Constant(AnyTexture::from(self))
    }
}

impl IntoValue<AnyTexture> for CollectedTexture {
    fn into_value(self) -> Value<AnyTexture> {
        Value::Constant(AnyTexture::from(self))
    }
}

impl IntoValue<AnyTexture> for TextureRegion {
    fn into_value(self) -> Value<AnyTexture> {
        Value::Constant(AnyTexture::from(self))
    }
}

/// An aspect mode for scaling an [`Image`].
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum Aspect {
    /// The aspect-fit scaling strategy scales the image to be the largest size
    /// it can be without clipping.
    #[default]
    Fit,

    /// The aspect-fill scaling strategy scales the image to be the smallest
    /// size it can be to cover the entire surface.
    Fill,
}
