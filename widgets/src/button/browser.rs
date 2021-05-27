use gooey_browser::{WebSys, WebSysTransmogrifier, WidgetClosure};
use gooey_core::{Widget, WidgetRef};
use wasm_bindgen::JsCast;
use web_sys::HtmlButtonElement;

use crate::{
    button::{Button, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent},
    window_document,
};

impl gooey_core::Transmogrifier<WebSys> for ButtonTransmogrifier {
    type State = Option<WidgetRef<Button>>;
    type Widget = Button;

    #[allow(unused_variables)]
    fn initialize(
        &self,
        _widget: &Self::Widget,
        reference: &WidgetRef<Self::Widget>,
        _frontend: &WebSys,
    ) -> Self::State {
        Some(reference.clone())
    }

    fn receive_command(
        &self,
        state: &mut Self::State,
        command: <Self::Widget as Widget>::TransmogrifierCommand,
        _widget: &Self::Widget,
        _frontend: &WebSys,
    ) {
        let document = window_document();
        if let Some(element) = document
            .get_element_by_id(
                &state
                    .as_ref()
                    .unwrap()
                    .registration()
                    .unwrap()
                    .id()
                    .id
                    .to_string(),
            )
            .and_then(|e| e.dyn_into::<HtmlButtonElement>().ok())
        {
            let ButtonCommand::SetLabel(new_label) = command;
            element.set_inner_text(&new_label);
        }
    }
}

impl WebSysTransmogrifier for ButtonTransmogrifier {
    fn transmogrify(
        &self,
        widget_ref: &Self::State,
        widget: &Button,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        widget_ref
            .as_ref()
            .and_then(|widget| widget.registration())
            .map(|registration| {
                let document = window_document();
                let element = document
                    .create_element("button")
                    .expect("couldn't create button")
                    .unchecked_into::<HtmlButtonElement>();
                element.set_id(&registration.id().id.to_string());
                element.set_inner_text(&widget.label);

                let closure = WidgetClosure::new::<WebSys, Button, _>(
                    WidgetRef::new(&registration, gooey.clone()).unwrap(),
                    || InternalButtonEvent::Clicked,
                );
                element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
                element.unchecked_into()
            })
    }
}
