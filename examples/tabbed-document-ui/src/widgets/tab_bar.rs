use std::default::Default;
use std::hash::Hash;
use slotmap::{new_key_type, SlotMap};
use cushy::figures::units::Px;
use cushy::styles::Color;
use cushy::styles::components::DefaultBackgroundColor;
use cushy::value::{Destination, Dynamic, IntoValue, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};

pub trait Tab {
    fn label(&self) -> String;
    fn make_content(&self) -> WidgetInstance;
}

// NOTE: Specifically NOT clone because we don't want to clone the tabs.
pub struct TabBar<TK> {
    tabs: Dynamic<SlotMap<TabKey, TK>>,
    tab_items: Dynamic<WidgetList>,
    content_area: Dynamic<WidgetInstance>,

    selected: Dynamic<Option<TabKey>>,
}

new_key_type! {
    pub struct TabKey;
}

// FIXME avoid the ` + Sync + Send + 'static` requirement if possible, required due to use of `Source::for_each`
impl<TK: Tab + Hash + Eq + Sync + Send + 'static> TabBar<TK> {
    pub fn new() -> Self {
        let tabs: SlotMap<TabKey, TK> = Default::default();
        let content_area = Dynamic::new(Space::clear().make_widget());

        Self {
            tabs: Dynamic::new(tabs),
            content_area,
            tab_items: Dynamic::new(WidgetList::new()),
            selected: Dynamic::new(None)
        }
    }

    pub fn add_tab(&mut self, tab: TK) {
        let tab_label = tab.label();

        let tab_key = self.tabs.lock().insert(tab);
        println!("tab_key: {:?}", tab_key);
        let select = self.selected.new_select(Some(tab_key), tab_label);

        self.tab_items
            .lock()
            .push(select);

        self.selected.set(Some(tab_key));
    }

    pub fn make_widget(&self) -> WidgetInstance {

        let callback = self.selected
            .for_each({
                let tabs = self.tabs.clone();
                let content_area = self.content_area.clone();
                move |selected_tab_key|{
                    if let Some(tab_key) = selected_tab_key.clone() {
                        let tab_binding = tabs.lock();
                        if let Some(tab) = tab_binding.get(tab_key) {
                            let content = tab.make_content();

                            content_area.set(content.clone())
                        }
                    }
                }
            });
        callback.persist();

        let widget = TabBarWidget {
            tab_items: self.tab_items.clone(),
            content_area: self.content_area.clone(),
        };

        widget.make_widget()
    }
}

// Intermediate widget, with only the things it needs, so that it's possible to call `make_widget` which consumes self.
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