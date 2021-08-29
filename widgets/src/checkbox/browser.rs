use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{
    styles::{style_sheet::State, ForegroundColor, SystemTheme},
    Frontend, TransmogrifierContext, WidgetRef,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, HtmlInputElement, HtmlLabelElement};

use crate::{
    button::ButtonColor,
    checkbox::{
        Checkbox, CheckboxCommand, CheckboxTransmogrifier, InternalCheckboxEvent, LABEL_PADDING,
    },
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

                if let Some(check) = window_document().get_element_by_id(&format!(
                    "{}-check",
                    widget_css_id(context.registration.id().id)
                )) {
                    let check = check.unchecked_into::<HtmlDivElement>();

                    check.set_class_name(if context.widget.checked {
                        "checked"
                    } else {
                        ""
                    });
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
        // This is a custom checkbox implementation that allows the input
        // element to still be visible for accessibility purposes, but renders
        // the state of the checkbox using a div that hides itself from
        // accessibility.
        let container = create_element::<HtmlLabelElement>("label");

        // The <input> field is marked with the class "sr-only" which ensures it
        // remains visible to screen readers, but will be hidden visually.
        let input = create_element::<HtmlInputElement>("input");
        let input_id = format!("{}-input", widget_css_id(context.registration.id().id));
        input.set_id(&input_id);
        input.set_type("checkbox");
        input.set_class_name("sr-only");
        container.append_child(&input).unwrap();

        // The visual checkbox is simply a div with another div inside representing the check.
        let checkbox = create_element::<HtmlDivElement>("div");
        let checkbox_id = format!("{}-checkbox", widget_css_id(context.registration.id().id));
        checkbox.set_id(&checkbox_id);
        checkbox.set_attribute("aria-hidden", "true").unwrap();

        let check = create_element::<HtmlDivElement>("div");
        let check_id = format!("{}-check", widget_css_id(context.registration.id().id));
        check.set_id(&check_id);
        check.set_attribute("aria-hidden", "true").unwrap();
        checkbox.append_child(&check).unwrap();
        container.append_child(&checkbox).unwrap();

        // The label is contained within a div to ensure wrapping doesn't go below the checkbox itself.
        let label = create_element::<HtmlDivElement>("div");
        let label_id = format!("{}-label", widget_css_id(context.registration.id().id));
        label.set_id(&label_id);
        label.set_inner_text(context.widget.label());
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
                &CssBlockBuilder::for_css_selector(&format!("#{}", checkbox_id))
                    .with_css_statement("width: 1em")
                    .with_css_statement("height: 1em")
                    .with_css_statement("display: flex")
                    .with_css_statement("justify-content: center")
                    .with_css_statement("align-items: center")
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

    fn additional_css_rules(
        &self,
        theme: SystemTheme,
        state: &State,
        context: &TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<CssRules> {
        let state_style = context
            .frontend
            .gooey()
            .stylesheet()
            .effective_style_for::<Checkbox>(context.style().clone(), state);
        let mut css = CssRules::default();
        if let Some(button_color) = state_style.get_with_fallback::<ButtonColor>() {
            let button_color = button_color.themed_color(theme);
            css = css.and(
                &CssBlockBuilder::for_css_selector(&format!(
                    "#{}-input",
                    widget_css_id(context.registration.id().id)
                ))
                .and_state(state)
                .and_additional_selector(&format!(
                    " + #{}-checkbox",
                    widget_css_id(context.registration.id().id)
                ))
                .with_theme(theme)
                .with_css_statement(format!(
                    "background-color: {}",
                    button_color.as_css_string()
                ))
                .to_string(),
            );
        }

        if let Some(foreground) = state_style.get_with_fallback::<ForegroundColor>() {
            let foreground = foreground.themed_color(theme);
            css = css.and(
                &CssBlockBuilder::for_css_selector(&format!(
                    "#{}-check.checked",
                    widget_css_id(context.registration.id().id)
                ))
                .with_theme(theme)
                .with_css_statement("width: .33em")
                .with_css_statement("height: .33em")
                .with_css_statement(&format!("background-color: {}", foreground.as_css_string()))
                .to_string(),
            );
        }
        Some(css)
    }
}
