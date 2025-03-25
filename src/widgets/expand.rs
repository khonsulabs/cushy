use figures::{IntoSigned, Size};

use crate::context::{AsEventContext, EventContext, LayoutContext};
use crate::widget::{MakeWidget, RootBehavior, WidgetRef, WrappedLayout, WrapperWidget};
use crate::widgets::Space;
use crate::ConstraintLimit;

/// A widget that expands its child widget to fill the parent.
///
/// Some parent widgets support weighting children when there is more than one
/// [`Expand`]ed widget.
#[derive(Debug, Clone)]
pub struct Expand {
    kind: ExpandKind,
    child: WidgetRef,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExpandKind {
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

    /// Returns a widget that expands to fill its parent horizontally, but has no contents.
    #[must_use]
    pub fn empty_horizontally() -> Self {
        Self {
            child: WidgetRef::new(Space::clear()),
            kind: ExpandKind::Horizontal,
        }
    }

    /// Returns a widget that expands to fill its parent vertically, but has no contents.
    #[must_use]
    pub fn empty_vertically() -> Self {
        Self {
            child: WidgetRef::new(Space::clear()),
            kind: ExpandKind::Vertical,
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
    pub(crate) fn weight(&self, vertical: bool) -> Option<u8> {
        match (self.kind, vertical) {
            (ExpandKind::Weighted(weight), _) => Some(weight),
            (ExpandKind::Horizontal, false) | (ExpandKind::Vertical, true) => Some(1),
            (ExpandKind::Horizontal | ExpandKind::Vertical, _) => None,
        }
    }

    pub(crate) const fn expand_kind(&self) -> ExpandKind {
        self.kind
    }
}

impl WrapperWidget for Expand {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn root_behavior(&mut self, _context: &mut EventContext<'_>) -> Option<RootBehavior> {
        Some(RootBehavior::Expand)
    }

    fn align_child(&self) -> bool {
        true
    }

    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        let available_space = match &self.kind {
            ExpandKind::Weighted(_) => available_space.map(|lim| ConstraintLimit::Fill(lim.max())),
            ExpandKind::Horizontal => Size::new(
                ConstraintLimit::Fill(available_space.width.max()),
                ConstraintLimit::SizeToFit(available_space.height.max()),
            ),
            ExpandKind::Vertical => Size::new(
                ConstraintLimit::SizeToFit(available_space.width.max()),
                ConstraintLimit::Fill(available_space.height.max()),
            ),
        };
        let child = self.child.mounted(&mut context.as_event_context());
        let layout = context.for_other(&child).layout(available_space);

        let (width, height) = match &self.kind {
            ExpandKind::Weighted(_) => (
                available_space.width.fit_measured(layout.size.width),
                available_space.height.fit_measured(layout.size.height),
            ),
            ExpandKind::Horizontal => (
                available_space.width.fit_measured(layout.size.width),
                layout.size.height.min(available_space.height.max()),
            ),
            ExpandKind::Vertical => (
                layout.size.width.min(available_space.width.max()),
                available_space.height.fit_measured(layout.size.height),
            ),
        };

        let size = Size::new(width, height);
        WrappedLayout {
            child: size.into_signed().into(),
            size,
            baseline: layout.baseline,
        }
    }
}
