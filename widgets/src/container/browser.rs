use gooey_browser::{AnyWidgetWebSysTransmogrifier, WebSys, WebSysTransmogrifier};
use gooey_core::{euclid::Length, styles::Points, Transmogrifier};
use wasm_bindgen::JsCast;

use crate::{
    container::{Container, ContainerTransmogrifier},
    window_document,
};

impl Transmogrifier<WebSys> for ContainerTransmogrifier {
    type Widget = Container;
}

impl WebSysTransmogrifier for ContainerTransmogrifier {
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        if let Some(child_transmogrifier) =
            frontend.ui.transmogrifier(widget.child.widget_type_id())
        {
            let container = window_document()
                .create_element("div")
                .expect("error creating div")
                .unchecked_into::<web_sys::HtmlDivElement>();
            set_element_style(&container, "display", Some("flex"));
            set_element_style(&container, "align-items", Some("center"));
            set_element_style(&container, "justify-content", Some("center"));

            set_element_padding(&container, "padding-left", widget.padding.left());
            set_element_padding(&container, "padding-right", widget.padding.right());
            set_element_padding(&container, "padding-top", widget.padding.top());
            set_element_padding(&container, "padding-bottom", widget.padding.bottom());

            parent.append_child(&container).unwrap();

            if let Some(child) =
                child_transmogrifier.transmogrify(&container, widget.child.as_ref(), frontend)
            {
                container
                    .append_child(&child)
                    .expect("error appending child");
            }
            Some(container.unchecked_into())
        } else {
            None
        }
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