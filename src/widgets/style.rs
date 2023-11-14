use crate::context::EventContext;
use crate::styles::Styles;
use crate::value::{IntoValue, Value};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};

/// A widget that applies a set of [`Styles`] to all contained widgets.
#[derive(Debug)]
pub struct Style {
    styles: Value<Styles>,
    child: WidgetRef,
}

impl Style {
    /// Returns a new widget that applies `styles` to `child` and any children
    /// it may have.
    pub fn new(styles: impl IntoValue<Styles>, child: impl MakeWidget) -> Self {
        Self {
            styles: styles.into_value(),
            child: WidgetRef::new(child),
        }
    }
}

impl WrapperWidget for Style {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {
        context.attach_styles(self.styles.clone());
    }
}
