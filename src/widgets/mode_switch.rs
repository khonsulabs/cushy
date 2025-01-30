use crate::context::EventContext;
use crate::reactive::value::{IntoValue, Value};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};
use crate::window::ThemeMode;

/// A widget that applies a set of [`ThemeMode`] to all contained widgets.
#[derive(Debug)]
pub struct ThemedMode {
    mode: Value<ThemeMode>,
    child: WidgetRef,
}

impl ThemedMode {
    /// Returns a new widget that applies `mode` to all of its children.
    pub fn new(mode: impl IntoValue<ThemeMode>, child: impl MakeWidget) -> Self {
        Self {
            mode: mode.into_value(),
            child: WidgetRef::new(child),
        }
    }
}

impl WrapperWidget for ThemedMode {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        context.attach_theme_mode(self.mode.clone());
    }
}
