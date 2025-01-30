//! A widget that displays an image/texture.

use figures::units::{Px, UPx};
use figures::{FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use kludgine::shapes::{CornerRadii, Shape};
use kludgine::{
    AnyTexture, CollectedTexture, Color, LazyTexture, SharedTexture, Texture, TextureRegion,
};

use crate::animation::ZeroToOne;
use crate::context::{LayoutContext, Trackable};
use crate::reactive::value::{IntoValue, Source, Value};
use crate::styles::Dimension;
use crate::widget::Widget;
use crate::ConstraintLimit;

/// A widget that displays an image/texture.
#[derive(Debug)]
pub struct Image {
    /// The texture to render.
    pub contents: Value<AnyTexture>,
    /// The scaling strategy to apply.
    pub scaling: Value<ImageScaling>,
    /// The opacity to render the image with.
    pub opacity: Value<ZeroToOne>,
}

impl Image {
    /// Returns a new image widget that renders `contents`, using the default
    /// [`ImageScaling`] strategy.
    pub fn new(contents: impl IntoValue<AnyTexture>) -> Self {
        Self {
            contents: contents.into_value(),
            scaling: Value::default(),
            opacity: Value::Constant(ZeroToOne::ONE),
        }
    }

    /// Applies the `scaling` strategies and returns self.
    #[must_use]
    pub fn scaling(mut self, scaling: impl IntoValue<ImageScaling>) -> Self {
        self.scaling = scaling.into_value();
        self
    }

    /// Applies `opacity` when drawing the image, returns self.
    #[must_use]
    pub fn opacity(mut self, opacity: impl IntoValue<ZeroToOne>) -> Self {
        self.opacity = opacity.into_value();
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
}

impl Widget for Image {
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        self.contents.invalidate_when_changed(context);
        let opacity = self.opacity.get_tracking_redraw(context);
        let radii = context.get(&ImageCornerRadius);
        let radii = radii.map(|r| r.into_px(context.gfx.scale()));
        let scaling = self.scaling.get_tracking_invalidate(context);

        self.contents.map(|texture| {
            let rect = scaling.render_area(texture.size(), context.gfx.size());
            if radii.is_zero() {
                context.gfx.draw_texture(texture, rect, opacity);
            } else {
                context.gfx.draw_textured_shape(
                    &Shape::textured_round_rect(
                        rect,
                        radii,
                        Rect::from(texture.size()),
                        Color::WHITE,
                    ),
                    texture,
                    opacity,
                );
            }
        });
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let scaling = self.scaling.get_tracking_invalidate(context);
        self.contents
            .map(|texture| scaling.layout_size(texture.size(), available_space))
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

impl ImageScaling {
    /// Returns the size that should be occupied given an image size and
    /// available space constraints.
    #[must_use]
    pub fn layout_size(
        &self,
        image_size: Size<UPx>,
        available_space: Size<ConstraintLimit>,
    ) -> Size<UPx> {
        let desired_size = self
            .render_area(image_size, available_space.map(ConstraintLimit::max))
            .size
            .into_unsigned();
        if matches!(self, ImageScaling::Aspect { .. }) {
            // If we're in aspect mode and we're expected to fill in a given
            // dimension, we need to return the fill size during layout to allow
            // the aspect orientation to be applied.
            Size::new(
                match available_space.width {
                    ConstraintLimit::Fill(width) => width,
                    ConstraintLimit::SizeToFit(_) => desired_size.width,
                },
                match available_space.height {
                    ConstraintLimit::Fill(height) => height,
                    ConstraintLimit::SizeToFit(_) => desired_size.height,
                },
            )
        } else {
            desired_size
        }
    }

    /// Returns the area inside of `available_space` that an image of the given
    /// size should be drawn.
    #[must_use]
    pub fn render_area(&self, image_size: Size<UPx>, available_space: Size<UPx>) -> Rect<Px> {
        let image_size = image_size.into_signed();
        let available_space = available_space.into_signed();
        match self {
            ImageScaling::Aspect { mode, orientation } => {
                let scale_width =
                    available_space.width.into_float() / image_size.width.into_float();
                let scale_height =
                    available_space.height.into_float() / image_size.height.into_float();

                let effective_scale = match mode {
                    Aspect::Fill => scale_width.max(scale_height),
                    Aspect::Fit => scale_width.min(scale_height),
                };
                let scaled = image_size * effective_scale;

                let x = (available_space.width - scaled.width) * *orientation.width;
                let y = (available_space.height - scaled.height) * *orientation.height;

                Rect::new(Point::new(x, y), scaled)
            }
            ImageScaling::Stretch => available_space.into(),
            ImageScaling::Scale(factor) => {
                let size = image_size.map(|px| px * *factor);
                size.into()
            }
        }
    }
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

define_components! {
    Image {
        /// The corner radius to use to clip when rendering an [`Image`].
        ImageCornerRadius(CornerRadii<Dimension>, "corner_radius", CornerRadii::ZERO)
    }
}
