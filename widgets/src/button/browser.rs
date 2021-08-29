use gooey_browser::{
    utils::{create_element, widget_css_id, window_document, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier, WidgetClosure,
};
use gooey_core::{styles::Style, Callback, Context, Frontend, TransmogrifierContext, WidgetRef};
use wasm_bindgen::JsCast;
use web_sys::{HtmlButtonElement, HtmlDivElement, HtmlImageElement};

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
            match command {
                ButtonCommand::LabelChanged | ButtonCommand::ImageChanged => {
                    recreate_button_content(&element, context);
                }
            }
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

        recreate_button_content(&element, &context);

        let closure = WidgetClosure::new::<WebSys, Button, _>(
            WidgetRef::new(&context.registration, context.frontend.clone()).unwrap(),
            || InternalButtonEvent::Clicked,
        );
        element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
        Some(element.unchecked_into())
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_standard_components_to_css(style, css)
            .with_css_statement("flex-direction: column")
    }
}

fn recreate_button_content(
    button: &HtmlButtonElement,
    context: &TransmogrifierContext<'_, ButtonTransmogrifier, WebSys>,
) {
    match context.widget.image.as_ref().and_then(|image| {
        context
            .frontend
            .asset_url(&image.asset)
            .map(|url| (image, url))
    }) {
        Some((image, url)) => {
            let children = button.children();
            let (label, img) = if children.length() > 0 {
                // Modify the nodes
                let img = children
                    .item(0)
                    .unwrap()
                    .unchecked_into::<HtmlImageElement>();
                let label = children.item(1).unwrap().unchecked_into::<HtmlDivElement>();
                (label, img)
            } else {
                // Create the nodes
                button.set_inner_text("");

                let callback_context = Context::from(context);
                context.frontend.load_image(
                    image,
                    Callback::new(move |_| {
                        callback_context.send_command(ButtonCommand::ImageChanged);
                    }),
                    Callback::default(),
                );

                let img = create_element::<HtmlImageElement>("img");
                button.append_child(&img).unwrap();

                let label = create_element::<HtmlDivElement>("div");
                button.append_child(&label).unwrap();

                (label, img)
            };

            label.set_inner_text(&context.widget.label);
            img.set_src(url.as_str());
        }
        None => {
            button.set_inner_text(&context.widget.label);
        }
    }
}
