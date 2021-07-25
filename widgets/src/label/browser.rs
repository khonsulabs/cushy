use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::TransmogrifierContext;
use wasm_bindgen::JsCast;
use web_sys::HtmlDivElement;

use crate::label::{Label, LabelCommand, LabelTransmogrifier};

impl gooey_core::Transmogrifier<WebSys> for LabelTransmogrifier {
    type State = Option<CssRules>;
    type Widget = Label;

    fn receive_command(
        &self,
        command: LabelCommand,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        let document = window_document();
        if let Some(element) = document
            .get_element_by_id(&widget_css_id(context.registration.id().id))
            .and_then(|e| e.dyn_into::<HtmlDivElement>().ok())
        {
            let LabelCommand::LabelChanged = command;
            element.set_inner_text(&context.widget.label);
        }
    }
}

impl WebSysTransmogrifier for LabelTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let element = create_element::<HtmlDivElement>("div");
        *context.state = self.initialize_widget_element(&element, &context);
        element.set_inner_text(&context.widget.label);

        Some(element.unchecked_into())
    }
}
