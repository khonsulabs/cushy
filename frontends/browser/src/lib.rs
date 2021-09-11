//! A [`Frontend`](gooey_core::Frontend) for `Gooey` that targets web browsers
//! by creating DOM elements using `web-sys` and `wasm-bindgen`.
//!
//! ## [`Window`] implementation
//!
//! The [`Window`] implementation in this frontend is limited by the browser's APIs. Of
//! note:
//!
//! - [`maximized()`](Window::maximized) and [`set_maximized()`](Window::set_maximized) interact with the fullscreen APIs.
//! - [`inner_position()`](Window::inner_position) and [`outer_position()`](Window::outer_position) are equivalent. This also means
//!   that [`set_outer_position()`](Window::set_outer_position) actually sets the inner position.
//! - Controlling the browser window (closing, resizing, moving) is dependent
//!   upon the browser allowing the operation. This generally is only possible
//!   if the window is opened via a javascript API and has no other tabs within
//!   in.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // TODO missing_docs,
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
    collections::HashMap,
    convert::TryFrom,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use gooey_core::{
    assets::{self, Configuration, FrontendImage, Image},
    figures::{Point, Size},
    styles::{
        border::Border,
        style_sheet::{Classes, State},
        Alignment, Autofocus, FontFamily, FontSize, Intent, Padding, Style, StyleComponent,
        SystemTheme, TabIndex, VerticalAlignment,
    },
    AnyTransmogrifier, AnyTransmogrifierContext, AnyWidget, AnyWindowBuilder, Callback, Frontend,
    Gooey, NativeTimer, Timer, Transmogrifier, TransmogrifierContext, TransmogrifierState,
    WeakWidgetRegistration, Widget, WidgetId, WidgetRef, WidgetRegistration, Window,
    WindowConfiguration,
};
use parking_lot::Mutex;
use wasm_bindgen::{prelude::*, JsCast};

pub mod utils;

use utils::{
    create_element, set_widget_classes, set_widget_id, widget_css_id, window_element_by_widget_id,
    CssBlockBuilder, CssManager, CssRules,
};
use web_sys::{
    window, ErrorEvent, HtmlElement, HtmlImageElement, KeyboardEvent, MediaQueryListEvent,
};

use crate::utils::window_document;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// `WebSys` is a [`Frontend`] implementor that implements Gooey interfaces
/// through `web-sys` in a web browser.
#[derive(Debug, Clone)]
pub struct WebSys {
    pub ui: Gooey<Self>,
    data: Arc<Data>,
}

#[derive(Debug)]
struct Data {
    styles: Vec<CssRules>,
    theme: Mutex<SystemTheme>,
    intent_callbacks: Mutex<HashMap<Intent, Vec<CallbackWidget>>>,
    configuration: Configuration,
    last_image_id: AtomicU64,
}

#[derive(Debug)]
struct CallbackWidget {
    widget: WeakWidgetRegistration,
    callback: Callback,
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
    pub fn new(ui: Gooey<Self>, configuration: Configuration) -> Self {
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
            manager.register_rule(
                &CssBlockBuilder::for_css_selector(".sr-only")
                    .with_css_statement("position: absolute")
                    .with_css_statement("height: 1px")
                    .with_css_statement("width: 1px")
                    .with_css_statement("clip: rect(0 0 0 0)")
                    .with_css_statement("clip-path: inset(100%)")
                    .with_css_statement("overflow: hidden")
                    .with_css_statement("white-space: nowrap")
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
            data: Arc::new(Data {
                styles,
                configuration,
                theme: Mutex::default(),
                intent_callbacks: Mutex::default(),
                last_image_id: AtomicU64::new(0),
            }),
        }
    }

