use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{
    styles::{Alignment, Style},
    Context, TransmogrifierContext,
};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::HtmlInputElement;

use crate::input::{Command, Input, InputTransmogrifier};

impl gooey_core::Transmogrifier<WebSys> for InputTransmogrifier {
    type State = Option<CssRules>;
    type Widget = Input;

    #[allow(clippy::cast_possible_truncation)]
    fn receive_command(
        &self,
        command: Command,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        if let Some(element) = input_element(&Context::from(&*context)) {
            match command {
                Command::ValueSet => {
                    element.set_value(&context.widget.value);
                }
                Command::SelectionSet => {
                    let start = context.widget.selection_start;
                    let end = context.widget.selection_end.unwrap_or(start);
                    if end >= start {
                        element
                            .set_selection_range_with_direction(start as u32, end as u32, "forward")
                            .unwrap();
                    } else {
                        element
                            .set_selection_range_with_direction(
                                end as u32,
                                start as u32,
                                "backward",
                            )
                            .unwrap();
                    }
                }
            }
        }
    }
}

impl WebSysTransmogrifier for InputTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let element = create_element::<HtmlInputElement>("input");
        *context.state = self.initialize_widget_element(&element, &context);
        element.set_value(&context.widget.value);

        let callback_context = Context::from(&context);
        element.set_oninput(
            Closure::wrap(Box::new(move || {
                callback_context.map_mut(|input, context| {
                    if let Some(element) = input_element(context) {
                        input.value = element.value();
                        input.changed.invoke(());
                    }
                });
            }) as Box<dyn FnMut()>)
            .into_js_value()
            .dyn_ref(),
        );

        let callback_context = Context::from(&context);
        element.set_onselect(
            Closure::wrap(Box::new(move || {
                callback_context.map_mut(|input, context| {
                    if let Some(element) = input_element(context) {
                        let start = element
                            .selection_start()
                            .unwrap_or_default()
                            .unwrap_or_default() as usize;
                        let end = element
                            .selection_end()
                            .unwrap_or_default()
                            .unwrap_or_default() as usize;
                        let direction = element.selection_direction().unwrap_or_default();
                        let (start, end) = match direction.as_deref() {
                            Some("backward") => (end, start),
                            _ => (start, end),
                        };
                        let end = if start == end { None } else { Some(end) };
                        input.selection_start = start;
                        input.selection_end = end;
                        input.selection_changed.invoke(());
                    }
                });
            }) as Box<dyn FnMut()>)
            .into_js_value()
            .dyn_ref(),
        );

        Some(element.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        let horizontal_alignment = match style.get_or_default::<Alignment>() {
            Alignment::Left => "left",
            Alignment::Center => "center",
            Alignment::Right => "right",
        };
        self.convert_standard_components_to_css(style, css)
            .with_css_statement(&format!("text-align: {}", horizontal_alignment))
            // hide the focus ring
            .with_css_statement("outline: none")
    }
}

fn input_element(context: &Context<Input>) -> Option<HtmlInputElement> {
    window_document()
        .get_element_by_id(&widget_css_id(
            context.registration().upgrade().unwrap().id().id,
        ))
        .map(JsCast::unchecked_into::<HtmlInputElement>)
}
