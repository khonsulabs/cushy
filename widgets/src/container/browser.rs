use gooey_browser::{WebSys, WebSysTransmogrifier};
use gooey_core::{euclid::Length, Points, Transmogrifier, TransmogrifierContext};
use wasm_bindgen::JsCast;

use crate::{
    browser_utils::window_document,
    container::{Container, ContainerTransmogrifier},
};

impl Transmogrifier<WebSys> for ContainerTransmogrifier {
    type State = u32;
    type Widget = Container;
}

impl WebSysTransmogrifier for ContainerTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        context.frontend.with_transmogrifier(
            context.widget.child.id(),
            |child_transmogrifier, mut child_context| {
                let container = window_document()
                    .create_element("div")
                    .expect("error creating div")
                    .unchecked_into::<web_sys::HtmlDivElement>();
                set_element_style(&container, "display", Some("flex"));
                set_element_style(&container, "align-items", Some("center"));
                set_element_style(&container, "justify-content", Some("center"));

                set_element_padding(&container, "padding-left", context.widget.padding.left());
                set_element_padding(&container, "padding-right", context.widget.padding.right());
                set_element_padding(&container, "padding-top", context.widget.padding.top());
                set_element_padding(
                    &container,
                    "padding-bottom",
                    context.widget.padding.bottom(),
                );

                if let Some(child) = child_transmogrifier.transmogrify(&mut child_context) {
                    container
                        .append_child(&child)
                        .expect("error appending child");
                }
                container.unchecked_into()
            },
        )
    }
}

fn set_element_style(element: &web_sys::HtmlElement, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        element.style().set_property(name, value).unwrap();
    } else {
        drop(element.style().remove_property(name));
    }
}

fn set_element_padding(
    element: &web_sys::HtmlElement,
    name: &str,
    dimension: Option<Length<f32, Points>>,
) {
    set_element_style(
        element,
        name,
        dimension
            .map(|length| format!("{}pts", length.get()))
            .as_deref(),
    );
}
