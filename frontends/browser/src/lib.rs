use std::any::TypeId;

use gooey_core::{
    AnyChannels, AnySendSync, AnyTransmogrifier, AnyWidget, Channels, Gooey, Transmogrifier,
    TransmogrifierState, WidgetId,
};

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
            |transmogrifier, state, widget, channels| {
                transmogrifier.transmogrify(state, channels, &parent, widget, self);
            },
        );
    }
}

pub struct RegisteredTransmogrifier(pub Box<dyn AnyWidgetWebSysTransmogrifier>);

impl AnyWidgetWebSysTransmogrifier for RegisteredTransmogrifier {
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        channels: &dyn AnyChannels,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        self.0
            .transmogrify(state, channels, parent, widget, frontend)
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
        channels: &Channels<<Self as Transmogrifier<WebSys>>::Widget>,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

pub trait AnyWidgetWebSysTransmogrifier: AnyTransmogrifier<WebSys> {
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        channels: &dyn AnyChannels,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

impl<T> AnyWidgetWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + AnyTransmogrifier<WebSys> + Send + Sync + 'static,
{
    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        channels: &dyn AnyChannels,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<WebSys>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<WebSys>>::State>()
            .unwrap();
        let channels = channels
            .as_any()
            .downcast_ref::<Channels<<T as Transmogrifier<WebSys>>::Widget>>()
            .unwrap();
        <T as WebSysTransmogrifier>::transmogrify(&self, state, channels, parent, widget, frontend)
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

    fn default_state_for(&self, widget_id: WidgetId) -> TransmogrifierState {
        self.0.default_state_for(widget_id)
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
