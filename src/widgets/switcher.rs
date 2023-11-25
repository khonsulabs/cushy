use std::fmt::Debug;

use kludgine::figures::Size;

use crate::context::{AsEventContext, LayoutContext};
use crate::value::{Dynamic, DynamicReader, IntoDynamic};
use crate::widget::{WidgetInstance, WidgetRef, WrapperWidget};
use crate::ConstraintLimit;

/// A widget that switches its contents based on a value of `T`.
#[derive(Debug)]
pub struct Switcher {
    source: DynamicReader<WidgetInstance>,
    child: WidgetRef,
}

impl Switcher {
    /// Returns a new widget that replaces its contents with the results of
    /// calling `map` each time `source` is updated.
    ///
    /// This function is equivalent to calling
    /// `Self::new(source.into_dynamic().map_each(map))`, but this function's
    /// signature helps the compiler's type inference work correctly. When using
    /// new directly, the compiler often requires annotating the closure's
    /// argument type.
    pub fn mapping<T, F>(source: impl IntoDynamic<T>, mut map: F) -> Self
    where
        F: FnMut(&T, &Dynamic<T>) -> WidgetInstance + Send + 'static,
        T: Send + 'static,
    {
        let source = source.into_dynamic();

        Self::new(source.clone().map_each(move |value| map(value, &source)))
    }

    /// Returns a new widget that replaces its contents with the result of
    /// `widget_factory` each time `value` changes.
    #[must_use]
    pub fn new(source: impl IntoDynamic<WidgetInstance>) -> Self {
        let mut source = source.into_dynamic().into_reader();
        let child = WidgetRef::new(source.get());
        Self { source, child }
    }
}

impl WrapperWidget for Switcher {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    // TODO this should be moved to an invalidated() event once we have it.
    fn adjust_child_constraints(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        if self.source.has_updated() {
            let removed = std::mem::replace(&mut self.child, WidgetRef::new(self.source.get()));
            if let WidgetRef::Mounted(removed) = removed {
                context.remove_child(&removed);
            }
        }
        context.invalidate_when_changed(&self.source);
        available_space
    }
}
