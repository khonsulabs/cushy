use std::any::TypeId;

use gooey_core::{AnyTransmogrifier, AnyWidget, Gooey, Transmogrifier};

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

        if let Some(transmogrifier) = self.ui.root_transmogrifier() {
            transmogrifier.transmogrify(&parent, self.ui.root_widget(), self);
        }
    }
}

pub struct RegisteredTransmogrifier(pub Box<dyn AnyWidgetWebSysTransmogrifier>);

impl AnyWidgetWebSysTransmogrifier for RegisteredTransmogrifier {
    fn widget_type_id(&self) -> TypeId {
        AnyWidgetWebSysTransmogrifier::widget_type_id(self.0.as_ref())
    }

    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        self.0.transmogrify(parent, widget, frontend)
    }
}

impl AnyTransmogrifier for RegisteredTransmogrifier {
    fn widget_type_id(&self) -> TypeId {
        self.0.as_ref().widget_type_id()
    }
}

impl gooey_core::Frontend for WebSys {
    type AnyWidgetTransmogrifier = RegisteredTransmogrifier;
    type Context = WebSys;
}
pub trait WebSysTransmogrifier: Transmogrifier<WebSys> {
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

pub trait AnyWidgetWebSysTransmogrifier {
    fn widget_type_id(&self) -> TypeId;

    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

impl<T> AnyWidgetWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + Send + Sync + 'static,
{
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<T as Transmogrifier<WebSys>>::Widget>()
    }

    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<WebSys>>::Widget>()
            .unwrap();
        <T as WebSysTransmogrifier>::transmogrify(&self, parent, widget, frontend)
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
