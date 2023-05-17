use std::fmt::Debug;
use std::mem;
use std::sync::Arc;

use gooey_core::{ActiveContext, AnyWidget, Frontend, Runtime, Widgets};
use gooey_reactor::ScopeGuard;
use web_sys::{window, Node};

pub fn attach_to_body<Widget, Initializer>(widgets: Widgets<WebApp>, init: Initializer)
where
    Initializer: FnOnce(&ActiveContext) -> Widget,
    Widget: gooey_core::Widget,
{
    console_error_panic_hook::set_once();
    let (app, node) = WebApp::new(widgets, init);

    let document = window()
        .expect("no window")
        .document()
        .expect("no document");
    if let Some(body) = document.body() {
        body.append_child(&node)
            .expect("error adding node to document");
    } else {
        document
            .append_child(&node)
            .expect("error adding node to document");
    };

    mem::forget(app);
}

#[derive(Debug, Clone)]
pub struct WebApp {
    runtime: Runtime,
    widgets: Arc<Widgets<Self>>,
}

impl WebApp {
    pub fn new<Widget, Initializer>(widgets: Widgets<Self>, init: Initializer) -> (Self, Node)
    where
        Initializer: FnOnce(&ActiveContext) -> Widget,
        Widget: gooey_core::Widget,
    {
        let runtime = Runtime::default();

        let app = Self {
            runtime,
            widgets: Arc::new(widgets),
        };
        let context = ActiveContext {
            scope: ***app.runtime.root_scope(),
            frontend: Arc::new(app.clone()),
        };
        let widget = init(&context);
        let node = app.widgets.instantiate(
            &widget,
            &WebContext {
                scope: app.runtime.root_scope().clone(),
                app: app.clone(),
            },
        );

        // let node = widget.instantiate(&WebContext {
        //     scope: app.runtime.root_scope().clone(),
        //     app: app.clone(),
        // });
        (app, node)
    }
}

impl Frontend for WebApp {
    // type AnyTransmogrifier = WebTransmogrifier;
    type Context = WebContext;
    type Instance = Node;
}

#[derive(Clone)]
pub struct WebContext {
    app: WebApp,
    scope: Arc<ScopeGuard>,
}

impl WebContext {
    pub fn instantiate(&self, widget: impl AsRef<dyn AnyWidget>) -> Node {
        self.app.widgets.instantiate(widget.as_ref(), self)
    }
}

// pub struct WebTransmogrifier(Box<dyn AnyTransmogrifier>);

// impl<T> From<T> for WebTransmogrifier
// where
//     T: AnyTransmogrifier,
// {
//     fn from(value: T) -> Self {
//         Self(Box::new(value))
//     }
// }

// impl Transmogrify<WebApp> for WebTransmogrifier {
//     fn transmogrify(
//         &self,
//         widget: &dyn AnyWidget,
//         context: &<WebApp as Frontend>::Context,
//     ) -> <WebApp as Frontend>::Instance {
//         self.0.transmogrify(widget, context)
//     }

//     fn widget_type_name(&self) -> &'static str {
//         self.0.widget_type_name()
//     }
// }

// trait AnyTransmogrifier: Send + Sync + 'static {
//     fn transmogrify(&self, widget: &dyn AnyWidget, context: &WebContext) -> Node;
//     fn widget_type_name(&self) -> &'static str;
// }

// struct Transmogrifier<W>(PhantomData<&'static W>)
// where
//     W: gooey_core::Widget;

// impl<W> AnyTransmogrifier for Transmogrifier<W>
// where
//     W: WebWidget,
// {
//     fn transmogrify(&self, widget: &dyn AnyWidget, context: &WebContext) -> Node {
//         widget
//             .as_any()
//             .downcast_ref::<W>()
//             .expect("type mismatch")
//             .instantiate(context)
//     }
//     fn widget_type_name(&self) -> &'static str {
//         type_name::<W>()
//     }
// }

// impl<T> WidgetTransmogrifier<WebApp> for Transmogrifier<T>
// where
//     T: WebWidget,
// {
//     type Widget = T;

//     fn make_transmogrifier() -> WebTransmogrifier {
//         WebTransmogrifier::from(Self(PhantomData))
//     }
// }
