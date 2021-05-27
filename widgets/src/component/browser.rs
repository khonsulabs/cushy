use gooey_browser::{AnyWidgetWebSysTransmogrifier, WebSys, WebSysTransmogrifier};
use gooey_core::Transmogrifier;

use crate::component::{Behavior, ComponentTransmogrifier};

impl<B: Behavior> WebSysTransmogrifier for ComponentTransmogrifier<B> {
    fn transmogrify(
        &self,
        _state: &Self::State,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        gooey
            .ui
            .with_transmogrifier(
                widget.content.id(),
                gooey,
                |child_transmogrifier, child_state, child_widget| {
                    child_transmogrifier.transmogrify(child_state, child_widget, gooey)
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
