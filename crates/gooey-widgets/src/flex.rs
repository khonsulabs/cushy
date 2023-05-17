use alot::OrderedLots;
use gooey_core::{ActiveContext, BoxedWidget, Widget, WidgetInstance, WidgetValue};

#[derive(Debug)]
pub struct Flex {
    pub children: WidgetValue<FlexChildren>,
}

impl Flex {
    pub fn columns(context: &ActiveContext) -> Builder<'_> {
        Builder::new(context, FlexDirection::Column)
    }

    pub fn rows(context: &ActiveContext) -> Builder<'_> {
        Builder::new(context, FlexDirection::Row)
    }
}

impl Widget for Flex {}

#[derive(Debug)]
pub struct Builder<'a> {
    context: &'a ActiveContext,
    direction: FlexDirection,
    reverse: bool,
    children: OrderedLots<FlexChild>,
}

impl<'a> Builder<'a> {
    fn new(context: &'a ActiveContext, direction: FlexDirection) -> Self {
        Self {
            context,
            direction,
            reverse: false,
            children: OrderedLots::default(),
        }
    }

    pub fn with<W>(self, widget_fn: impl FnOnce(ActiveContext) -> W) -> Self
    where
        W: Widget,
    {
        self.with_config(widget_fn, FlexConfig::default())
    }

    pub fn with_config<W>(
        mut self,
        widget_fn: impl FnOnce(ActiveContext) -> W,
        config: FlexConfig,
    ) -> Self
    where
        W: Widget,
    {
        let widget = self.context.new_widget(widget_fn).boxed();
        self.children.push(FlexChild { widget, config });
        self
    }

    pub fn with_widget<W>(self, widget: W) -> Self
    where
        W: Widget,
    {
        self.with(|_| widget)
    }

    pub fn with_widget_and_config<W>(self, widget: W, config: FlexConfig) -> Self
    where
        W: Widget,
    {
        self.with_config(|_| widget, config)
    }

    pub fn finish(self) -> Flex {
        Flex {
            children: WidgetValue::Static(FlexChildren {
                context: self.context.clone(),
                children: self.children,
            }),
        }
    }

    pub fn finish_dynamic(self) -> Flex {
        let children = self.context.new_value(FlexChildren {
            children: self.children,
            context: self.context.clone(),
        });
        Flex {
            children: WidgetValue::Value(children),
        }
    }
}

#[derive(Debug)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug)]
pub struct FlexChildren {
    context: ActiveContext,
    children: OrderedLots<FlexChild>,
}

impl FlexChildren {
    pub const fn new(context: ActiveContext) -> Self {
        Self {
            context,
            children: OrderedLots::new(),
        }
    }

    pub fn from_children(
        context: ActiveContext,
        children: impl IntoIterator<Item = FlexChild>,
    ) -> Self {
        Self {
            context,
            children: children.into_iter().collect(),
        }
    }

    pub fn push<Template, WidgetFn>(&mut self, widget_fn: WidgetFn)
    where
        WidgetFn: FnOnce(ActiveContext) -> Template,
        Template: Widget,
    {
        let widget = self.context.new_widget(widget_fn);
        self.children.push(widget.into());
    }

    pub fn entries(&self) -> alot::ordered::EntryIter<'_, FlexChild> {
        self.children.entries()
    }
}

#[derive(Debug)]
pub struct FlexChild {
    widget: WidgetInstance<BoxedWidget>,
    config: FlexConfig,
}

impl<W> From<WidgetInstance<W>> for FlexChild
where
    W: Widget,
{
    fn from(widget: WidgetInstance<W>) -> Self {
        Self {
            widget: widget.boxed(),
            config: FlexConfig::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct FlexConfig {
    pub basis: u32,
    pub align: Option<SelfAlign>,
    pub justify: Option<SelfJustify>,
}

#[derive(Debug)]
pub enum SelfAlign {
    Stretch,
    Start,
    End,
    Center,
    Baseline,
    FirstBaseline,
    LastBaseline,
}

#[derive(Debug)]
pub enum SelfJustify {}

#[derive(Default)]
pub struct FlexTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::{WebApp, WebContext};
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::flex::FlexTransmogrifier;
    use crate::Flex;

    impl WidgetTransmogrifier<WebApp> for FlexTransmogrifier {
        type Widget = Flex;

        fn transmogrify(&self, widget: &Self::Widget, context: &WebContext) -> Node {
            log::info!("instantiating flex");
            let mut tracked_children = Vec::new();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let container = document
                .create_element("div")
                .expect("failed to create button")
                .dyn_into::<HtmlElement>()
                .expect("incorrect element type");
            widget.children.map_ref(|children| {
                for (id, child) in children.entries() {
                    let child = context.instantiate(&child.widget.widget);
                    container
                        .append_child(&child)
                        .expect("error appending child");
                    tracked_children.push((id, child));
                }
            });

            if let WidgetValue::Value(children) = widget.children {
                let container = container.clone();
                let context = context.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut children = children.into_stream();
                    while children.wait_next().await {
                        children.map_ref(|children| {
                            'children: for (index, (id, child)) in children.entries().enumerate() {
                                for tracked_index in index..tracked_children.len() {
                                    if tracked_children[tracked_index].0 == id {
                                        // This node already exists, move it in
                                        // the array if needed.
                                        if index != tracked_index {
                                            tracked_children.swap(tracked_index, index);
                                        }
                                        continue 'children;
                                    }
                                }

                                // The child wasn't found.
                                let child = context.instantiate(&child.widget.widget);
                                if let Some(next_node) = tracked_children.get(index + 1) {
                                    container.insert_before(&child, Some(&next_node.1)).unwrap();
                                } else {
                                    container.append_child(&child).unwrap();
                                }
                                tracked_children.insert(index, (id, child));
                            }

                            for (_, removed) in tracked_children.drain(children.children.len()..) {
                                container.remove_child(&removed).unwrap();
                            }
                        });
                    }
                });
            }
            container.into()
        }
    }
}

/*
Flex::
 */
