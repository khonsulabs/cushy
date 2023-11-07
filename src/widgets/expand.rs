use kludgine::figures::units::UPx;
use kludgine::figures::{IntoSigned, Rect, Size};

use crate::context::{AsEventContext, GraphicsContext, LayoutContext};
use crate::widget::{MakeWidget, Widget, WidgetRef};
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

impl Expand {
    /// Returns a widget that expands `child` to fill the parent widget.
    #[must_use]
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
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
    pub fn child(&self) -> &WidgetRef {
        &self.child
    }
}

impl Widget for Expand {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let child = self.child.mounted(&mut context.as_event_context());
        context.for_other(&child).redraw();
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let available_space = Size::new(
            ConstraintLimit::Known(available_space.width.max()),
            ConstraintLimit::Known(available_space.height.max()),
        );
        let child = self.child.mounted(&mut context.as_event_context());
        let size = context.for_other(&child).layout(available_space);
        context.set_child_layout(&child, Rect::from(size.into_signed()));
        size
    }
}
