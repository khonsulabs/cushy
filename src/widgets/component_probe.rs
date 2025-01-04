use std::fmt::Debug;

use super::Space;
use crate::styles::{ComponentDefinition, ContextFreeComponent};
use crate::value::{Destination, Dynamic};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};

/// A widget that provides access to a [`ComponentDefinition`]'s value through a
/// [`Dynamic`].
///
/// This widget enables access to runtime values provided by the theme without
/// creating a custom widget. After creating a probe, [`value()`](Self::value)
/// can be used to observe and use the value.
///
/// The theme information retrieved will be the effective theme at the location
/// the probe is inserted in the widget hierarchy.
#[derive(Debug)]
pub struct ComponentProbe<Component>
where
    Component: ComponentDefinition,
{
    component: Component,
    probed: Dynamic<Component::ComponentType>,
    child: WidgetRef,
}

impl<Component> ComponentProbe<Component>
where
    Component: ComponentDefinition,
{
    /// Returns a new probe that provides access to the runtime value of
    /// `Component`.
    ///
    /// The initial contents of the dynamic will be `initial_value`.
    pub fn new(component: Component, initial_value: Component::ComponentType) -> Self {
        Self::new_wrapping(component, initial_value, Space::clear())
    }

    /// Returns a new probe wrapping `child` that provides access to the runtime
    /// value of this component.
    ///
    /// The initial contents of the dynamic will be `initial_value`.
    pub fn new_wrapping(
        component: Component,
        initial_value: Component::ComponentType,
        child: impl MakeWidget,
    ) -> Self {
        Self {
            component,
            probed: Dynamic::new(initial_value),
            child: WidgetRef::new(child),
        }
    }

    /// Returns a new probe that provides access to the runtime value of
    /// `Component`.
    pub fn default_for(component: Component) -> Self
    where
        Component: ContextFreeComponent,
    {
        let default = component.default();
        Self::new(component, default)
    }

    /// Returns a new probe wrapping `child` that provides access to the runtime
    /// value of this component.
    pub fn default_wrapping(component: Component, child: impl MakeWidget) -> Self
    where
        Component: ContextFreeComponent,
    {
        let default = component.default();
        Self::new_wrapping(component, default, child)
    }

    /// Returns the dynamic that contains the component's current value.
    ///
    /// This dynamic's contents will be updated whenever this probe is
    /// invalidated.
    pub const fn value(&self) -> &Dynamic<Component::ComponentType> {
        &self.probed
    }
}

impl<Component> WrapperWidget for ComponentProbe<Component>
where
    Component: ComponentDefinition + Debug + Send + 'static,
    Component::ComponentType: PartialEq + Debug + Send + 'static,
{
    fn child_mut(&mut self) -> &mut crate::widget::WidgetRef {
        &mut self.child
    }

    fn adjust_child_constraints(
        &mut self,
        available_space: figures::Size<crate::ConstraintLimit>,
        context: &mut crate::context::LayoutContext<'_, '_, '_, '_>,
    ) -> figures::Size<crate::ConstraintLimit> {
        self.probed.set(context.get(&self.component));
        available_space
    }

    fn redraw_foreground(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        self.probed.set(context.get(&self.component));
    }
}
