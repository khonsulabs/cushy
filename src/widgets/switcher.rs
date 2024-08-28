use std::fmt::Debug;
use std::mem;

use ahash::HashMap;
use figures::Size;
use kludgine::KludgineId;

use crate::context::{AsEventContext, LayoutContext};
use crate::value::{Dynamic, DynamicReader, IntoDynamic, IntoReader, Source};
use crate::widget::{MountedWidget, WidgetInstance, WidgetRef, WrapperWidget};
use crate::window::WindowLocal;
use crate::ConstraintLimit;

/// A widget that switches its contents based on a value of `T`.
#[derive(Debug)]
pub struct Switcher {
    source: DynamicReader<WidgetInstance>,
    child: WidgetRef,
    pending_unmount: HashMap<KludgineId, MountedWidget>,
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
    pub fn new(source: impl IntoReader<WidgetInstance>) -> Self {
        let source = source.into_reader();
        let child = WidgetRef::new(source.get());
        Self {
            source,
            child,
            pending_unmount: HashMap::default(),
        }
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
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        if let Some(pending_unmount) = self.pending_unmount.remove(&context.kludgine_id()) {
            context.remove_child(&pending_unmount);
        }

        let current_source = self.source.get_tracking_invalidate(context);
        if &current_source != self.child.widget() {
            // immediately unmount in the current context.
            self.child.unmount_in(context);
            let old_mounts = <WindowLocal<MountedWidget>>::from(mem::replace(
                &mut self.child,
                WidgetRef::new(current_source),
            ));

            // For all other contexts, we have to wait until this callback to
            // try unmounting.
            for (id, mounted) in old_mounts {
                let existing = self.pending_unmount.insert(id, mounted);
                debug_assert!(
                    existing.is_none(),
                    "Existing unmount found, but should have already been unmounted"
                );
            }
        }

        context.invalidate_when_changed(&self.source);

        available_space
    }
}
