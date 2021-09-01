use std::sync::Arc;

use gooey_core::{assets::Configuration, AnyWindowBuilder, AppContext, WindowConfiguration};

use crate::{
    core::{Frontend, Gooey, Transmogrifiers, Widget, WidgetStorage},
    frontends::browser::WebSys,
    style::default_stylesheet,
    widgets::browser::{default_transmogrifiers, register_transmogrifiers},
};

/// Runs a browser-based [`App`](crate::app::App) with `transmogrifiers` and the
/// root widget from `initializer`. Unless overriden by `transmogrifier`, all
/// widgets from [`gooey::widget`](crate::widgets) will use the built-in
/// transmogrifiers.
pub fn browser_main_with<W: Widget + Send + Sync>(
    transmogrifiers: Transmogrifiers<WebSys>,
    mut initial_window: gooey_core::WindowBuilder<W>,
    context: AppContext,
) {
    browser_run(
        browser_app(transmogrifiers, &mut initial_window, context),
        initial_window.configuration,
    );
}

/// Runs a browser-based [`App`](crate::app::App) with the root widget from
/// `initializer`. All widgets from [`gooey::widget`](crate::widgets) will be
/// usable. If you wish to use other widgets, use `browser_main_with` and
/// provide the transmogrifiers for the widgets you wish to use.
pub fn browser_main<W: Widget + Send + Sync>(
    initial_window: gooey_core::WindowBuilder<W>,
    context: AppContext,
) {
    browser_main_with(default_transmogrifiers(), initial_window, context);
}

/// Returns an initialized frontend using the root widget returned from `initializer`.
pub fn browser_app(
    mut transmogrifiers: Transmogrifiers<WebSys>,
    builder: &mut dyn AnyWindowBuilder,
    context: AppContext,
) -> WebSys {
    register_transmogrifiers(&mut transmogrifiers);
    let transmogrifiers = Arc::new(transmogrifiers);
    let storage = WidgetStorage::new(context);
    let ui = WebSys::new(
        Gooey::new(
            transmogrifiers,
            default_stylesheet(),
            builder.build(&storage),
            storage,
        ),
        Configuration::default(),
    );
    ui.gooey().process_widget_messages(&ui);
    ui
}

/// Runs an initialized frontend.
pub fn browser_run(mut ui: WebSys, window_config: WindowConfiguration) {
    ui.install_in_id("gooey", window_config);
}
