//! The tabs for the application.

use cushy::value::Dynamic;
use cushy::widget::{WidgetInstance};
use crate::action::Action;
use crate::app_tabs::document::{DocumentTab, DocumentTabAction, DocumentTabMessage};
use crate::app_tabs::home::{HomeTab, HomeTabAction, HomeTabMessage};
use crate::app_tabs::new::{NewTab, NewTabAction, NewTabMessage};
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

#[derive(Clone, Debug)]
pub enum TabKindMessage {
    HomeTabMessage(HomeTabMessage),
    DocumentTabMessage(DocumentTabMessage),
    NewTabMessage(NewTabMessage),
}

pub enum TabKindAction {
    HomeTabAction(TabKey, HomeTabAction),
    DocumentTabAction(TabKey, DocumentTabAction),
    NewTabAction(TabKey, NewTabAction),
}

impl Tab<TabKindMessage, TabKindAction> for TabKind {
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

    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: TabKindMessage) -> Action<TabKindAction> {
        match (self, message) {
            (TabKind::Home(tab), TabKindMessage::HomeTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|action|{
                        TabKindAction::HomeTabAction(tab_key, action)
                    })
            },
            (TabKind::New(tab), TabKindMessage::NewTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|action|{
                        TabKindAction::NewTabAction(tab_key, action)
                    })
            },
            (TabKind::Document(tab), TabKindMessage::DocumentTabMessage(message)) => {
                tab
                    .update(context, tab_key, message)
                    .map(|action|{
                        TabKindAction::DocumentTabAction(tab_key, action)
                    })
            },
            (_, _) => {
                unreachable!()
            },
        }
    }
}