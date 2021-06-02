use crate::{
    core::{Frontend, Gooey, StyledWidget, Transmogrifiers, Widget, WidgetStorage},
    frontends::browser::WebSys,
    style::default_stylesheet,
    widgets::browser::{default_transmogrifiers, register_transmogrifiers},
};

pub fn browser_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    mut transmogrifiers: Transmogrifiers<WebSys>,
    initializer: C,
) {
    register_transmogrifiers(&mut transmogrifiers);
    let mut ui = WebSys::new(Gooey::with(
        transmogrifiers,
        default_stylesheet(),
        initializer,
    ));
    ui.process_widget_messages();
    ui.install_in_id("gooey")
}

pub fn browser_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> StyledWidget<W>>(
    initializer: C,
) {
    browser_main_with(default_transmogrifiers(), initializer)
}
