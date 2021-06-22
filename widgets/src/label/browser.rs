use gooey_browser::{
    utils::{widget_css_id, window_document, CssBlockBuilder, CssRule},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{styles::Style, TransmogrifierContext};
use wasm_bindgen::JsCast;
use web_sys::HtmlDivElement;

use crate::label::{Label, LabelCommand, LabelTransmogrifier};

impl gooey_core::Transmogrifier<WebSys> for LabelTransmogrifier {
    type State = Option<CssRule>;
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
        let document = window_document();
        let element = document
            .create_element("div")
            .expect("couldn't create div")
            .unchecked_into::<HtmlDivElement>();
        *context.state = self.initialize_widget_element(&element, &context);
        element.set_inner_text(&context.widget.label);

        Some(element.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_colors_to_css(style, css)
    }
}
