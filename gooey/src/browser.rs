use crate::{
    core::{Frontend, Gooey, Transmogrifiers, Widget, WidgetStorage},
    frontends::browser::WebSys,
    widgets::browser::{default_transmogrifiers, register_transmogrifiers},
};

pub fn browser_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(
    mut transmogrifiers: Transmogrifiers<WebSys>,
    initializer: C,
) {
    register_transmogrifiers(&mut transmogrifiers);
    let ui = WebSys::new(Gooey::with(transmogrifiers, initializer));
    ui.process_widget_messages();
    ui.install_in_id("gooey")
}

pub fn browser_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) {
    browser_main_with(default_transmogrifiers(), initializer)
}
