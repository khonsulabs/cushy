use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, Rect, Size};

use crate::context::{AsEventContext, LayoutContext};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};
use crate::widgets::Space;
use crate::ConstraintLimit;

/// A widget that expands its child widget to fill the parent.
///
/// Some parent widgets support weighting children when there is more than one
/// [`Expand`]ed widget.
#[derive(Debug)]
pub struct Expand {
    /// The weight to use when splitting available space with multiple
    /// [`Expand`] widgets.
    pub weight: u8,
    child: WidgetRef,
}

impl Default for Expand {
    fn default() -> Self {
        Self::empty()
    }
}

impl Expand {
    /// Returns a widget that expands `child` to fill the parent widget.
    #[must_use]
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            weight: 1,
        }
    }

    /// Returns a widget that expands to fill its parent, but has no contents.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            child: WidgetRef::new(Space),
            weight: 1,
        }
    }

    /// Returns a widget that expands `child` to fill the parent widget, using
    /// `weight` when competing with available space with other [`Expand`]s.
    ///
    /// Note: Not all container widgets support weighted expansion.
    #[must_use]
    pub fn weighted(weight: u8, child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            weight,
        }
    }

    /// Returns a reference to the child widget.
    #[must_use]
    pub const fn child(&self) -> &WidgetRef {
        &self.child
    }
}

impl WrapperWidget for Expand {
    fn child(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Rect<Px> {
        let available_space = Size::new(
            ConstraintLimit::Known(available_space.width.max()),
            ConstraintLimit::Known(available_space.height.max()),
        );
        let child = self.child.mounted(&mut context.as_event_context());
        let size = context.for_other(&child).layout(available_space);

        Size::<UPx>::new(
            available_space
                .width
                .fit_measured(size.width, context.gfx.scale()),
            available_space
                .height
                .fit_measured(size.height, context.gfx.scale()),
        )
        .into_signed()
        .into()
    }
}