    pub fn install_in_id(&mut self, id: &str, _window_config: WindowConfiguration) {
        // TODO handle window_config
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let parent = document.get_element_by_id(id).expect("id not found");

        let root_id = id.to_owned();
        // Initialize with light theme
        update_system_theme(&root_id, false, &self.data.theme);

        // TODO This shouldn't leak, it should be stored somewhere, but since
        // wasm-bindgen isn't Send+Sync, we can't store it in `self`. Also,
        // todo: I don't know that the callback is being invoked. My testing has
        // been done limited so far.
        let data = self.data.clone();
        std::mem::forget(
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
                .and_then(move |mql| {
                    update_system_theme(&root_id, mql.matches(), &data.theme);
                    mql.add_listener_with_opt_callback(Some(
                        Closure::wrap(Box::new(move |event: MediaQueryListEvent| {
                            update_system_theme(&root_id, event.matches(), &data.theme);
                        }) as Box<dyn FnMut(_)>)
                        .as_ref()
                        .unchecked_ref(),
                    ))
                    .map(|_| mql)
                    .ok()
                }),
        );

        // Install a keyboard handler in the window for handling enter/escape keypresses
        let data = self.data.clone();
        document.set_onkeydown(
            Closure::wrap(Box::new(move |event: KeyboardEvent| {
                let intent = match event.key().as_str() {
                    "Enter" | "Return" => Intent::Default,
                    "Escape" => Intent::Cancel,
                    _ => return,
                };
                let mut intent_callbacks = data.intent_callbacks.lock();
                if let Some(potential_handlers) = intent_callbacks.get_mut(&intent) {
                    // Widgets report themselves as handlers when they initially
                    // transmogrify. The order of this execution is such that
                    // the earliest widgets should be closer to the root. This
                    // isn't perfect at all, but putting multiple submit buttons
                    // on the same layer should be discouraged.
                    while !potential_handlers.is_empty() {
                        let handler = potential_handlers.last().unwrap();
                        // Check that the widget is still alive.
                        if handler.widget.upgrade().is_some() {
                            handler.callback.invoke(());
                            event.prevent_default();
                            break;
                        }

                        // If the widget is dead, pop it off the queue.
                        potential_handlers.pop();
                    }
                }
            }) as Box<dyn FnMut(KeyboardEvent)>)
            .into_js_value()
            .dyn_ref(),
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

    pub fn register_intent_handler(
        &self,
        intent: Intent,
        widget: &WidgetRegistration,
        callback: Callback,
    ) {
        let mut intent_callbacks = self.data.intent_callbacks.lock();
        let callbacks = intent_callbacks.entry(intent).or_default();
        callbacks.push(CallbackWidget {
            widget: WeakWidgetRegistration::from(widget),
            callback,
        });
    }
}

fn update_system_theme(root_id: &str, dark: bool, theme: &Mutex<SystemTheme>) {
    let (system_theme, active_theme, inactive_theme) = if dark {
        (SystemTheme::Dark, "gooey-dark", "gooey-light")
    } else {
        (SystemTheme::Light, "gooey-light", "gooey-dark")
    };

    let mut theme = theme.lock();
    *theme = system_theme;

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

    fn theme(&self) -> SystemTheme {
        let theme = self.data.theme.lock();
        *theme
    }

    fn load_image(&self, image: &Image, completed: Callback<Image>, error: Callback<String>) {
        if let Some(url) = Frontend::asset_url(self, &image.asset) {
            let element = create_element::<HtmlImageElement>("img");
            let image_id = self.data.last_image_id.fetch_add(1, Ordering::SeqCst);
            image.set_data(LoadedImageId(image_id));
            element.set_id(&image.css_id().expect("type error"));
            element.set_onerror(Some(
                &Closure::once_into_js(move |e: ErrorEvent| {
                    error.invoke(e.message());
                })
                .unchecked_into(),
            ));
            let completed_image = image.clone();
            element.set_onload(Some(
                &Closure::once_into_js(move || {
                    completed.invoke(completed_image);
                })
                .unchecked_into(),
            ));
            element.set_src(&url.to_string());

            // Store it in the <head>
            window_document()
                .head()
                .unwrap()
                .append_child(&element)
                .unwrap();
        }
    }

    fn asset_configuration(&self) -> &assets::Configuration {
        &self.data.configuration
    }

    fn schedule_timer(
        &self,
        callback: Callback,
        period: std::time::Duration,
        repeating: bool,
    ) -> Timer {
        let browser = BrowserTimer {
            timeout_id: 0,
            callback,
            period,
            repeating,
        };
        let timer = Timer::from_native(browser);
        schedule_timer(&timer, period);
        timer
    }

    fn window(&self) -> Option<&dyn gooey_core::Window> {
        Some(self)
    }

    fn open(&self, _window: Box<dyn AnyWindowBuilder>) -> bool {
        false
    }
}

fn schedule_timer(timer: &Timer, period: Duration) {
    let weak_timer = timer.downgrade();
    // TODO this should be refactored to reuse the same closure rather than rescheduling it.
    let timeout_id = web_sys::window()
        .unwrap()
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            Closure::once_into_js(move || {
                if let Some(timer) = weak_timer.upgrade() {
                    let reschedule = {
                        let native = timer.native::<BrowserTimer>().unwrap();
                        native.callback.invoke(());

                        native.repeating
                    };

                    if reschedule {
                        schedule_timer(&timer, period);
                    }
                }
            })
            .as_ref()
            .unchecked_ref(),
            i32::try_from(period.as_millis()).unwrap_or(i32::MAX),
        )
        .unwrap();
    let mut native = timer.native::<BrowserTimer>().unwrap();
    native.timeout_id = timeout_id;
}

#[derive(Debug)]
struct BrowserTimer {
    timeout_id: i32,
    callback: Callback,
    period: Duration,
    repeating: bool,
}

impl NativeTimer for BrowserTimer {}

#[derive(Clone, Debug)]
struct LoadedImageId(u64);

impl Drop for LoadedImageId {
    fn drop(&mut self) {
        let css_id = image_css_id(self.0);
        if let Some(element) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|doc| doc.get_element_by_id(&css_id))
        {
            element.remove();
        } else {
            // This should only happen if an `Image` was passed to a separate
            // thread, which is a no-no for Gooey at the moment.
            log::error!("unable to clean up dropped image: {}", css_id);
        }
    }
}

