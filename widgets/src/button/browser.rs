use gooey_browser::{
    utils::{initialize_widget_element, widget_css_id, window_document, CssBlockBuilder, CssRule},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{
    styles::{BackgroundColor, Style, TextColor},
    TransmogrifierContext, WidgetRef,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlButtonElement;

use crate::button::{Button, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent};

impl gooey_core::Transmogrifier<WebSys> for ButtonTransmogrifier {
    type State = Option<CssRule>;
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
            let ButtonCommand::SetLabel(new_label) = command;
            element.set_inner_text(&new_label);
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
        initialize_widget_element::<Button>(&element, context.registration.id().id);
        element.set_inner_text(&context.widget.label);

        let closure = WidgetClosure::new::<WebSys, Button, _>(
            WidgetRef::new(&context.registration, context.frontend.clone()).unwrap(),
            || InternalButtonEvent::Clicked,
        );
        element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
        Some(element.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, mut css: CssBlockBuilder) -> CssBlockBuilder {
        if let Some(text_color) = style.get_with_fallback::<TextColor>() {
            css = css
                .with_css_statement(format!("color: {}", text_color.light_color.to_css_string()));
        }
        if let Some(text_color) = style.get_with_fallback::<BackgroundColor>() {
            css = css.with_css_statement(format!(
                "background-color: {}",
                text_color.light_color.to_css_string()
            ));
        }

        css.with_css_statement("border: none") // TODO support borders
    }
}
