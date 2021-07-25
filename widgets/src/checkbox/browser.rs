use gooey_browser::{
    utils::{widget_css_id, window_document, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{styles::Style, TransmogrifierContext, WidgetRef};
use wasm_bindgen::JsCast;
use web_sys::{HtmlButtonElement, HtmlInputElement};

use super::CheckboxTransmogrifier;
use crate::button::{Button, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent};

impl gooey_core::Transmogrifier<WebSys> for CheckboxTransmogrifier {
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
            // element.set_inner_text(&context.widget.label);
        }
    }
}

impl WebSysTransmogrifier for CheckboxTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let document = window_document();
        let element = document
            .create_element("input")
            .expect("couldn't create button")
            .unchecked_into::<HtmlInputElement>();
        *context.state = self.initialize_widget_element(&element, &context);
        // element.set_inner_text(&context.widget.label);
        // element.set_type("checkbox");

        let closure = WidgetClosure::new::<WebSys, Button, _>(
            WidgetRef::new(&context.registration, context.frontend.clone()).unwrap(),
            || InternalButtonEvent::Clicked,
        );
        element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
        Some(element.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_standard_components_to_css(style, css)
            .with_css_statement("border: none") // TODO support borders
    }
}
