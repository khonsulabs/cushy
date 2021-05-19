use std::any::TypeId;

use gooey_core::{
    AnySendSync, AnyTransmogrifier, AnyWidgetInstance, Gooey, Transmogrifier, TransmogrifierState,
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

        self.ui
            .with_transmogrifier(self.ui.root_widget(), |transmogrifier, state| {
                transmogrifier.transmogrify(state, &parent, self.ui.root_widget(), self);
            });
    }
}

pub struct RegisteredTransmogrifier(pub Box<dyn AnyWidgetWebSysTransmogrifier>);

impl AnyWidgetWebSysTransmogrifier for RegisteredTransmogrifier {
    fn widget_type_id(&self) -> TypeId {
        AnyWidgetWebSysTransmogrifier::widget_type_id(self.0.as_ref())
    }

    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        parent: &web_sys::Node,
        widget: &dyn AnyWidgetInstance,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement> {
        self.0.transmogrify(state, parent, widget, frontend)
    }

    fn default_state(&self) -> TransmogrifierState {
        self.0.default_state()
    }
}

impl AnyTransmogrifier for RegisteredTransmogrifier {
    fn widget_type_id(&self) -> TypeId {
        self.0.widget_type_id()
    }

    fn default_state(&self) -> TransmogrifierState {
        self.0.default_state()
    }
}

impl gooey_core::Frontend for WebSys {
    type AnyWidgetTransmogrifier = RegisteredTransmogrifier;
    type Context = WebSys;
}
pub trait WebSysTransmogrifier: Transmogrifier<WebSys> {
    fn transmogrify(
        &self,
        state: &Self::State,
        parent: &web_sys::Node,
        widget: &<Self as Transmogrifier<WebSys>>::Widget,
        frontend: &WebSys,
    ) -> Option<web_sys::HtmlElement>;
}

pub trait AnyWidgetWebSysTransmogrifier {
    fn widget_type_id(&self) -> TypeId;

    fn default_state(&self) -> TransmogrifierState;

    fn transmogrify(
        &self,
        state: &mut dyn AnySendSync,
        parent: &web_sys::Node,
        widget: &dyn AnyWidgetInstance,
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
        state: &mut dyn AnySendSync,
        parent: &web_sys::Node,
        widget: &dyn AnyWidgetInstance,
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
        <T as WebSysTransmogrifier>::transmogrify(&self, state, parent, widget, frontend)
    }

    fn default_state(&self) -> TransmogrifierState {
        TransmogrifierState(Box::new(
            <<T as Transmogrifier<WebSys>>::State as Default>::default(),
        ))
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
