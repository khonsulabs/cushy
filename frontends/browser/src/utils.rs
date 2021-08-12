use std::{
    collections::HashSet,
    convert::TryInto,
    fmt::Write,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use gooey_core::{
    figures::{Approx, Figure},
    styles::{
        border::{Border, BorderOptions},
        style_sheet::{Classes, Rule, State},
        Padding, StyleComponent, SystemTheme,
    },
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use wasm_bindgen::JsCast;
use web_sys::{CssStyleSheet, HtmlElement, HtmlStyleElement};

static REGISTERED_CSS_RULES: OnceCell<Mutex<CssSheetState>> = OnceCell::new();
static LAST_CSS_RULE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct CssSheetState {
    taken_ids: HashSet<u32>,
    rules: Vec<CssRule>,
}

#[derive(Debug, Clone)]
struct CssRule {
    id: u32,
    #[cfg(debug_assertions)]
    css: Arc<String>,
}

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

    fn internal_register_rule(&self, rule: &str) -> CssRule {
        // The CssStyleSheet type manages rules by index, which are not stable
        // when you begin removing existing entries. Thus, we create a unique id
        // for each registered rule and keep our own Vec<> of rules that matches
        // the order of the stylesheet rules. When removing, we can use the
        // index we find the rule inside of the Vec.
        let mut state = REGISTERED_CSS_RULES.get_or_init(Mutex::default).lock();
        let id = loop {
            let next_id = LAST_CSS_RULE_ID.fetch_add(1, Ordering::SeqCst);
            if !state.taken_ids.contains(&next_id) {
                break next_id;
            }
        };
        state.taken_ids.insert(id);
        let css_rule = CssRule {
            id,
            #[cfg(debug_assertions)]
            css: Arc::new(rule.to_string()),
        };
        self.sheet
            .insert_rule_with_index(rule, state.rules.len().try_into().unwrap())
            .unwrap();
        state.rules.push(css_rule.clone());

        css_rule
    }

    pub fn register_rule(&self, rule: &str) -> CssRules {
        CssRules {
            index: vec![self.internal_register_rule(rule)],
        }
    }

    pub fn unregister_rule(&self, rule: &mut CssRules) {
        let mut state = REGISTERED_CSS_RULES.get().unwrap().lock();
        for rule in std::mem::take(&mut rule.index) {
            let index = state
                .rules
                .binary_search_by_key(&rule.id, |rule| rule.id)
                .unwrap();
            state.rules.remove(index);
            self.sheet.delete_rule(index.try_into().unwrap()).unwrap();
            state.taken_ids.remove(&rule.id);
        }
    }
}

#[derive(Debug, Default)]
#[must_use]
pub struct CssRules {
    index: Vec<CssRule>,
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
            .push(CssManager::shared().internal_register_rule(rule));
        self
    }

    pub fn extend(&mut self, mut other: Self) {
        let other_index = std::mem::take(&mut other.index);
        self.index.extend(other_index.into_iter());
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

    pub fn with_padding(self, padding: &Padding) -> CssBlockBuilder {
        let left = padding.left.unwrap_or_default();
        let right = padding.right.unwrap_or_default();
        let top = padding.top.unwrap_or_default();
        let bottom = padding.bottom.unwrap_or_default();
        if left.approx_eq(&right) {
            if top.approx_eq(&bottom) {
                if top.approx_eq(&left) {
                    self.with_css_statement(&format!("padding: {:03}pt", top.get()))
                } else {
                    self.with_css_statement(&format!(
                        "padding: {:03}pt {:03}pt",
                        top.get(),
                        left.get()
                    ))
                }
            } else {
                self.with_css_statement(&format!(
                    "padding: {:03}pt {:03}pt {:03}pt",
                    top.get(),
                    left.get(),
                    bottom.get(),
                ))
            }
        } else {
            self.with_css_statement(&format!(
                "padding: {:03}pt {:03}pt {:03}pt {:03}pt",
                top.get(),
                left.get(),
                bottom.get(),
                right.get(),
            ))
        }
    }

    fn with_single_border(self, name: &str, options: &BorderOptions) -> Self {
        if options.width.get() > 0. {
            self.with_css_statement(&format!(
                "border-{}: {:.03}pt solid {}",
                name,
                options.width.get(),
                options.color.as_css_string()
            ))
        } else {
            self.with_css_statement(&format!("border-{}: none", name))
        }
    }

    pub fn with_border(self, border: &Border) -> Self {
        let left = border.left.unwrap_or_default();
        let right = border.right.unwrap_or_default();
        let top = border.top.unwrap_or_default();
        let bottom = border.bottom.unwrap_or_default();
        let widths_are_same = left.width.approx_eq(&right.width)
            && top.width.approx_eq(&bottom.width)
            && left.width.approx_eq(&top.width);
        let has_border = !widths_are_same || !left.width.approx_eq(&Figure::default());

        if has_border {
            let one_rule = widths_are_same
                && left.color == right.color
                && top.color == bottom.color
                && left.color == top.color;

            if one_rule {
                self.with_css_statement(&format!(
                    "border: {:.03}pt solid {}",
                    left.width.get(),
                    left.color.as_css_string(),
                ))
            } else {
                self.with_single_border("left", &left)
                    .with_single_border("right", &right)
                    .with_single_border("top", &top)
                    .with_single_border("bottom", &bottom)
            }
        } else {
            // no border
            self.with_css_statement("border: none")
        }
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
