//! The tabs for the application.

use cushy::widget::{WidgetInstance};
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::context::Context;
use crate::widgets::tab_bar::Tab;

pub mod document;
pub mod home;

#[derive(Clone, Copy)]
pub enum TabKind {
    Home(HomeTab),
    Document(DocumentTab),
}

impl Tab for TabKind {
    fn label(&self) -> String {
        match self {
            TabKind::Home(tab) => tab.create_label(),
            TabKind::Document(tab) => tab.create_label(),
        }
    }

    fn make_content(&self, context: &mut Context) -> WidgetInstance {
        match self {
            TabKind::Home(tab) => tab.create_content(context),
            TabKind::Document(tab) => tab.create_content(context),
        }
    }
}