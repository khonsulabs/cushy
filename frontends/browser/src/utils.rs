use std::fmt::Write;

use gooey_core::styles::{
    style_sheet::{Classes, Rule, State},
    StyleComponent, SystemTheme,
};
use wasm_bindgen::JsCast;
use web_sys::{CssStyleSheet, HtmlElement, HtmlStyleElement};

#[must_use]
pub fn window_document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

#[must_use]
pub fn create_element<T: JsCast>(name: &str) -> T {
    window_document()
        .create_element(name)
        .unwrap()
        .unchecked_into::<T>()
}

#[must_use]
pub fn window_element_by_widget_id<T: JsCast>(widget_id: u32) -> Option<T> {
    window_document()
        .get_element_by_id(&widget_css_id(widget_id))
        .and_then(|e| e.dyn_into::<T>().ok())
}

#[must_use]
pub fn widget_css_id(widget_id: u32) -> String {
    format!("gooey-{}", widget_id)
}

pub fn set_widget_id(element: &HtmlElement, widget_id: u32) {
    element.set_id(&widget_css_id(widget_id));
}

pub fn set_widget_classes(element: &HtmlElement, classes: &Classes) {
    element.set_class_name(&classes.to_vec().join(" "));
}

pub struct CssManager {
    sheet: CssStyleSheet,
}

impl CssManager {
    #[must_use]
    pub fn shared() -> Self {
        let doc = window_document();
        let style_tag = doc.get_element_by_id("gooey-styles").map_or_else(
            || {
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
            },
            JsCast::unchecked_into::<HtmlStyleElement>,
        );

        Self {
            sheet: style_tag.sheet().expect("missing sheet").unchecked_into(),
        }
    }

    pub fn register_rule(&self, rule: &str) -> CssRules {
        CssRules {
            index: vec![self.sheet.insert_rule(rule).unwrap()],
        }
    }

    pub fn unregister_rule(&self, rule: &mut CssRules) {
        for index in std::mem::take(&mut rule.index) {
            self.sheet.delete_rule(index).unwrap();
        }
    }
}

#[derive(Debug, Default)]
#[must_use]
pub struct CssRules {
    index: Vec<u32>,
}

pub enum CssMapping {
    Color,
}

impl Drop for CssRules {
    fn drop(&mut self) {
        CssManager::shared().unregister_rule(self);
    }
}

impl CssRules {
    pub fn and(mut self, rule: &str) -> Self {
        self.index
            .push(CssManager::shared().sheet.insert_rule(rule).unwrap());
        self
    }

    pub fn extend(&mut self, mut other: Self) {
        let other = std::mem::take(&mut other.index);
        self.index.extend(other.into_iter());
    }
}

#[must_use]
pub struct CssBlockBuilder {
    selector: String,
    pub theme: Option<SystemTheme>,
    statements: Vec<String>,
}

impl CssBlockBuilder {
    pub fn for_id(widget_id: u32) -> Self {
        Self {
            selector: format!("#{}", widget_css_id(widget_id)),
            statements: Vec::default(),
            theme: None,
        }
    }

    pub fn for_css_selector<S: ToString>(selector: S) -> Self {
        Self {
            selector: selector.to_string(),
            statements: Vec::default(),
            theme: None,
        }
    }

    pub fn for_classes(classes: &Classes) -> Self {
        Self {
            selector: format!(".{}", classes.to_vec().join(".")),
            statements: Vec::default(),
            theme: None,
        }
    }

    pub fn for_classes_and_rule(classes: &Classes, rule: &Rule) -> Self {
        let mut builder = rule.classes.as_ref().map_or_else(
            || Self::for_classes(classes),
            |rule_classes| Self::for_classes(&classes.merge(rule_classes)),
        );
        if let Some(active) = rule.active {
            builder.selector += if active { ":active" } else { ":not(:active)" };
        }
        if let Some(focused) = rule.focused {
            builder.selector += if focused { ":focus" } else { ":not(:focus)" };
        }
        if let Some(hovered) = rule.hovered {
            builder.selector += if hovered { ":hover" } else { ":not(:hover)" };
        }

        builder
    }

    pub fn and_state(mut self, state: &State) -> Self {
        self.selector += if state.active {
            ":active"
        } else {
            ":not(:active)"
        };
        self.selector += if state.focused {
            ":focus"
        } else {
            ":not(:focus)"
        };
        self.selector += if state.hovered {
            ":hover"
        } else {
            ":not(:hover)"
        };
        self
    }

    pub fn and_additional_selector(mut self, selector: &str) -> Self {
        self.selector.push_str(selector);
        self
    }

    pub fn with_css_statement<S: ToString>(mut self, css: S) -> Self {
        self.statements.push(css.to_string());
        self
    }

    pub const fn with_theme(mut self, theme: SystemTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

impl std::fmt::Display for CssBlockBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(theme) = self.theme {
            f.write_char('.')?;
            f.write_str(match theme {
                SystemTheme::Light => "gooey-light",
                SystemTheme::Dark => "gooey-dark",
            })?;
            f.write_char(' ')?;
        }
        f.write_str(&self.selector)?;
        f.write_char('{')?;
        for statement in &self.statements {
            f.write_str(statement)?;
            f.write_char(';')?;
        }
        f.write_str("}")
    }
}
