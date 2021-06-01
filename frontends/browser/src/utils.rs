use std::fmt::Write;

use gooey_core::styles::style_sheet::Classes;
use wasm_bindgen::JsCast;
use web_sys::{CssStyleSheet, HtmlElement, HtmlStyleElement};

pub fn window_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

pub fn widget_css_id(widget_id: u32) -> String {
    format!("goo-{}", widget_id)
}

fn set_widget_id(element: &HtmlElement, widget_id: u32) {
    element.set_id(&widget_css_id(widget_id));
}

fn set_widget_classes(element: &HtmlElement, classes: Option<&Classes>) {
    if let Some(classes) = classes {
        element.set_class_name(&classes.join(" "));
    } else {
        drop(element.remove_attribute("class"));
    }
}

pub fn initialize_widget_element(element: &HtmlElement, widget_id: u32, classes: Option<&Classes>) {
    set_widget_id(element, widget_id);
    set_widget_classes(element, classes);
}

pub struct CssManager {
    sheet: CssStyleSheet,
}

impl CssManager {
    pub fn shared() -> Self {
        let doc = window_document();
        let style_tag = if let Some(style_tag) = doc.get_element_by_id("gooey-styles") {
            style_tag.unchecked_into::<HtmlStyleElement>()
        } else {
            let style = doc
                .create_element("style")
                .expect("error creating style")
                .unchecked_into::<web_sys::HtmlStyleElement>();
            style.set_id("gooey-styles");
            doc.head()
                .expect("missing <head>")
                .append_child(&style)
                .unwrap();
            style
        };

        Self {
            sheet: style_tag.sheet().expect("missing sheet").unchecked_into(),
        }
    }

    pub fn register_rule(&self, rule: &str) -> CssRule {
        CssRule {
            index: Some(self.sheet.insert_rule(rule).unwrap()),
        }
    }

    pub fn unregister_rule(&self, rule: &mut CssRule) {
        if let Some(index) = rule.index.take() {
            self.sheet.delete_rule(index).unwrap();
        }
    }
}

#[derive(Debug)]
pub struct CssRule {
    index: Option<u32>,
}

pub enum CssMapping {
    Color,
}

impl Drop for CssRule {
    fn drop(&mut self) {
        CssManager::shared().unregister_rule(self);
    }
}

pub struct CssBlockBuilder {
    selector: String,
    statements: Vec<String>,
}

impl CssBlockBuilder {
    pub fn for_id(widget_id: u32) -> Self {
        Self {
            selector: format!("#{}", widget_css_id(widget_id)),
            statements: Vec::default(),
        }
    }

    pub fn with_css_statement<S: ToString>(mut self, css: S) -> Self {
        self.statements.push(css.to_string());
        self
    }
}

impl std::fmt::Display for CssBlockBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.selector)?;
        f.write_char('{')?;
        for statement in &self.statements {
            f.write_str(&statement)?;
            f.write_char(';')?;
        }
        f.write_char('}')
    }
}
