use gooey_core::style::FontSize;
use gooey_core::{ActiveContext, AnyCallback, Callback, NewWidget, Widget, WidgetValue};

#[derive(Debug, Default, Clone, Widget)]
#[widget(authority = gooey)]
pub struct Label {
    pub label: WidgetValue<String>,
    pub on_click: Option<Callback<()>>,
}

impl Label {
    pub fn new(label: impl Into<WidgetValue<String>>, context: &ActiveContext) -> NewWidget<Self> {
        NewWidget::new(
            Self {
                label: label.into(),
                ..Self::default()
            },
            context,
        )
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

pub trait LabelExt {
    fn font_size(self, size: impl Into<WidgetValue<FontSize>>) -> NewWidget<Label>;
}

impl LabelExt for NewWidget<Label> {
    fn font_size(self, size: impl Into<WidgetValue<FontSize>>) -> NewWidget<Label> {
        self.style.push(size.into());
        self
    }
}

#[derive(Default)]
pub struct LabelTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use std::fmt::Write;

    use futures_util::StreamExt;
    use gooey_core::reactor::Value;
    use gooey_core::style::{Dimension, FontSize};
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::WebApp;
    use stylecs::Style;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::label::{Label, LabelTransmogrifier};

    impl WidgetTransmogrifier<WebApp> for LabelTransmogrifier {
        type Widget = Label;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: Value<Style>,
            _context: &gooey_web::WebContext,
        ) -> Node {
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

            // Apply initial styling.
            // style.map_ref(|style| {
            //     let mut css = String::new();
            //     if let Some(font_size) = style.get::<FontSize>() {
            //         let FontSize(Dimension::Pixels(pixels)) = font_size else { todo!("implement better dimension conversion") };
            //         write!(&mut css, "font-size:{pixels}px;").expect("error writing css");
            //     }

            //     // if !css.is_empty() {
            //         element
            //             .set_attribute("style", &css)
            //             .expect("error setting style");
            //     // }
            // });
            wasm_bindgen_futures::spawn_local({
                let element = element.clone();
                async move {
                    let mut style = style.into_stream();
                    while style.wait_next().await {
                        style.map_ref(|style| {
                            let mut css = String::new();
                            if let Some(font_size) = style.get::<FontSize>() {
                                let FontSize(Dimension::Pixels(pixels)) = font_size else { todo!("implement better dimension conversion") };
                                write!(&mut css, "font-size:{pixels}px;").expect("error writing css");
                            }

                            // if !css.is_empty() {
                                element
                                    .set_attribute("style", &css)
                                    .expect("error setting style");
                            // }
                        });
                    }
                }
            });

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
