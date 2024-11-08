use std::default::Default;
use std::hash::Hash;
use std::io::Empty;
use std::sync::{Arc, Mutex};
use slotmap::{new_key_type, SlotMap};
use slotmap::basic::Iter;
use cushy::figures::units::Px;
use cushy::styles::{Color, CornerRadii, Dimension};
use cushy::styles::components::{CornerRadius, TextColor, WidgetBackground};
use cushy::value::{Destination, Dynamic, IntoValue, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};
use cushy::widgets::button::{ButtonActiveBackground, ButtonForeground, ButtonHoverForeground};
use cushy::widgets::select::SelectedColor;
use crate::context::Context;

pub trait Tab {
    fn label(&self) -> String;
    fn make_content(&self, context: &mut Context) -> WidgetInstance;
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

impl<TK: Tab + Hash + Eq + Send + 'static> TabBar<TK> {
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
        let select = self.selected
            .new_select(Some(tab_key), tab_label)
            .with(&ButtonForeground, Color::LIGHTGRAY)
            .with(&ButtonHoverForeground, Color::WHITE)
            .with(&ButtonActiveBackground, Color::GRAY)
            // TODO remove this workaround for the select button's background inheritance
            .with(&WidgetBackground, Color::CLEAR_BLACK)
            .with(&SelectedColor, Color::GRAY);

        self.tab_items
            .lock()
            .push(select);

        self.activate(tab_key)
    }

    pub fn close_all(&mut self) {
        self.selected.set(None);
        self.tab_items.lock().clear();
        self.tabs.lock().clear();
    }

    pub fn make_widget(&self, context: &mut Arc<Mutex<Context>>) -> WidgetInstance {

        let callback = self.selected.for_each({
            let tabs = self.tabs.clone();
            let content_area = self.content_area.clone();
            let context = context.clone();

            move |selected_tab_key|{
                let mut context_guard = context.lock().unwrap();
                let context = &mut *context_guard;

                println!("key: {:?}", selected_tab_key);
                if let Some(tab_key) = selected_tab_key.clone() {
                    let tab_binding = tabs.lock();
                    if let Some(tab) = tab_binding.get(tab_key) {
                        let content = tab.make_content(context);
                        content_area.set(content.clone())
                    }
                } else {
                    let no_tabs_content = Expand::empty().make_widget();

                    content_area.set(no_tabs_content);
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

    pub fn with_tabs<R, F>(&self, f: F) -> R
    where
        F: Fn(Iter<TabKey, TK>) -> R
    {
        let tabs = self.tabs.lock();
        let iter = tabs.iter();

        f(iter)
    }

    pub fn activate(&self, tab_key: TabKey) {
        self.selected.set(Some(tab_key));
    }
}

static VERY_DARK_GREY: Color = Color::new(0x32, 0x32, 0x32, 255);

// Intermediate widget, with only the things it needs, so that it's possible to call `make_widget` which consumes self.
struct TabBarWidget {
    tab_items: Dynamic<WidgetList>,
    content_area: Dynamic<WidgetInstance>,
}

impl MakeWidget for TabBarWidget {
    fn make_widget(self) -> WidgetInstance {

        let tab_bar = [
            Stack::new(Orientation::Column, self.tab_items)
                .make_widget(),
            Expand::empty()
                // FIXME this causes the tab bar to take the entire height of the area under the toolbar unless a height is specified
                //       but we don't want to specify a height in pixels, we want the height to be be automatic
                //       like it is when the background color is not specified.
                .with(&WidgetBackground, VERY_DARK_GREY)
                // FIXME remove this, see above.
                .height(Px::new(38))
                .make_widget(),
        ]
            .into_columns()
            .with(&WidgetBackground, VERY_DARK_GREY)
            .with(&TextColor, Color::GRAY);

        tab_bar
            .and(self.content_area.expand())
            .into_rows()
            .with(&CornerRadius, CornerRadii::from(Dimension::Px(Px::new(0))))
            .make_widget()
    }
}