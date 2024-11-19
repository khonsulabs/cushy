//! The tabs for the application.

use cushy::value::Dynamic;
use cushy::widget::{WidgetInstance};
use crate::app_tabs::document::{DocumentTab, DocumentTabMessage};
use crate::app_tabs::home::{HomeTab, HomeTabMessage};
use crate::app_tabs::new::{NewTab, NewTabMessage};
use crate::context::Context;
use crate::task::Task;
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

#[derive(Clone, PartialEq)]
pub enum TabKindMessage {
    HomeTabMessage(HomeTabMessage),
    DocumentTabMessage(DocumentTabMessage),
    NewTabMessage(NewTabMessage),
}


impl Tab<TabKindMessage> for TabKind {
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

    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: TabKindMessage) -> Task<TabKindMessage> {
        match (self, message) {
            (TabKind::Home(tab), TabKindMessage::HomeTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|message|{
                        TabKindMessage::HomeTabMessage(message)
                    })
            },
            (TabKind::New(tab), TabKindMessage::NewTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|message|{
                        TabKindMessage::NewTabMessage(message)
                    })
            },
            (TabKind::Document(tab), TabKindMessage::DocumentTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|message|{
                        TabKindMessage::DocumentTabMessage(message)
                    })
            },
            (_, _) => {
                unreachable!()
            },
        }
    }
}