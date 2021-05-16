use crate::{AnyWidget, Widget, WidgetState};

pub struct Gooey {
    root: Box<dyn AnyWidget>,
}

impl Gooey {
    pub fn new<W: Widget>(root: W) -> Self {
        Self {
            root: Box::new(WidgetState {
                widget: root,
                state: None,
            }),
        }
    }

    pub fn update(&mut self) -> bool {
        self.root.update()
    }

    pub fn root_widget(&self) -> &dyn AnyWidget {
        self.root.as_ref()
    }
}
