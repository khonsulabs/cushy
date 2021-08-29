use std::fmt::Write;

use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{
    styles::{FontFamily, FontSize, Style, SystemTheme},
    Frontend, TransmogrifierContext,
};
use gooey_text::{Span, Text};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, HtmlElement, HtmlSpanElement};

use super::LabelColor;
use crate::label::{Command, Label, LabelTransmogrifier};

impl gooey_core::Transmogrifier<WebSys> for LabelTransmogrifier {
    type State = Option<CssRules>;
    type Widget = Label;

    fn receive_command(
        &self,
        command: Command,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        let document = window_document();
        if let Some(element) = document
            .get_element_by_id(&widget_css_id(context.registration.id().id))
            .and_then(|e| e.dyn_into::<HtmlDivElement>().ok())
        {
            let Command::LabelChanged = command;
            // Clear the content
            element.set_inner_text("");
            text_to_html(
                &context.widget.label,
                &element,
                context.style(),
                context.frontend.theme(),
            );
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
        text_to_html(
            &context.widget.label,
            &element,
            context.style(),
            context.frontend.theme(),
        );

        Some(element.unchecked_into())
    }
}

fn text_to_html(text: &Text, target: &HtmlElement, element_style: &Style, theme: SystemTheme) {
    for span in text.iter() {
        span_to_html(span, target, element_style, theme);
    }
}

fn span_to_html(span: &Span, target: &HtmlElement, element_style: &Style, theme: SystemTheme) {
    let element = create_element::<HtmlSpanElement>("span");
    element.set_inner_text(span.text());
    let mut style = String::new();
    if let Some(color) = span
        .style
        .get_with_fallback::<LabelColor>()
        .or_else(|| element_style.get_with_fallback::<LabelColor>())
    {
        write!(
            &mut style,
            "color: {};",
            color.themed_color(theme).as_css_string()
        )
        .unwrap();
    }

    if let Some(size) = span
        .style
        .get::<FontSize>()
        .or_else(|| element_style.get::<FontSize>())
    {
        write!(&mut style, "font-size: {:.03}pt;", size.get()).unwrap();
    }

    if let Some(family) = span
        .style
        .get::<FontFamily>()
        .or_else(|| element_style.get::<FontFamily>())
    {
        write!(&mut style, "font-family: {};", family.0).unwrap();
    }

    element.style().set_css_text(&style);
    target.append_child(&element).unwrap();
}
