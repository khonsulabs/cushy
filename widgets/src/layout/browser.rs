use std::collections::HashMap;

use gooey_browser::{
    utils::{window_document, window_element_by_widget_id, CssBlockBuilder, CssManager, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{Transmogrifier, TransmogrifierContext};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, HtmlElement};

use crate::layout::{
    Dimension, Layout, LayoutChild, LayoutChildren, LayoutCommand, LayoutTransmogrifier,
    WidgetLayout,
};

#[derive(Default, Debug)]
pub struct WebSysLayoutState {
    children: HashMap<u32, BrowserChild>,
    css: Vec<CssRules>,
}

#[derive(Debug)]
struct BrowserChild {
    element_id: String,
    rules: Vec<CssRules>,
}

impl Transmogrifier<WebSys> for LayoutTransmogrifier {
    type State = WebSysLayoutState;
    type Widget = Layout;

    fn receive_command(
        &self,
        command: LayoutCommand,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        match command {
            LayoutCommand::ChildRemoved(child) => {
                context.state.children.remove(&child.id);
                if let Some(element) = window_element_by_widget_id::<HtmlElement>(child.id) {
                    element.remove();
                }
            }
            LayoutCommand::ChildAdded(child) => {
                if let Some(container) =
                    window_element_by_widget_id::<HtmlDivElement>(context.registration.id().id)
                {
                    if let Some(child) = context.widget.child_by_widget_id(child.id()).cloned() {
                        materialize_child(&child, context, &container);
                    }
                }
            }
        }
    }
}

impl WebSysTransmogrifier for LayoutTransmogrifier {
    fn transmogrify(
        &self,
        mut context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let css_manager = CssManager::shared();

        let container = window_document()
            .create_element("div")
            .expect("error creating div")
            .unchecked_into::<web_sys::HtmlDivElement>();
        if let Some(rule) = self.initialize_widget_element(&container, &context) {
            context.state.css.push(rule);
        }

        let container_css = CssBlockBuilder::for_id(context.registration.id().id)
            .with_css_statement("width: 100%")
            .with_css_statement("height: 100%")
            .with_css_statement("position: relative");
        context
            .state
            .css
            .push(css_manager.register_rule(&container_css.to_string()));

        for layout_child in context.widget.layout_children() {
            materialize_child(&layout_child, &mut context, &container);
        }
        Some(container.unchecked_into())
    }
}

fn materialize_child(
    layout_child: &LayoutChild,
    context: &mut TransmogrifierContext<'_, LayoutTransmogrifier, WebSys>,
    container: &HtmlElement,
) {
    context.frontend.with_transmogrifier(
        layout_child.registration.id(),
        |child_transmogrifier, mut child_context| {
            if let Some(child) = child_transmogrifier.transmogrify(&mut child_context) {
                context.state.children.insert(
                    layout_child.registration.id().id,
                    BrowserChild {
                        rules: vec![CssManager::shared().register_rule(
                            &apply_layout_rules(
                                &layout_child.layout,
                                CssBlockBuilder::for_id(layout_child.registration.id().id)
                                    .with_css_statement("position: absolute"),
                            )
                            .to_string(),
                        )],
                        element_id: child.id(),
                    },
                );
                container
                    .append_child(&child)
                    .expect("error appending child");
            }
        },
    );
}

fn apply_layout_rules(layout: &WidgetLayout, mut css: CssBlockBuilder) -> CssBlockBuilder {
    css = apply_dimension("left", layout.left, css);
    css = apply_dimension("right", layout.right, css);
    css = apply_dimension("top", layout.top, css);
    css = apply_dimension("bottom", layout.bottom, css);
    css = apply_dimension("width", layout.width, css);
    css = apply_dimension("height", layout.height, css);
    css
}

fn dimension_css(dimension: Dimension) -> Option<String> {
    match dimension {
        Dimension::Auto => None,
        Dimension::Exact(length) => Some(format!("{:0.2}pt", length.get())),
        Dimension::Percent(percent) => Some(format!("{}%", percent * 100.)),
    }
}

fn apply_dimension(name: &str, dimension: Dimension, css: CssBlockBuilder) -> CssBlockBuilder {
    if let Some(dimension) = dimension_css(dimension) {
        css.with_css_statement(format!("{}: {}", name, dimension))
    } else {
        css
    }
}
