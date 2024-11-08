//! The tabs for the application.

use cushy::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetInstance};
use crate::config::Config;
use crate::context::Context;
use crate::home;
use crate::widgets::tab_bar::Tab;

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum TabKind {
    Home,
    Document,
}

impl Tab for TabKind {
    fn label(&self) -> String {
        match self {
            TabKind::Home => "Home".to_string(),
            TabKind::Document => "Document".to_string(),
        }
    }

    fn make_content(&self, context: &mut Context) -> WidgetInstance {
        match self {
            TabKind::Home => {
                context.with_context::<Dynamic<Config>, _, _>(|config|{
                    let config = config.lock();
                    let show_on_startup_value = Dynamic::new(config.show_home_on_startup);
                    home::create_content(show_on_startup_value)
                }).unwrap()
            },
            TabKind::Document => "Document tab content".make_widget(),
        }
    }
}