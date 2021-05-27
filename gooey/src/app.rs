use gooey_core::{Frontend, Transmogrifiers, Widget, WidgetStorage};

#[derive(Default, Debug)]
pub struct App {
    transmogrifiers: Transmogrifiers<crate::ActiveFrontend>,
}

impl App {
    pub fn with<T: Into<<crate::ActiveFrontend as Frontend>::AnyTransmogrifier>>(
        mut self,
        transmogrifier: T,
    ) -> Self {
        self.transmogrifiers
            .register_transmogrifier(transmogrifier)
            .expect("a transmogrifier is already registered for this widget");
        self
    }

    pub fn run<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(self, initializer: C) {
        crate::main_with(self.transmogrifiers, initializer)
    }
}
