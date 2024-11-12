use std::default::Default;
use std::hash::Hash;
use slotmap::{new_key_type, SlotMap};
use cushy::figures::units::Px;
use cushy::styles::{Color, CornerRadii, Dimension};
use cushy::styles::components::{CornerRadius, TextColor, WidgetBackground};
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};
use cushy::widgets::button::{ButtonActiveBackground, ButtonForeground, ButtonHoverForeground};
use cushy::widgets::select::SelectedColor;
use crate::context::Context;

pub trait Tab {
    fn label(&self, context: &mut Context) -> String;
    fn make_content(&self, context: &mut Context) -> WidgetInstance;
}

#[derive(PartialEq)]
enum TabState {
    Uninitialized,
    Active,
    Hidden(WidgetInstance)
}

impl Default for TabState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

// NOTE: Specifically NOT clone because we don't want to clone the tabs.
pub struct TabBar<TK> {
    tabs: Dynamic<SlotMap<TabKey, (TK, Dynamic<TabState>)>>,
    tab_items: Dynamic<WidgetList>,

    content_area: Dynamic<WidgetInstance>,

    selected: Dynamic<Option<TabKey>>,

    previous_tab: Dynamic<Option<TabKey>>,
}

new_key_type! {
    pub struct TabKey;
}

impl<TK: Tab + Send + Copy + 'static> TabBar<TK> {
    pub fn new() -> Self {
        let tabs: SlotMap<TabKey, (TK, Dynamic<TabState>)> = Default::default();
        let content_area = Dynamic::new(Space::clear().make_widget());

        Self {
            tabs: Dynamic::new(tabs),
            content_area,
            tab_items: Dynamic::new(WidgetList::new()),
            selected: Dynamic::new(None),
            previous_tab: Dynamic::new(None),
        }
    }

    pub fn add_tab(&mut self, context: &mut Context, tab: TK) -> TabKey {
        let tab_label = tab.label(context);
        let tab_content = tab.make_content(context);

        let tab_key = self.tabs.lock().insert((tab, Dynamic::new(TabState::Hidden(tab_content))));
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

        self.activate(tab_key);

        tab_key
    }

    pub fn close_all(&mut self) {
        self.selected.set(None);
        self.tab_items.lock().clear();
        self.tabs.lock().clear();
    }

    pub fn make_widget(&self) -> WidgetInstance {

        println!("TabBar::make_widget");

        let callback = self.selected.for_each({
            let tabs = self.tabs.clone();
            let content_area = self.content_area.clone();
            let previous_tab_key = self.previous_tab.clone();

            move |selected_tab_key|{

                println!("key: {:?}, previous_tab_key: {:?}", selected_tab_key, previous_tab_key.get());
                if let Some(tab_key) = selected_tab_key.clone() {
                    let mut tab_binding = tabs.lock();
                    let previous_tab = if let Some((_tab, tab_state)) = tab_binding.get_mut(tab_key) {

                        let tab_state_value = tab_state.take();

                        match tab_state_value {
                            TabState::Hidden(content_widget) => {

                                let previous_content_widget = content_area.replace(content_widget);

                                let result = match previous_tab_key.lock().take() {
                                    None => None,
                                    Some(previous_tab_key) => Some((previous_tab_key, previous_content_widget))
                                };

                                previous_tab_key.lock().replace(tab_key);

                                tab_state.set(TabState::Active);

                                result
                            }
                            TabState::Uninitialized => unreachable!(),
                            TabState::Active => None,
                        }
                    } else {
                        None
                    };

                    match previous_tab {
                        Some((tab_key, Some(widget))) => {
                            if let Some((_tab, tab_state)) = tab_binding.get_mut(tab_key) {
                                *tab_state.lock() = TabState::Hidden(widget);
                            }
                        }
                        _ => {}
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
        F: Fn(TabsIter<'_, TK>) -> R
    {
        let iter = self.into_iter();
        f(iter)
    }

    pub fn activate(&self, tab_key: TabKey) {
        self.selected.set(Some(tab_key));
    }
}

pub struct TabsIter<'a, TK> {
    tab_bar: &'a TabBar<TK>,
    keys: Vec<TabKey>,
    index: usize,
}

impl<'a, TK> TabsIter<'a, TK> {
    pub fn new(tab_bar: &'a TabBar<TK>) -> Self {
        let keys = tab_bar.tabs.lock().keys().collect();

        Self {
            tab_bar,
            keys,
            index: 0,
        }
    }
}

impl<'a, TK: Copy> Iterator for TabsIter<'a, TK> {
    type Item = (TabKey, TK);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];

            let binding = self.tab_bar.tabs.lock();
            let value = binding
                .get(key)
                .map(|(tab, _state) | (key, *tab) );

            self.index += 1;

            value

        } else {
            None
        }
    }
}

impl<'a, TK: Copy> IntoIterator for &'a TabBar<TK> {
    type Item = (TabKey, TK);
    type IntoIter = TabsIter<'a, TK>;

    fn into_iter(self) -> Self::IntoIter {
        TabsIter::new(
            self
        )
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