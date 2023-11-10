use kludgine::figures::units::UPx;
use kludgine::figures::{Fraction, IntoSigned, IntoUnsigned, Rect, ScreenScale, Size};

use crate::context::{AsEventContext, LayoutContext};
use crate::styles::DimensionRange;
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};
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
    pub fn width(width: impl Into<DimensionRange>, child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            width: width.into(),
            height: DimensionRange::from(..),
        }
    }

    /// Resizes `child`'s height to `height`.
    #[must_use]
    pub fn height(height: impl Into<DimensionRange>, child: impl MakeWidget) -> Self {
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
    ) -> Rect<kludgine::figures::units::Px> {
        let child = self.child.mounted(&mut context.as_event_context());
        let size = if let (Some(width), Some(height)) =
            (self.width.exact_dimension(), self.height.exact_dimension())
        {
            Size::new(
                width.into_px(context.gfx.scale()).into_unsigned(),
                height.into_px(context.gfx.scale()).into_unsigned(),
            )
        } else {
            let available_space = Size::new(
                override_constraint(available_space.width, self.width, context.gfx.scale()),
                override_constraint(available_space.height, self.height, context.gfx.scale()),
            );
            context.for_other(&child).layout(available_space)
        };
        Size::<UPx>::new(
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
        ConstraintLimit::Known(size) => ConstraintLimit::Known(range.clamp(size, scale)),
        ConstraintLimit::ClippedAfter(clipped_after) => match (range.minimum(), range.maximum()) {
            (Some(min), Some(max)) if min == max => {
                ConstraintLimit::Known(min.into_px(scale).into_unsigned())
            }
            _ => ConstraintLimit::ClippedAfter(range.clamp(clipped_after, scale)),
        },
    }
}
