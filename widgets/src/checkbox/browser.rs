use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{styles::Style, TransmogrifierContext, WidgetRef};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, HtmlInputElement, HtmlLabelElement};

use crate::checkbox::{
    Checkbox, CheckboxCommand, CheckboxTransmogrifier, InternalCheckboxEvent, LABEL_PADDING,
};

impl gooey_core::Transmogrifier<WebSys> for CheckboxTransmogrifier {
    type State = Option<CssRules>;
    type Widget = Checkbox;

    fn receive_command(
        &self,
        command: CheckboxCommand,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        match command {
            CheckboxCommand::Toggled => {
                if let Some(input) = window_document().get_element_by_id(&format!(
                    "{}-input",
                    widget_css_id(context.registration.id().id)
                )) {
                    let input = input.unchecked_into::<HtmlInputElement>();
                    input.set_checked(context.widget.checked);
                }
            }
            CheckboxCommand::LabelChanged => {
                if let Some(span) = window_document().get_element_by_id(&format!(
                    "{}-label",
                    widget_css_id(context.registration.id().id)
                )) {
                    let span = span.unchecked_into::<HtmlDivElement>();
                    span.set_inner_text(&context.widget.label);
                }
            }
        }
    }
}

impl WebSysTransmogrifier for CheckboxTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        // Create this html layout: <label><input /><div /></label>
        let container = create_element::<HtmlLabelElement>("label");
        let input = create_element::<HtmlInputElement>("input");
        let label = create_element::<HtmlDivElement>("div");
        let input_id = format!("{}-input", widget_css_id(context.registration.id().id));
        input.set_id(&input_id);
        input.set_type("checkbox");
        container.append_child(&input).unwrap();

        let label_id = format!("{}-label", widget_css_id(context.registration.id().id));
        label.set_id(&label_id);
        label.set_inner_text(&context.widget.label());
        container.append_child(&label).unwrap();

        let mut css = self
            .initialize_widget_element(&container, &context)
            .unwrap_or_default();
        css = css
            .and(
                &CssBlockBuilder::for_id(context.registration.id().id)
                    .with_css_statement("display: flex")
                    .with_css_statement("align-items: start")
                    .to_string(),
            )
            .and(
                &CssBlockBuilder::for_css_selector(&format!("#{}", input_id))
                    .with_css_statement(format!("margin-right: {:.03}pt", LABEL_PADDING.get()))
                    .to_string(),
            );
        *context.state = Some(css);

        let closure = WidgetClosure::new::<WebSys, Checkbox, _>(
            WidgetRef::new(&context.registration, context.frontend.clone()).unwrap(),
            || InternalCheckboxEvent::Clicked,
        );
        input.set_oninput(Some(closure.into_js_value().unchecked_ref()));
        Some(container.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_standard_components_to_css(style, css)
            .with_css_statement("border: none") // TODO support borders
    }
}
