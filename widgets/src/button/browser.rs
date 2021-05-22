use gooey_browser::{WebSys, WebSysTransmogrifier, WidgetClosure};
use gooey_core::{Channels, Widget};
use wasm_bindgen::JsCast;
use web_sys::HtmlButtonElement;

use crate::{
    button::{Button, ButtonEvent, ButtonTransmogrifier},
    window_document,
};

impl gooey_core::Transmogrifier<WebSys> for ButtonTransmogrifier {
    type State = u32;
    type Widget = Button;

    fn receive_command(
        &self,
        state: &mut Self::State,
        command: <Self::Widget as Widget>::TransmogrifierCommand,
        widget: &Self::Widget,
        channels: &Channels<Button>,
        frontend: &WebSys,
    ) {
        todo!("Got command:{:?}", command)
    }
}

impl WebSysTransmogrifier for ButtonTransmogrifier {
    fn transmogrify(
        &self,
        _state: &Self::State,
        channels: &Channels<Button>,
        widget: &Button,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        channels.widget().map(|registration| {
            let document = window_document();
            let element = document
                .create_element("button")
                .expect("couldn't create button")
                .unchecked_into::<HtmlButtonElement>();
            element.set_id(&registration.id().id.to_string());
            element.set_inner_text(&widget.label);

            let closure = WidgetClosure::new(channels.widget_ref(gooey.clone()).unwrap(), || {
                ButtonEvent::Clicked
            });
            element.set_onclick(Some(closure.into_js_value().unchecked_ref()));
            element.unchecked_into()
        })
    }
}
