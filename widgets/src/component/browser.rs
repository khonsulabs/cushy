use gooey_browser::{
    utils::{initialize_widget_element, window_document},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::TransmogrifierContext;
use wasm_bindgen::JsCast;

use crate::component::{Behavior, Component, ComponentTransmogrifier};

impl<B: Behavior> WebSysTransmogrifier for ComponentTransmogrifier<B> {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let container = window_document()
            .create_element("div")
            .expect("error creating div")
            .unchecked_into::<web_sys::HtmlDivElement>();
        initialize_widget_element::<Component<B>>(&container, context.registration.id().id);
        if let Some(child) = context
            .frontend
            .with_transmogrifier(
                context.widget.content.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier.transmogrify(&mut child_context)
                },
            )
            .unwrap_or_default()
        {
            container.append_child(&child).unwrap();
        }
        Some(container.unchecked_into())
    }
}

impl<B: Behavior> From<ComponentTransmogrifier<B>> for gooey_browser::RegisteredTransmogrifier {
    fn from(transmogrifier: ComponentTransmogrifier<B>) -> Self {
        Self(std::boxed::Box::new(transmogrifier))
    }
}
