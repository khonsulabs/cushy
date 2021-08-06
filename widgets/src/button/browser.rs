use gooey_browser::{
    utils::{widget_css_id, window_document, CssRules},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{TransmogrifierContext, WidgetRef};
use wasm_bindgen::JsCast;
use web_sys::HtmlButtonElement;

use crate::button::{Button, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent};

impl gooey_core::Transmogrifier<WebSys> for ButtonTransmogrifier {
    type State = Option<CssRules>;
    type Widget = Button;

    fn receive_command(
        &self,
        command: ButtonCommand,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        let document = window_document();
        if let Some(element) = document
            .get_element_by_id(&widget_css_id(context.registration.id().id))
            .and_then(|e| e.dyn_into::<HtmlButtonElement>().ok())
        {
            let ButtonCommand::LabelChanged = command;
            element.set_inner_text(&context.widget.label);
        }
    }
}

impl WebSysTransmogrifier for ButtonTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let document = window_document();
        let element = document
            .create_element("button")
            .expect("couldn't create button")
            .unchecked_into::<HtmlButtonElement>();
        *context.state = self.initialize_widget_element(&element, &context);
        element.set_inner_text(&context.widget.label);

        let closure = WidgetClosure::new::<WebSys, Button, _>(
            WidgetRef::new(&context.registration, context.frontend.clone()).unwrap(),
            || InternalButtonEvent::Clicked,
        );
        element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
        Some(element.unchecked_into())
    }
}