impl FrontendImage for LoadedImageId {
    fn size(&self) -> Option<gooey_core::figures::Size<u32, gooey_core::Pixels>> {
        let css_id = image_css_id(self.0);
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|doc| doc.get_element_by_id(&css_id))
            .and_then(|img| img.dyn_into::<HtmlImageElement>().ok())
            .map(|img| Size::new(img.width() as u32, img.height() as u32))
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
        let widget_id = context.registration.id().id;
        set_widget_id(element, widget_id);
        let mut classes = Self::widget_classes();
        if let Some(custom_classes) = context.style.get::<Classes>() {
            classes = classes.merge(custom_classes);
        }
        set_widget_classes(element, &classes);
        let mut rules = None;

        let style_sheet = context.frontend.gooey().stylesheet();
        for theme in [SystemTheme::Light, SystemTheme::Dark] {
            for state in State::permutations() {
                let mut css = CssBlockBuilder::for_id(widget_id)
                    .and_state(&state)
                    .with_theme(theme);

                let effective_style = style_sheet
                    .effective_style_for::<Self::Widget>(context.style().clone(), &state);
                css = self.convert_style_to_css(&effective_style, css);

                let css = css.to_string();
                rules = Some(rules.map_or_else(
                    || CssManager::shared().register_rule(&css),
                    |existing: CssRules| existing.and(&css),
                ));

                if let Some(extra_rules) = self.additional_css_rules(theme, &state, context) {
                    let rules = rules.as_mut().unwrap();
                    rules.extend(extra_rules);
                }
            }
        }

