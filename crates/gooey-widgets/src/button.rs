use gooey_core::{AnyCallback, Callback, Widget, WidgetValue};

#[derive(Debug, Default, Clone)]
pub struct Button {
    pub label: WidgetValue<String>,
    pub on_click: Option<Callback<()>>,
}

impl Button {
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

impl Widget for Button {}

#[derive(Default)]
pub struct ButtonTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use futures_util::StreamExt;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::WebApp;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlButtonElement;

    use crate::button::{Button, ButtonTransmogrifier};

    impl WidgetTransmogrifier<WebApp> for ButtonTransmogrifier {
        type Widget = Button;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            _context: &<WebApp as gooey_core::Frontend>::Context,
        ) -> <WebApp as gooey_core::Frontend>::Instance {
            let label = widget.label.clone();
            let on_click = widget.on_click.clone();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let button = document
                .create_element("button")
                .expect("failed to create button")
                .dyn_into::<HtmlButtonElement>()
                .expect("incorrect element type");

            label.map_ref(|label| button.set_inner_text(label));

            if let WidgetValue::Value(label) = label {
                let button = button.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        button.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_click) = on_click {
                let closure = Closure::new(move || {
                    on_click.invoke(());
                });
                button
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .expect("error installing button callback");
                closure.forget();
            }

            button.into()
        }
    }
}
