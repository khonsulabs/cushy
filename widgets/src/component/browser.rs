use gooey_browser::{WebSys, WebSysTransmogrifier};
use gooey_core::TransmogrifierContext;

use crate::component::{Behavior, ComponentTransmogrifier};

impl<B: Behavior> WebSysTransmogrifier for ComponentTransmogrifier<B> {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.content.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier.transmogrify(&mut child_context)
                },
            )
            .unwrap_or_default()
    }
}

impl<B: Behavior> From<ComponentTransmogrifier<B>> for gooey_browser::RegisteredTransmogrifier {
    fn from(transmogrifier: ComponentTransmogrifier<B>) -> Self {
        Self(std::boxed::Box::new(transmogrifier))
    }
}
