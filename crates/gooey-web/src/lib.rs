use std::any::type_name;
use std::fmt::Debug;
use std::mem;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use gooey_core::style::Style;
use gooey_core::window::{NewWindow, Window};
use gooey_core::{BoxedWidget, Context, Frontend, Runtime, Widget, WidgetInstance, Widgets};
use gooey_reactor::{Dynamic, ScopeGuard};
use web_sys::{window, Node};

pub fn attach_to_body<Widget>(widgets: Arc<Widgets<WebApp>>, init: NewWindow<Widget>)
where
    Widget: gooey_core::Widget,
{
    console_error_panic_hook::set_once();
    let (app, widget) = WebApp::new(widgets, init);

    let document = window()
        .expect("no window")
        .document()
        .expect("no document");
    if let Some(body) = document.body() {
        body.append_child(&widget.node)
            .expect("error adding node to document");
    } else {
        document
            .append_child(&widget.node)
            .expect("error adding node to document");
    };

    mem::forget(app);
    mem::forget(widget);
}

#[derive(Debug, Clone)]
pub struct WebApp {
    runtime: Runtime,
    widgets: Arc<Widgets<WebApp>>,
}

impl WebApp {
    pub fn new<IntoWidget>(
        widgets: Arc<Widgets<WebApp>>,
        init: NewWindow<IntoWidget>,
    ) -> (Self, WebWidget<IntoWidget::Widget>)
    where
        IntoWidget: gooey_core::IntoNewWidget,
    {
        let runtime = Runtime::default();

        let app = Self { runtime, widgets };
        let window_scope = Arc::new(app.runtime.root_scope().new_scope());
        let context = Context {
            scope: **window_scope,
            frontend: Arc::new(app.clone()),
        };
        let window = Window::new(init.attributes, &context);
        let widget = (init.init)(&context, &window).into_new(&context);

        // TODO support title/size

        let node = app.widgets.instantiate(
            &widget.widget,
            widget.style,
            &WebContext {
                _scope: window_scope.clone(),

                app: app.clone(),
            },
        );

        (
            app,
            WebWidget {
                widget: widget.widget,
                scope: window_scope,
                node,
            },
        )
    }
}

pub struct WebWidget<T> {
    pub widget: T,
    pub scope: Arc<ScopeGuard>,
    pub node: Node,
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
        style: Dynamic<Style>,
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
            .instantiate(&*widget.widget, widget.style, self)
    }
}
