use kludgine::figures::{IntoSigned, Size};

use crate::context::{AsEventContext, LayoutContext};
use crate::widget::{MakeWidget, WidgetRef, WrappedLayout, WrapperWidget};
use crate::widgets::Space;
use crate::ConstraintLimit;

/// A widget that expands its child widget to fill the parent.
///
/// Some parent widgets support weighting children when there is more than one
/// [`Expand`]ed widget.
#[derive(Debug)]
pub struct Expand {
    kind: ExpandKind,
    child: WidgetRef,
}

#[derive(Debug)]
enum ExpandKind {
    Weighted(u8),
    Horizontal,
    Vertical,
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
            kind: ExpandKind::Weighted(1),
        }
    }

    /// Returns a widget that expands `child` to fill the parent widget horizontally.
    #[must_use]
    pub fn horizontal(child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            kind: ExpandKind::Horizontal,
        }
    }

    /// Returns a widget that expands `child` to fill the parent widget vertically.
    #[must_use]
    pub fn vertical(child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            kind: ExpandKind::Vertical,
        }
    }

    /// Returns a widget that expands to fill its parent, but has no contents.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            child: WidgetRef::new(Space::clear()),
            kind: ExpandKind::Weighted(1),
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
            kind: ExpandKind::Weighted(weight),
        }
    }

    /// Returns a reference to the child widget.
    #[must_use]
    pub const fn child(&self) -> &WidgetRef {
        &self.child
    }

    #[must_use]
    pub(crate) fn weight(&self) -> Option<u8> {
        match self.kind {
            ExpandKind::Weighted(weight) => Some(weight),
            ExpandKind::Horizontal | ExpandKind::Vertical => None,
        }
    }
}

impl WrapperWidget for Expand {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> WrappedLayout {
        let available_space = available_space.map(|lim| ConstraintLimit::Fill(lim.max()));
        let child = self.child.mounted(&mut context.as_event_context());
        let size = context.for_other(&child).layout(available_space);

        let (width, height) = match &self.kind {
            ExpandKind::Weighted(_) => (
                available_space
                    .width
                    .fit_measured(size.width, context.gfx.scale()),
                available_space
                    .height
                    .fit_measured(size.height, context.gfx.scale()),
            ),
            ExpandKind::Horizontal => (
                available_space
                    .width
                    .fit_measured(size.width, context.gfx.scale()),
                size.height,
            ),
            ExpandKind::Vertical => (
                size.width,
                available_space
                    .height
                    .fit_measured(size.height, context.gfx.scale()),
            ),
        };

        Size::new(width, height).into_signed().into()
    }
}
