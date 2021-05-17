use std::borrow::Cow;

use gooey_core::{
    stylecs::{Dimension, Points},
    Transmogrifier,
};
use gooey_widgets::container::{Container, ContainerTransmogrifier};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::{window_document, WebSys, WebSysTransmogrifier};

impl Transmogrifier<WebSys> for ContainerTransmogrifier {
    type Widget = Container;
}

impl WebSysTransmogrifier for ContainerTransmogrifier {
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::Element> {
        if let Some(child_transmogrifier) = frontend.transmogrifier(&widget.child.widget_type_id())
        {
            let container = window_document()
                .create_element("div")
                .expect("error creating div");
            set_element_style(&container, "display", "flex");
            set_element_style(&container, "align-items", "center");
            set_element_style(&container, "justify-content", "center");

            set_element_padding(&container, "padding-left", widget.padding.left);
            set_element_padding(&container, "padding-right", widget.padding.right);
            set_element_padding(&container, "padding-top", widget.padding.top);
            set_element_padding(&container, "padding-bottom", widget.padding.bottom);

            parent.append_child(&container).unwrap();

            if let Some(child) =
                child_transmogrifier.transmogrify(&container, widget.child.as_ref(), frontend)
            {
                container
                    .append_child(&child)
                    .expect("error appending child");
            }
            Some(container)
        } else {
            None
        }
    }
}
fn set_element_style(element: &web_sys::Element, name: &str, value: &str) {
    let element_style = element.unchecked_ref::<HtmlElement>().style();
    element_style.set_property(name, value).unwrap();
}

fn set_element_padding(element: &web_sys::Element, name: &str, dimension: Dimension<Points>) {
    set_element_style(
        element,
        name,
        match dimension {
            Dimension::Auto => Cow::Borrowed("auto"),
            Dimension::Minimal => Cow::Borrowed("0"),
            Dimension::Length(value) => Cow::Owned(format!("{}pts", value.get())),
        }
        .as_ref(),
    );
}
