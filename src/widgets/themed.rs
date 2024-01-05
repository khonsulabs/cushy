use crate::context::EventContext;
use crate::styles::ThemePair;
use crate::value::{IntoValue, Value};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};

/// A widget that applies a [`ThemePair`] to all contained widgets.
#[derive(Debug)]
pub struct Themed {
    theme: Value<ThemePair>,
    child: WidgetRef,
}

impl Themed {
    /// Returns a new widget that applies `theme` to all of its children.
    pub fn new(theme: impl IntoValue<ThemePair>, child: impl MakeWidget) -> Self {
        Self {
            theme: theme.into_value(),
            child: WidgetRef::new(child),
        }
    }
}

impl WrapperWidget for Themed {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        context.attach_theme(self.theme.clone());
    }
}
