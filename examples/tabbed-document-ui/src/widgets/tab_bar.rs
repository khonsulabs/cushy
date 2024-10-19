use std::hash::Hash;
use cushy::figures::units::Px;
use cushy::styles::Color;
use cushy::styles::components::DefaultBackgroundColor;
use cushy::value::{Destination, Dynamic, IntoValue};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};

pub trait Tab {
    fn label(&self) -> String;
    fn make_content(&self) -> WidgetInstance;
}

// NOTE: Specifically NOT clone because we don't want to clone 'next_id' or the TK instances.
pub struct TabBar<TK> {
    tabs: Vec<TK>,
    tab_items: Dynamic<WidgetList>,
    content_area: Dynamic<WidgetInstance>,

    selected: Dynamic<TK>,
    next_id: usize,
}

impl<TK: Tab + Hash + Eq> TabBar<TK> {
    pub fn new() -> Self {
        let tabs: Vec<TK> = vec![];
        let content_area = Dynamic::new(Space::clear().make_widget());

        Self {
            tabs,
            content_area,
            tab_items: Dynamic::new(WidgetList::new()),
            next_id: 0,
            selected: Dynamic::default(),
        }
    }

    pub fn add_tab(&mut self, tab: TK) {
        let content = tab.make_content();

        let tab_button = tab.label()
            .into_button()
            .on_click({
                let content_area = self.content_area.clone();
                move |_| content_area.set(content.clone())
            });

        let tab_id = self.new_tab_id();
        println!("tab_id: {}", tab_id);
        let select = self.selected.new_select(tab, tab_button);


        self.tab_items
            .lock()
            .push(select)
    }

    fn new_tab_id(&mut self) -> usize {

        let id = self.next_id;
        self.next_id += 1;

        id
    }

    pub fn make_widget(&self) -> WidgetInstance {
        let widget = TabBarWidget {
            tab_items: self.tab_items.clone(),
            content_area: self.content_area.clone(),
        };

        widget.make_widget()
    }
}

struct TabBarWidget {
    tab_items: Dynamic<WidgetList>,
    content_area: Dynamic<WidgetInstance>,
}

impl MakeWidget for TabBarWidget {
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