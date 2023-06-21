use gooey_core::style::StyleComponent;
use gooey_core::{Children, Widget, WidgetValue};

#[derive(Debug, Widget)]
#[widget(authority = gooey)]
pub struct Flex {
    pub direction: WidgetValue<FlexDirection>,
    pub children: WidgetValue<Children>,
}

impl Flex {
    pub fn new(
        direction: impl Into<WidgetValue<FlexDirection>>,
        children: impl Into<WidgetValue<Children>>,
    ) -> Self {
        Self {
            direction: direction.into(),
            children: children.into(),
        }
    }

    pub fn columns(children: impl Into<WidgetValue<Children>>) -> Self {
        Self::new(FlexDirection::columns(), children)
    }

    pub fn rows(children: impl Into<WidgetValue<Children>>) -> Self {
        Self::new(FlexDirection::rows(), children)
    }
}

#[derive(Debug, StyleComponent)]
#[style(authority = gooey)]
pub enum FlexDirection {
    Row { reverse: bool },
    Column { reverse: bool },
}

impl FlexDirection {
    pub const fn columns() -> Self {
        Self::Column { reverse: false }
    }

    pub const fn columns_rev() -> Self {
        Self::Column { reverse: true }
    }

    pub const fn rows() -> Self {
        Self::Row { reverse: false }
    }

    pub const fn rows_rev() -> Self {
        Self::Row { reverse: true }
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
    use gooey_core::reactor::Value;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::{WebApp, WebContext};
    use stylecs::Style;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::flex::FlexTransmogrifier;
    use crate::Flex;

    impl WidgetTransmogrifier<WebApp> for FlexTransmogrifier {
        type Widget = Flex;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: Value<Style>,
            context: &WebContext,
        ) -> Node {
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
                    let child = context.instantiate(child);
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
                                let child = context.instantiate(child);
                                if let Some(next_node) = tracked_children.get(index + 1) {
                                    container.insert_before(&child, Some(&next_node.1)).unwrap();
                                } else {
                                    container.append_child(&child).unwrap();
                                }
                                tracked_children.insert(index, (id, child));
                            }

                            for (_, removed) in tracked_children.drain(children.len()..) {
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
