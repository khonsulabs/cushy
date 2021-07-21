//! A [`Frontend`](gooey_core::Frontend) for `Gooey` that targets web browsers
//! by creating DOM elements using `web-sys` and `wasm-bindgen`.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // TODO missing_docs,
    clippy::nursery,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::if_not_else,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::multiple_crate_versions, // this is a mess due to winit dependencies and wgpu dependencies not lining up
    clippy::missing_errors_doc, // TODO clippy::missing_errors_doc
    clippy::missing_panics_doc, // TODO clippy::missing_panics_doc
)]
#![cfg_attr(doc, warn(rustdoc::all))]

use std::{
    any::TypeId,
    convert::TryFrom,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use gooey_core::{
    styles::{
        style_sheet::Classes, Alignment, Style, StyleComponent, SystemTheme, VerticalAlignment,
    },
    AnyTransmogrifier, AnyTransmogrifierContext, AnyWidget, Frontend, Gooey, Transmogrifier,
    TransmogrifierContext, TransmogrifierState, Widget, WidgetId, WidgetRef, WidgetRegistration,
};
use wasm_bindgen::{prelude::*, JsCast};

pub mod utils;

use utils::{set_widget_classes, set_widget_id, CssBlockBuilder, CssManager, CssRules};
use web_sys::{HtmlElement, MediaQueryListEvent};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct WebSys {
    pub ui: Gooey<Self>,
    styles: Arc<Vec<CssRules>>,
}

impl WebSys {
    pub fn initialize() {
        if INITIALIZED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            wasm_logger::init(wasm_logger::Config::default());
        }
    }

    #[must_use]
    pub fn new(ui: Gooey<Self>) -> Self {
        Self::initialize();

        let manager = CssManager::shared();
        let mut styles = vec![
            manager.register_rule(
                &CssBlockBuilder::for_id(ui.root_widget().id().id)
                    .with_css_statement("width: 100%")
                    .with_css_statement("height: 100%")
                    .with_css_statement("display: flex")
                    .to_string(),
            ),
            manager.register_rule(
                &CssBlockBuilder::for_css_selector("#gooey")
                    .with_css_statement("margin: 0")
                    .with_css_statement("padding: 0")
                    .to_string(),
            ),
        ];

        for rule in ui
            .stylesheet()
            .rules
            .iter()
            .filter(|rule| rule.widget_type_id.is_some())
        {
            if let Some(transmogrifier) =
                ui.transmogrifier_for_type_id(rule.widget_type_id.unwrap())
            {
                for theme in [SystemTheme::Light, SystemTheme::Dark] {
                    let css = transmogrifier.convert_style_to_css(
                        &rule.style,
                        CssBlockBuilder::for_classes_and_rule(
                            &transmogrifier.widget_classes(),
                            rule,
                        )
                        .with_theme(theme),
                    );
                    styles.push(manager.register_rule(&css.to_string()));
                }
            }
        }

        Self {
            ui,
            styles: Arc::new(styles),
        }
    }

    pub fn install_in_id(&mut self, id: &str) {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let parent = document.get_element_by_id(id).expect("id not found");

        let root_id = id.to_owned();
        // Initialize with light theme
        update_system_theme(&root_id, false);

        // TODO This shouldn't leak, it should be stored somewhere, but since
        // wasm-bindgen isn't Send+Sync, we can't store it in `self`. Also,
        // todo: I don't know that the callback is being invoked. My testing has
        // been done limited so far.
        std::mem::forget(
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
                .and_then(move |mql| {
                    update_system_theme(&root_id, mql.matches());
                    mql.add_listener_with_opt_callback(Some(
                        Closure::wrap(Box::new(move |event: MediaQueryListEvent| {
                            update_system_theme(&root_id, event.matches());
                        }) as Box<dyn FnMut(_)>)
                        .as_ref()
                        .unchecked_ref(),
                    ))
                    .map(|_| mql)
                    .ok()
                }),
        );

        self.with_transmogrifier(self.ui.root_widget().id(), |transmogrifier, mut context| {
            if let Some(root_element) = transmogrifier.transmogrify(&mut context) {
                parent.append_child(&root_element).unwrap();
            }
        });
    }

    /// Executes `callback` with the transmogrifier and transmogrifier state as
    /// parameters.
    #[allow(clippy::missing_panics_doc)] // unwrap is guranteed due to get_or_initialize
    pub fn with_transmogrifier<
        TResult,
        C: FnOnce(&'_ dyn AnyWebSysTransmogrifier, AnyTransmogrifierContext<'_, Self>) -> TResult,
    >(
        &self,
        widget_id: &WidgetId,
        callback: C,
    ) -> Option<TResult> {
        self.ui
            .with_transmogrifier(widget_id, self, |transmogrifier, context| {
                callback(transmogrifier.as_ref(), context)
            })
    }
}

