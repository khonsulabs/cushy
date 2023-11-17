use kludgine::figures::{Fraction, IntoSigned, ScreenScale, Size};

use crate::context::{AsEventContext, LayoutContext};
use crate::styles::DimensionRange;
use crate::widget::{MakeWidget, WidgetRef, WrappedLayout, WrapperWidget};
use crate::ConstraintLimit;

/// A widget that resizes its contained widget to an explicit size.
#[derive(Debug)]
pub struct Resize {
    /// The range of allowed width for the child widget.
    pub width: DimensionRange,
    /// The range of allowed height for the child widget.
    pub height: DimensionRange,
    child: WidgetRef,
}

impl Resize {
    /// Returns a reference to the child widget.
    #[must_use]
    pub fn child(&self) -> &WidgetRef {
        &self.child
    }

    /// Resizes `child` to `size`.
    #[must_use]
    pub fn to<T>(size: Size<T>, child: impl MakeWidget) -> Self
    where
        T: Into<DimensionRange>,
    {
        Self {
            child: WidgetRef::new(child),
            width: size.width.into(),
            height: size.height.into(),
        }
    }

    /// Resizes `child`'s width to `width`.
    #[must_use]
    pub fn from_width(width: impl Into<DimensionRange>, child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            width: width.into(),
            height: DimensionRange::from(..),
        }
    }

    /// Resizes `self` to `width`.
    ///
    /// `width` can be an any of:
    ///
    /// - [`Dimension`](crate::styles::Dimension)
    /// - [`Px`](crate::kludgine::figures::units::Px)
    /// - [`Lp`](crate::kludgine::figures::units::Lp)
    /// - A range of any fo the above.
    #[must_use]
    pub fn width(mut self, width: impl Into<DimensionRange>) -> Self {
        self.width = width.into();
        self
    }

    /// Resizes `self` to `height`.
    ///
    /// `height` can be an any of:
    ///
    /// - [`Dimension`](crate::styles::Dimension)
    /// - [`Px`](crate::kludgine::figures::units::Px)
    /// - [`Lp`](crate::kludgine::figures::units::Lp)
    /// - A range of any fo the above.
    #[must_use]
    pub fn height(mut self, height: impl Into<DimensionRange>) -> Self {
        self.height = height.into();
        self
    }

    /// Resizes `child`'s height to `height`.
    #[must_use]
    pub fn from_height(height: impl Into<DimensionRange>, child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            width: DimensionRange::from(..),
            height: height.into(),
        }
    }
}

impl WrapperWidget for Resize {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> WrappedLayout {
        let child = self.child.mounted(&mut context.as_event_context());
        let size = if let (Some(width), Some(height)) =
            (self.width.exact_dimension(), self.height.exact_dimension())
        {
            Size::new(
                width.into_upx(context.gfx.scale()),
                height.into_upx(context.gfx.scale()),
            )
        } else {
            let available_space = Size::new(
                override_constraint(available_space.width, self.width, context.gfx.scale()),
                override_constraint(available_space.height, self.height, context.gfx.scale()),
            );
            context.for_other(&child).layout(available_space)
        };
        Size::new(
            self.width.clamp(size.width, context.gfx.scale()),
            self.height.clamp(size.height, context.gfx.scale()),
        )
        .into_signed()
        .into()
    }
}

fn override_constraint(
    constraint: ConstraintLimit,
    range: DimensionRange,
    scale: Fraction,
) -> ConstraintLimit {
    match constraint {
        ConstraintLimit::Fill(size) => ConstraintLimit::Fill(range.clamp(size, scale)),
        ConstraintLimit::SizeToFit(clipped_after) => match (range.minimum(), range.maximum()) {
            (Some(min), Some(max)) if min == max => ConstraintLimit::Fill(min.into_upx(scale)),
            _ => ConstraintLimit::SizeToFit(range.clamp(clipped_after, scale)),
        },
    }
}
