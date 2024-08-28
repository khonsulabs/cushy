use std::time::Duration;

use figures::units::Px;
use figures::{Size, Zero};

use crate::animation::{AnimationHandle, AnimationTarget, Spawn};
use crate::context::LayoutContext;
use crate::styles::components::{EasingIn, EasingOut};
use crate::value::{Dynamic, Generation, IntoDynamic, Source};
use crate::widget::{MakeWidget, WidgetInstance, WidgetRef, WrappedLayout, WrapperWidget};
use crate::ConstraintLimit;

/// A widget that collapses/hides its contents based on a [`Dynamic<bool>`].
#[derive(Debug)]
pub struct Collapse {
    child: WidgetRef,
    collapse: Dynamic<bool>,
    collapse_generation: Generation,
    size: Dynamic<Px>,
    collapse_animation: Option<CollapseAnimation>,
    vertical: bool,
}

impl Collapse {
    fn new(collapse: Dynamic<bool>, child: WidgetInstance, vertical: bool) -> Self {
        let collapse_generation = collapse.generation();
        Self {
            collapse,
            collapse_generation,
            child: WidgetRef::new(child),
            size: Dynamic::default(),
            vertical,
            collapse_animation: None,
        }
    }

    /// Returns a widget that collapses `child` vertically based on the dynamic
    /// boolean value.
    ///
    /// This widget will be collapsed when the dynamic contains `true`, and
    /// revealed when the dynamic contains `false`.
    pub fn vertical(collapse_when: impl IntoDynamic<bool>, child: impl MakeWidget) -> Self {
        Self::new(collapse_when.into_dynamic(), child.make_widget(), true)
    }

    /// Returns a widget that collapses `child` horizontally based on the
    /// dynamic boolean value.
    ///
    /// This widget will be collapsed when the dynamic contains `true`, and
    /// revealed when the dynamic contains `false`.
    pub fn horizontal(collapse_when: impl IntoDynamic<bool>, child: impl MakeWidget) -> Self {
        Self::new(collapse_when.into_dynamic(), child.make_widget(), false)
    }

    fn note_child_size(
        &mut self,
        size: Px,
        current_size: Px,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Px {
        context.invalidate_when_changed(&self.collapse);
        let (generation, collapse) = self.collapse.map_generational(|c| (c.generation(), *c));
        let (easing, target) = if collapse {
            (context.get(&EasingOut), Px::ZERO)
        } else {
            (context.get(&EasingIn), size)
        };
        match &self.collapse_animation {
            Some(state) if state.target == target => {}
            Some(_) if generation == self.collapse_generation => {
                // The resize happened from a reason other than our toggle.
                // Immediately apply it.
                let mut stored_size = self.size.lock();
                stored_size.prevent_notifications();
                *stored_size = target;
                return target;
            }
            _ => {
                // If this is our first setup, immediately give the child the
                // space they request.
                let duration = if self.collapse_animation.is_some() {
                    Duration::from_millis(250)
                } else {
                    Duration::ZERO
                };
                self.collapse_animation = Some(CollapseAnimation {
                    target,
                    _handle: self
                        .size
                        .transition_to(target)
                        .over(duration)
                        .with_easing(easing)
                        .spawn(),
                });
            }
        }
        self.collapse_generation = generation;
        current_size
    }
}

impl WrapperWidget for Collapse {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn position_child(
        &mut self,
        size: Size<Px>,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        let clip_size = self.size.get_tracking_invalidate(context);
        if self.vertical {
            let height = self.note_child_size(size.height, clip_size, context);

            Size::new(size.width, height)
        } else {
            let width = self.note_child_size(size.width, clip_size, context);

            Size::new(width, size.height)
        }
        .into()
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Collapse")
            .field("collapse", &self.collapse)
            .field("child", &self.child)
            .finish()
    }
}

#[derive(Debug)]
struct CollapseAnimation {
    target: Px,
    _handle: AnimationHandle,
}