fn update_system_theme(root_id: &str, dark: bool) {
    let (active_theme, inactive_theme) = if dark {
        ("gooey-dark", "gooey-light")
    } else {
        ("gooey-light", "gooey-dark")
    };

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let parent = document.get_element_by_id(root_id).expect("id not found");
    let class_list = parent.class_list();
    drop(class_list.add_1(active_theme));
    drop(class_list.remove_1(inactive_theme));
}

#[derive(Debug)]
pub struct RegisteredTransmogrifier(pub Box<dyn AnyWebSysTransmogrifier>);

impl AnyWebSysTransmogrifier for RegisteredTransmogrifier {
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        self.0.transmogrify(context)
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.0.convert_style_to_css(style, css)
    }

    fn widget_classes(&self) -> Classes {
        self.0.widget_classes()
    }
}

impl Deref for RegisteredTransmogrifier {
    type Target = Box<dyn AnyWebSysTransmogrifier>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
    }
}

impl gooey_core::Frontend for WebSys {
    type AnyTransmogrifier = RegisteredTransmogrifier;
    type Context = Self;

    fn gooey(&self) -> &'_ Gooey<Self> {
        &self.ui
    }

    fn set_widget_has_messages(&self, widget: WidgetId) {
        self.gooey().set_widget_has_messages(widget);
        // If we're not inside of a render
        if !self.gooey().is_managed_code() {
            self.gooey().process_widget_messages(self);
        }
    }
}

pub trait WebSysTransmogrifier: Transmogrifier<WebSys> {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement>;

    #[must_use]
    fn initialize_widget_element(
        &self,
        element: &HtmlElement,
        context: &TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<CssRules> {
        set_widget_id(element, context.registration.id().id);
        let mut classes = Self::widget_classes();
        if let Some(custom_classes) = context.style.get::<Classes>() {
            classes = classes.merge(custom_classes);
        }
        set_widget_classes(element, &classes);
        let mut rules = None;

        for theme in [SystemTheme::Light, SystemTheme::Dark] {
            let mut css = CssBlockBuilder::for_id(context.registration.id().id).with_theme(theme);
            css = self.convert_style_to_css(context.style, css);

            let style_sheet = context.frontend.gooey().stylesheet();
            if let Some(rules) = style_sheet.rules_by_widget.get(&None) {
                for &rule_index in rules {
                    let rule = &style_sheet.rules[rule_index];
                    if rule.widget_type_id.is_none()
                        && rule.applies(context.ui_state, Some(&classes))
                    {
                        css = self.convert_style_to_css(&rule.style, css);
                    }
                }
            }

            let css = css.to_string();
            rules = Some(rules.map_or_else(
                || CssManager::shared().register_rule(&css),
                |existing: CssRules| existing.and(&css),
            ));
        }
        rules
    }

    #[allow(unused_variables)]
    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_standard_components_to_css(style, css)
    }

    fn convert_standard_components_to_css(
        &self,
        style: &Style,
        css: CssBlockBuilder,
    ) -> CssBlockBuilder {
        self.convert_alignment_to_css(style, self.convert_colors_to_css(style, css))
    }

