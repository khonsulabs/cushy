//! The tabs for the application.

use cushy::value::Dynamic;
use cushy::widget::{WidgetInstance};
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::new::NewTab;
use crate::context::Context;
use crate::widgets::tab_bar::{Tab, TabKey};

pub mod document;
pub mod home;
pub mod new;

#[derive(Clone)]
pub enum TabKind {
    Home(HomeTab),
    Document(DocumentTab),
    New(NewTab),
}

impl Tab for TabKind {
    fn label(&self, context: &Dynamic<Context>) -> String {
        match self {
            TabKind::Home(tab) => tab.label(context),
            TabKind::Document(tab) => tab.label(context),
            TabKind::New(tab) => tab.label(context),
        }
    }

    fn make_content(&self, context: &Dynamic<Context>, tab_key: TabKey) -> WidgetInstance {
        match self {
            TabKind::Home(tab) => tab.make_content(context, tab_key),
            TabKind::Document(tab) => tab.make_content(context, tab_key),
            TabKind::New(tab) => tab.make_content(context, tab_key),
        }
    }
}