use unic_langid::LanguageIdentifier;
use crate::context::EventContext;
use crate::value::{IntoValue, Value};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};

/// A widget that applies a [`LanguageIdentifier`] to all contained widgets.
#[derive(Debug)]
pub struct Localized {
    locale: Value<LanguageIdentifier>,
    child: WidgetRef,
}

impl Localized {
    /// Returns a new widget that applies `locale` to all of its children.
    pub fn new(locale: impl IntoValue<LanguageIdentifier>, child: impl MakeWidget) -> Self {
        Self {
            locale: locale.into_value(),
            child: WidgetRef::new(child),
        }
    }
}

impl WrapperWidget for Localized {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        context.attach_locale(self.locale.clone());
    }
}