        if context.style.get::<Autofocus>().is_some() {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    Closure::once_into_js(move || {
                        if let Some(element) = window_element_by_widget_id::<HtmlElement>(widget_id)
                        {
                            drop(element.focus());
                        }
                    })
                    .as_ref()
                    .unchecked_ref(),
                    0,
                )
                .unwrap();
        }

        if let Some(index) = context.style.get::<TabIndex>() {
            // In CSS, tab index is limited to the bounds of an i16. 0 has a
            // special meaning, so we have to add 1 since TabIndex(0) is valid
            // in Gooey.
            let index = index
                .0
                .checked_add(1)
                .and_then(|index| i16::try_from(index).ok())
                .expect("tab index out of bounds");
            element.set_tab_index(i32::from(index));
        } else if <Self::Widget as Widget>::FOCUSABLE {
            element.set_tab_index(0);
        } else {
            element.set_tab_index(-1);
        }

        rules
    }

    #[allow(unused_variables)]
    fn additional_css_rules(
        &self,
        theme: SystemTheme,
        state: &State,
        context: &TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<CssRules> {
        None
    }

    fn convert_style_to_css(&self, style: &Style, css: CssBlockBuilder) -> CssBlockBuilder {
        self.convert_standard_components_to_css(style, css)
    }

    fn convert_standard_components_to_css(
        &self,
        style: &Style,
        mut css: CssBlockBuilder,
    ) -> CssBlockBuilder {
        css = self
            .convert_font_to_css(
                style,
                self.convert_alignment_to_css(style, self.convert_colors_to_css(style, css)),
            )
            .with_border(&style.get_or_default::<Border>());

        if let Some(padding) = style.get::<Padding>() {
            css = css.with_padding(padding);
        } else {
            css = css.with_css_statement("padding: 0");
        }

        css
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
            css = css.with_css_statement(format!(
                "justify-content: {}",
                match alignment {
                    Alignment::Left => "start",
                    Alignment::Center => "center",
                    Alignment::Right => "end",
                },
            ));
            css = css.with_css_statement(format!(
                "text-align: {}",
                match alignment {
                    Alignment::Left => "left",
                    Alignment::Center => "center",
                    Alignment::Right => "right",
                },
            ));
        }
        if let Some(vertical_alignment) = vertical_alignment {
            css = css.with_css_statement(format!(
                "align-items: {}",
                match vertical_alignment {
                    VerticalAlignment::Top => "start",
                    VerticalAlignment::Center => "center",
                    VerticalAlignment::Bottom => "end",
                },
            ));
        }
        css
    }

    fn convert_font_to_css(&self, style: &Style, mut css: CssBlockBuilder) -> CssBlockBuilder {
        if let Some(size) = style.get::<FontSize>() {
            // Accommodate for
            css = css.with_css_statement(format!("font-size: {:.03}pt;", size.get()));
        }

        if let Some(family) = style.get::<FontFamily>() {
            css = css.with_css_statement(format!("font-family: {};", family.0));
        }
        css
    }

    #[must_use]
    fn widget_classes() -> Classes {
        <<Self as Transmogrifier<WebSys>>::Widget as Widget>::classes()
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
        self.0.process_messages(context);
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

pub trait ImageExt {
    fn css_id(&self) -> Option<String>;
}

impl ImageExt for Image {
    fn css_id(&self) -> Option<String> {
        self.map_data(|opt_id| {
            opt_id
                .and_then(|id| id.as_any().downcast_ref::<LoadedImageId>())
                .map(|id| id.0)
        })
        .map(image_css_id)
    }
}

fn image_css_id(id: u64) -> String {
    format!("gooey-img-{}", id)
}
impl Window for WebSys {
    fn set_title(&self, title: &str) {
        let document = window_document();
        document.set_title(title);
    }

    fn inner_size(&self) -> Size<u32, gooey_core::Pixels> {
        let window = window().unwrap();
        let width = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default();
        let height = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default();

        Size::new(width, height).try_cast().unwrap_or_default()
    }

    fn set_inner_size(&self, new_size: Size<u32, gooey_core::Pixels>) {
        let window = window().unwrap();
        drop(window.resize_to(
            i32::try_from(new_size.width).unwrap(),
            i32::try_from(new_size.height).unwrap(),
        ));
    }

    fn set_outer_position(&self, new_position: Point<i32, gooey_core::Pixels>) {
        let window = window().unwrap();
        drop(window.move_to(new_position.x, new_position.y));
    }

    fn inner_position(&self) -> Point<i32, gooey_core::Pixels> {
        let window = window().unwrap();
        let x = window
            .screen_x()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default();
        let y = window
            .screen_y()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default();

        Point::new(x, y).try_cast().unwrap_or_default()
    }

    fn set_always_on_top(&self, _always: bool) {}

    fn maximized(&self) -> bool {
        window_document().fullscreen()
    }

    fn set_maximized(&self, maximized: bool) {
        if maximized {
            let body = window_document()
                .get_element_by_id(&widget_css_id(self.ui.root_widget().id().id))
                .unwrap();
            drop(body.request_fullscreen());
        } else {
            window_document().exit_fullscreen();
        }
    }

    fn set_minimized(&self, _minimized: bool) {}

    fn close(&self) {
        drop(window().unwrap().close());
    }
}
