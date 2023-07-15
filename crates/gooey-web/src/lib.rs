use std::any::type_name;
use std::fmt::Debug;
use std::mem;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use gooey_core::style::Style;
use gooey_core::{ActiveContext, BoxedWidget, Frontend, Runtime, Widget, WidgetInstance, Widgets};
use gooey_reactor::{ScopeGuard, Value};
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
    widgets: Arc<Widgets<WebApp>>,
}

impl WebApp {
    pub fn new<Widget, Initializer>(widgets: Widgets<WebApp>, init: Initializer) -> (Self, Node)
    where
        Initializer: FnOnce(&ActiveContext) -> Widget,
        Widget: gooey_core::IntoNewWidget,
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
        let widget = init(&context).into_new(&context);
        let node = app.widgets.instantiate(
            &widget.widget,
            *widget.style,
            &WebContext {
                _scope: app.runtime.root_scope().clone(),

                app: app.clone(),
            },
        );

        (app, node)
    }
}

impl Frontend for WebApp {
    type Context = WebContext;
    type Instance = Node;
}

pub trait WebTransmogrifier: RefUnwindSafe + UnwindSafe + Send + Sync + 'static {
    type Widget: Widget;

    fn transmogrify(
        &self,
        widget: &Self::Widget,
        style: Value<Style>,
        context: &WebContext,
    ) -> Node;

    fn widget_type_name(&self) -> &'static str {
        type_name::<Self::Widget>()
    }
}

#[derive(Clone)]
pub struct WebContext {
    app: WebApp,
    _scope: Arc<ScopeGuard>,
}

impl WebContext {
    pub fn instantiate(&self, widget: &WidgetInstance<BoxedWidget>) -> Node {
        self.app
            .widgets
            .instantiate(widget.widget.as_ref(), *widget.style, self)
    }
}
