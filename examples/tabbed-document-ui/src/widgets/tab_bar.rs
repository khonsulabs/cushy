use std::collections::HashMap;
use std::hash::Hash;
use cushy::figures::units::Px;
use cushy::styles::Color;
use cushy::styles::components::DefaultBackgroundColor;
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};
use crate::tabs::TabKind;

pub trait TabThing {
    fn label(&self) -> String;
    fn make_content(&self) -> WidgetInstance;
}

#[derive(Clone)]
pub struct TabBar<TK: Clone> {
    tabs: Vec<TK>,
    tab_items: Dynamic<WidgetList>,
    content_area: Dynamic<WidgetInstance>,
}

impl<TK: TabThing + Hash + Eq + Clone > TabBar<TK> {
    pub fn new() -> Self {
        let tabs: Vec<TK> = vec![];
        let content_area = Dynamic::new(Space::clear().make_widget());

        Self {
            tabs,
            content_area,
            tab_items: Dynamic::new(WidgetList::new()),
        }
    }

    pub fn add_tab(&mut self, tab: TK) {
        let content = tab.make_content();

        let tab_button = tab.label()
            .into_button()
            .on_click({
                let content_area = self.content_area.clone();
                move |_| content_area.set(content.clone())
            })
            .make_widget();
        self.tab_items.lock().push(tab_button)
    }

}

impl<TK: Clone> MakeWidget for TabBar<TK> {
    fn make_widget(self) -> WidgetInstance {

        let tab_bar: Stack = [
            Stack::new(Orientation::Column, self.tab_items)
                .make_widget(),
            Expand::empty()
                .with(&DefaultBackgroundColor, Color::RED)
                .height(Px::new(32))
                .make_widget(),
        ].into_columns();

        tab_bar
            .and(self.content_area.expand())
            .into_rows()
            .make_widget()
    }
}