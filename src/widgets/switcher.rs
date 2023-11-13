use std::fmt::Debug;
use std::panic::UnwindSafe;

use kludgine::figures::Size;

use crate::context::{AsEventContext, LayoutContext};
use crate::value::{Generation, IntoValue, Value};
use crate::widget::{MakeWidget, WidgetInstance, WidgetRef, WrapperWidget};
use crate::ConstraintLimit;

/// A widget that switches its contents based on a value of `T`.
pub struct Switcher<T> {
    value: Value<T>,
    value_generation: Option<Generation>,
    factory: Box<dyn SwitchMap<T>>,
    child: WidgetRef,
}

impl<T> Switcher<T> {
    /// Returns a new widget that replaces its contents with the result of
    /// `widget_factory` each time `value` changes.
    #[must_use]
    pub fn new<W, F>(value: impl IntoValue<T>, mut widget_factory: F) -> Self
    where
        F: for<'a> FnMut(&'a T) -> W + Send + UnwindSafe + 'static,
        W: MakeWidget,
    {
        let value = value.into_value();
        let value_generation = value.generation();
        let child = WidgetRef::new(value.map(|value| widget_factory(value)));
        Self {
            value,
            value_generation,
            factory: Box::new(widget_factory),
            child,
        }
    }
}

impl<T> Debug for Switcher<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switcher")
            .field("value", &self.value)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T> WrapperWidget for Switcher<T>
where
    T: Debug + Send + UnwindSafe + 'static,
{
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    // TODO this should be moved to an invalidated() event once we have it.
    fn adjust_child_constraint(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        let current_generation = self.value.generation();
        if self.value_generation != current_generation {
            self.value_generation = current_generation;
            let new_child = WidgetRef::new(self.value.map(|value| self.factory.invoke(value)));
            let removed = std::mem::replace(&mut self.child, new_child);
            if let WidgetRef::Mounted(removed) = removed {
                context.remove_child(&removed);
            }
        }
        available_space
    }
}

trait SwitchMap<T>: UnwindSafe + Send {
    fn invoke(&mut self, value: &T) -> WidgetInstance;
}

impl<W, T, F> SwitchMap<T> for F
where
    F: for<'a> FnMut(&'a T) -> W + Send + UnwindSafe,
    W: MakeWidget,
{
    fn invoke(&mut self, value: &T) -> WidgetInstance {
        self(value).make_widget()
    }
}
