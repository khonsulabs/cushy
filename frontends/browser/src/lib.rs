use std::any::TypeId;

use gooey_core::{
    AnyChannels, AnySendSync, AnyTransmogrifier, AnyWidget, Frontend, Gooey, Transmogrifier,
    TransmogrifierState, Widget, WidgetRef, WidgetRegistration,
};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
pub struct WebSys {
    pub ui: Gooey<Self>,
}

impl WebSys {
    pub fn new(ui: Gooey<Self>) -> Self {
        Self { ui }
    }

    pub fn install_in_id(&self, id: &str) {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let parent = document.get_element_by_id(id).expect("id not found");

        self.ui.with_transmogrifier(
            self.ui.root_widget().id(),
            self,
            |transmogrifier, state, widget| {
                if let Some(root_element) = transmogrifier.transmogrify(state, widget, self) {
                    root_element.style().set_property("width", "100%").unwrap();
                    root_element.style().set_property("height", "100%").unwrap();
                    parent.append_child(&root_element).unwrap();
                }
            },
        );
    }
}

#[derive(Debug)]
pub struct RegisteredTransmogrifier(pub Box<dyn AnyWidgetWebSysTransmogrifier>);

impl AnyWidgetWebSysTransmogrifier for RegisteredTransmogrifier {
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        self.0.transmogrify(state, widget, gooey)
    }
}

impl gooey_core::Frontend for WebSys {
    type AnyTransmogrifier = RegisteredTransmogrifier;
    type Context = WebSys;

    fn gooey(&self) -> &'_ Gooey<Self> {
        &self.ui
    }
}

pub trait WebSysTransmogrifier: Transmogrifier<WebSys> {
    fn transmogrify(
        &self,
        state: &Self::State,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

pub trait AnyWidgetWebSysTransmogrifier: AnyTransmogrifier<WebSys> + Send + Sync {
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

impl<T> AnyWidgetWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + AnyTransmogrifier<WebSys> + Send + Sync + 'static,
{
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        gooey: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<WebSys>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<WebSys>>::State>()
            .unwrap();
        <T as WebSysTransmogrifier>::transmogrify(&self, state, widget, gooey)
    }
}

impl AnyTransmogrifier<WebSys> for RegisteredTransmogrifier {
    fn process_messages(
        &self,
        state: &mut dyn AnySendSync,
        widget: &mut dyn AnyWidget,
        channels: &dyn AnyChannels,
        frontend: &WebSys,
    ) {
        self.0.process_messages(state, widget, channels, frontend)
    }

    fn widget_type_id(&self) -> TypeId {
        self.0.widget_type_id()
    }

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &WidgetRegistration,
        frontend: &WebSys,
    ) -> TransmogrifierState {
        self.0.default_state_for(widget, registration, frontend)
    }
}

#[macro_export]
macro_rules! make_browser {
    ($transmogrifier:ident) => {
        impl From<$transmogrifier> for $crate::RegisteredTransmogrifier {
            fn from(transmogrifier: $transmogrifier) -> Self {
                Self(std::boxed::Box::new(transmogrifier))
            }
        }
    };
}

pub struct WidgetClosure;

impl WidgetClosure {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<
        F: Frontend,
        W: Widget,
        C: FnMut() -> <W as Widget>::TransmogrifierEvent + 'static,
    >(
        widget: WidgetRef<W>,
        mut event_generator: C,
    ) -> Closure<dyn FnMut()> {
        Closure::wrap(Box::new(move || {
            let event = event_generator();
            widget.post_event::<F>(event);
        }) as Box<dyn FnMut()>)
    }
}
