use gooey_core::{AnyCallback, Callback, Widget, WidgetValue};

#[derive(Debug, Default, Clone)]
pub struct Label {
    pub label: WidgetValue<String>,
    pub on_click: Option<Callback<()>>,
}

impl Label {
    pub fn new(label: impl Into<WidgetValue<String>>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    pub fn label(mut self, label: impl Into<WidgetValue<String>>) -> Self {
        self.label = label.into();
        self
    }

    pub fn on_click<CB: AnyCallback<()>>(mut self, cb: CB) -> Self {
        self.on_click = Some(Callback::new(cb));
        self
    }
}

impl Widget for Label {}

#[derive(Default)]
pub struct LabelTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use futures_util::StreamExt;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::WebApp;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::label::{Label, LabelTransmogrifier};

    impl WidgetTransmogrifier<WebApp> for LabelTransmogrifier {
        type Widget = Label;

        fn transmogrify(&self, widget: &Self::Widget, _context: &gooey_web::WebContext) -> Node {
            let label = widget.label.clone();
            let on_click = widget.on_click.clone();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let element = document
                .create_element("div")
                .expect("failed to create button")
                .dyn_into::<HtmlElement>()
                .expect("incorrect element type");

            label.map_ref(|label| element.set_inner_text(label));

            if let WidgetValue::Value(label) = label {
                let element = element.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        element.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_click) = on_click {
                let closure = Closure::new(move || {
                    on_click.invoke(());
                });
                element
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .expect("error installing button callback");
                closure.forget();
            }

            element.into()
        }
    }
}
