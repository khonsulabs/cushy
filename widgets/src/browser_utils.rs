use std::collections::HashMap;

use gooey_core::styles::style_sheet::Rule;

pub fn window_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

pub fn widget_css_id(widget_id: u32) -> String {
    format!("goo-{}", widget_id)
}

pub struct CssManager;

pub struct CssRule {
    pub source: Rule,
    pub index: u32,
    pub mappings: HashMap<String, CssMapping>,
}

impl CssManager {}

pub enum CssMapping {
    Color,
}
