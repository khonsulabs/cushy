use std::{any::TypeId, convert::TryFrom, ops::Deref, sync::Arc};

use gooey_core::{
    AnyTransmogrifier, AnyTransmogrifierContext, AnyWidget, Frontend, Gooey, Transmogrifier,
    TransmogrifierContext, TransmogrifierState, Widget, WidgetId, WidgetRef, WidgetRegistration,
};
use wasm_bindgen::prelude::*;

pub mod utils;

use utils::{CssBlockBuilder, CssManager, CssRule};

#[derive(Debug, Clone)]
pub struct WebSys {
    pub ui: Gooey<Self>,
    body_style: Arc<CssRule>,
}

impl WebSys {
    pub fn new(ui: Gooey<Self>) -> Self {
        wasm_logger::init(wasm_logger::Config::default());
        log::info!(
            "Statement: {}",
            CssBlockBuilder::for_id(ui.root_widget().id().id)
                .with_css_statement("width: 100%")
                .with_css_statement("height: 100%")
                .to_string()
        );
        let body_style = Arc::new(
            CssManager::shared().register_rule(
                &CssBlockBuilder::for_id(ui.root_widget().id().id)
                    .with_css_statement("width: 100%")
                    .with_css_statement("height: 100%")
                    .with_css_statement("display: flex")
                    .to_string(),
            ),
        );
        Self { ui, body_style }
    }

    pub fn install_in_id(&mut self, id: &str) {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let parent = document.get_element_by_id(id).expect("id not found");

        self.with_transmogrifier(self.ui.root_widget().id(), |transmogrifier, mut context| {
            if let Some(root_element) = transmogrifier.transmogrify(&mut context) {
                parent.append_child(&root_element).unwrap();
            }
        });
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn with_transmogrifier<
        TResult,
        C: FnOnce(
            &'_ dyn AnyWidgetWebSysTransmogrifier,
            AnyTransmogrifierContext<'_, Self>,
        ) -> TResult,
    >(
        &self,
        widget_id: &WidgetId,
        callback: C,
    ) -> Option<TResult> {
        self.ui
            .with_transmogrifier(widget_id, self, |transmogrifier, context| {
                callback(transmogrifier.as_ref(), context)
            })
    }
}

#[derive(Debug)]
pub struct RegisteredTransmogrifier(pub Box<dyn AnyWidgetWebSysTransmogrifier>);

impl AnyWidgetWebSysTransmogrifier for RegisteredTransmogrifier {
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        self.0.transmogrify(context)
    }
}

impl Deref for RegisteredTransmogrifier {
    type Target = Box<dyn AnyWidgetWebSysTransmogrifier>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
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
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement>;
}

pub trait AnyWidgetWebSysTransmogrifier: AnyTransmogrifier<WebSys> + Send + Sync {
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement>;
}

impl<T> AnyWidgetWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + AnyTransmogrifier<WebSys> + Send + Sync + 'static,
{
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        <T as WebSysTransmogrifier>::transmogrify(
            &self,
            TransmogrifierContext::try_from(context).unwrap(),
        )
    }
}

impl AnyTransmogrifier<WebSys> for RegisteredTransmogrifier {
    fn process_messages(&self, context: AnyTransmogrifierContext<'_, WebSys>) {
        self.0.process_messages(context)
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
