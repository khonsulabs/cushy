use std::collections::HashMap;

use gooey_browser::{
    utils::{window_document, CssBlockBuilder, CssManager, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{Transmogrifier, TransmogrifierContext};
use wasm_bindgen::JsCast;

use crate::layout::{Dimension, Layout, LayoutChildren, LayoutTransmogrifier, WidgetLayout};

impl Transmogrifier<WebSys> for LayoutTransmogrifier {
    type State = HashMap<Option<u32>, Vec<CssRules>>;
    type Widget = Layout;
}

impl WebSysTransmogrifier for LayoutTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let css_manager = CssManager::shared();

        let container = window_document()
            .create_element("div")
            .expect("error creating div")
            .unchecked_into::<web_sys::HtmlDivElement>();
        let mut css_rules = Vec::new();
        if let Some(rule) = self.initialize_widget_element(&container, &context) {
            css_rules.push(rule);
        }

        let container_css = CssBlockBuilder::for_id(context.registration.id().id)
            .with_css_statement("width: 100%")
            .with_css_statement("height: 100%")
            .with_css_statement("position: relative");
        css_rules.push(css_manager.register_rule(&container_css.to_string()));
        context.state.insert(None, css_rules);

        for layout_child in context.widget.layout_children() {
            context.frontend.with_transmogrifier(
                layout_child.registration.id(),
                |child_transmogrifier, mut child_context| {
                    if let Some(child) = child_transmogrifier.transmogrify(&mut child_context) {
                        context
                            .state
                            .insert(Some(layout_child.registration.id().id), vec![css_manager
                                .register_rule(
                                    &apply_layout_rules(
                                        &layout_child.layout,
                                        CssBlockBuilder::for_id(layout_child.registration.id().id)
                                            .with_css_statement("position: absolute"),
                                    )
                                    .to_string(),
                                )]);
                        container
                            .append_child(&child)
                            .expect("error appending child");
                    }
                },
            );
        }
        Some(container.unchecked_into())
    }
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
