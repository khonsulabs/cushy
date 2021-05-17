use std::any::TypeId;

use gooey_core::{AnyWidget, Gooey, Transmogrifier};
use gooey_widgets::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

mod widgets;

pub struct WebSys {
    ui: Gooey<Self>,
}

impl WebSys {
    pub fn new(ui: Gooey<Self>) -> Self {
        let mut frontend = Self { ui };

        frontend.register_transmogrifier(ButtonTransmogrifier);
        frontend.register_transmogrifier(ContainerTransmogrifier);

        frontend
    }

    pub fn register_transmogrifier<M: WebSysTransmogrifier + Send + Sync + 'static>(
        &mut self,
        transmogrifier: M,
    ) {
        self.ui
            .transmogrifiers
            .insert(TypeId::of::<M::Widget>(), Box::new(transmogrifier));
    }

    pub fn install_in_id(&self, id: &str) {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let parent = document.get_element_by_id(id).expect("id not found");

        if let Some(transmogrifier) = self
            .ui
            .transmogrifiers
            .get(&self.ui.root_widget().widget_type_id())
            .map(|b| b.as_ref())
        {
            transmogrifier.transmogrify(&parent, self.ui.root_widget(), self);
        }
    }

    pub fn transmogrifier(
        &self,
        widget_type_id: &TypeId,
    ) -> Option<&'_ dyn AnyWidgetWebSysTransmogrifier> {
        self.ui
            .transmogrifiers
            .get(widget_type_id)
            .map(|b| b.as_ref())
    }
}

impl gooey_core::Frontend for WebSys {
    type AnyWidgetTransmogrifier = Box<dyn AnyWidgetWebSysTransmogrifier>;
    type Context = WebSys;
}

pub trait WebSysTransmogrifier: Transmogrifier<WebSys> {
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::Element>;
}

pub trait AnyWidgetWebSysTransmogrifier {
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::Element>;
}

impl<T> AnyWidgetWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + Send + Sync + 'static,
{
    fn transmogrify(
        &self,
        parent: &web_sys::Node,
        widget: &dyn AnyWidget,
        frontend: &WebSys,
    ) -> Option<web_sys::Element> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<WebSys>>::Widget>()
            .unwrap();
        <T as WebSysTransmogrifier>::transmogrify(&self, parent, widget, frontend)
    }
}

fn window_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}