    fn convert_colors_to_css(&self, style: &Style, mut css: CssBlockBuilder) -> CssBlockBuilder {
        if let Some(color) = <Self::Widget as Widget>::text_color(style) {
            let color = color
                .themed_color(css.theme.expect("theme is required"))
                .as_css_string();
            css = css.with_css_statement(format!("color: {}", color));
        }
        if let Some(color) = <Self::Widget as Widget>::background_color(style) {
            let color = color
                .themed_color(css.theme.expect("theme is required"))
                .as_css_string();
            css = css.with_css_statement(format!("background-color: {}", color));
        }
        css
    }

    /// Converts [`Alignment`] and [`VerticalAlignment`] components to CSS
    /// rules. Also emits `display: flex` if any alignments are set.
    fn convert_alignment_to_css(&self, style: &Style, mut css: CssBlockBuilder) -> CssBlockBuilder {
        let alignment = style.get::<Alignment>();
        let vertical_alignment = style.get::<VerticalAlignment>();

        if vertical_alignment.is_some() || alignment.is_some() {
            css = css.with_css_statement("display: flex");
        }

        if let Some(alignment) = alignment {
            css = css.with_css_statement(format!("justify-content: {}", match alignment {
                Alignment::Left => "start",
                Alignment::Center => "center",
                Alignment::Right => "end",
            },))
        }
        if let Some(vertical_alignment) = vertical_alignment {
            css = css.with_css_statement(format!("align-items: {}", match vertical_alignment {
                VerticalAlignment::Top => "start",
                VerticalAlignment::Center => "center",
                VerticalAlignment::Bottom => "end",
            },))
        }
        css
    }

    #[must_use]
    fn widget_classes() -> Classes {
        Classes::from(<<Self as Transmogrifier<WebSys>>::Widget as Widget>::CLASS)
    }
}

pub trait AnyWebSysTransmogrifier: AnyTransmogrifier<WebSys> + Send + Sync {
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement>;

    #[allow(unused_variables)]
    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder;

    fn widget_classes(&self) -> Classes;
}

impl<T> AnyWebSysTransmogrifier for T
where
    T: WebSysTransmogrifier + AnyTransmogrifier<WebSys> + Send + Sync + 'static,
{
    fn transmogrify(
        &self,
        context: &mut AnyTransmogrifierContext<'_, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        <Self as WebSysTransmogrifier>::transmogrify(
            self,
            TransmogrifierContext::try_from(context).unwrap(),
        )
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        <Self as WebSysTransmogrifier>::convert_style_to_css(self, style, css)
    }

    fn widget_classes(&self) -> Classes {
        <Self as WebSysTransmogrifier>::widget_classes()
    }
}

impl AnyTransmogrifier<WebSys> for RegisteredTransmogrifier {
    fn process_messages(&self, context: AnyTransmogrifierContext<'_, WebSys>) {
        self.0.process_messages(context)
    }

    fn widget_type_id(&self) -> TypeId {
        self.0.widget_type_id()
    }

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &WidgetRegistration,
        frontend: &WebSys,
    ) -> TransmogrifierState {
        self.0.default_state_for(widget, registration, frontend)
    }
}

#[macro_export]
macro_rules! make_browser {
    ($transmogrifier:ident) => {
        impl From<$transmogrifier> for $crate::RegisteredTransmogrifier {
            fn from(transmogrifier: $transmogrifier) -> Self {
                Self(std::boxed::Box::new(transmogrifier))
            }
        }
    };
}

pub struct WidgetClosure;

impl WidgetClosure {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<F: Frontend, W: Widget, C: FnMut() -> <W as Widget>::Event + 'static>(
        widget: WidgetRef<W>,
        mut event_generator: C,
    ) -> Closure<dyn FnMut()> {
        Closure::wrap(Box::new(move || {
            let event = event_generator();
            widget.post_event::<F>(event);
        }) as Box<dyn FnMut()>)
    }
}
