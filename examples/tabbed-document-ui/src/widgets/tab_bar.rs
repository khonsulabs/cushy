use std::default::Default;
use std::hash::Hash;
use slotmap::{new_key_type, SlotMap};
use cushy::figures::units::Px;
use cushy::styles::{Color, CornerRadii, Dimension, Edges};
use cushy::styles::components::{CornerRadius, IntrinsicPadding, TextColor, WidgetBackground};
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};
use cushy::widgets::button::{ButtonActiveBackground, ButtonActiveForeground, ButtonBackground, ButtonForeground, ButtonHoverForeground};
use cushy::widgets::label::Displayable;
use cushy::widgets::select::SelectedColor;
use crate::context::Context;

pub trait Tab {
    fn label(&self, context: &Dynamic<Context>) -> String;
    fn make_content(&self, context: &Dynamic<Context>, tab_key: TabKey) -> WidgetInstance;
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

// Needs `Clone` so that the `on_click` close button handler can access all the state.
#[derive(Clone)]
pub struct TabBar<TK> {
    /// holds the actual tab instances and activation state which includes the tab's content widget instance
    tabs: Dynamic<SlotMap<TabKey, (TK, Dynamic<TabState>, Dynamic<String>)>>,
    /// tab bar buttons
    tab_items: Dynamic<WidgetList>,
    /// maintains an orders list of TabKeys, used when removing tabs from `tab_items` property.
    tab_items_keys: Dynamic<Vec<TabKey>>,
    /// the active tab's content area instance
    content_area: Dynamic<WidgetInstance>,
    /// the active tab's key, `None` when there are no tabs.
    active: Dynamic<Option<TabKey>>,
    /// this is used to track the switching out of the content area and updating the tab state.
    previous_tab: Dynamic<Option<TabKey>>,
    /// tracks the most recently used tab, used when closing a tab.
    history: Dynamic<Vec<TabKey>>,
}

new_key_type! {
    pub struct TabKey;
}

impl<TK: Tab + Send + Clone + 'static> TabBar<TK> {
    pub fn new() -> Self {
        let tabs: SlotMap<TabKey, (TK, Dynamic<TabState>, Dynamic<String>)> = Default::default();
        let content_area = Dynamic::new(Space::clear().make_widget());

        Self {
            tabs: Dynamic::new(tabs),
            content_area,
            tab_items: Dynamic::new(WidgetList::new()),
            tab_items_keys: Dynamic::new(Vec::new()),
            active: Dynamic::new(None),
            previous_tab: Dynamic::new(None),
            history: Dynamic::new(Vec::new()),
        }
    }

    pub fn replace(&mut self, tab_key: TabKey, context: &Dynamic<Context>, tab: TK) {

        let tab_label = tab.label(context);
        let tab_content_widget = tab.make_content(context, tab_key).make_widget();

        match self.active.get() {
            Some(active_tab_key) if active_tab_key.eq(&tab_key) => {
                self.content_area.replace(tab_content_widget);
                let mut tabs = self.tabs.lock();
                let (existing_tab, state, label) = tabs.get_mut(tab_key).unwrap();
                *existing_tab = tab;
                state.replace(TabState::Active);
                label.set(tab_label);
            }
            _ => {
                let mut tabs = self.tabs.lock();
                let (existing_tab, state, label) = tabs.get_mut(tab_key).unwrap();
                *existing_tab = tab;
                state.replace(TabState::Hidden(tab_content_widget));
                label.set(tab_label);
            }
        }
    }

    pub fn add_tab(&mut self, context: &Dynamic<Context>, tab: TK) -> TabKey {
        let tab_state = Dynamic::new(TabState::Uninitialized);

        let tab_label = tab.label(context);

        let tab_key = self.tabs.lock().insert((tab, tab_state, Dynamic::new(tab_label)));
        println!("tab_key: {:?}", tab_key);

        let tabs = self.tabs.lock();
        let (tab, tab_state, tab_label) = tabs.get(tab_key).unwrap();

        let tab_content = tab.make_content(context, tab_key);

        tab_state.set(TabState::Hidden(tab_content));

        let close_button = "X".into_button()
            .on_click({
                let tab_bar = Dynamic::new(self.clone());
                move |_event|{
                    tab_bar.lock().close_tab(tab_key);
                }
            })
            .with(&ButtonForeground, Color::LIGHTGRAY)
            .with(&ButtonBackground, Color::CLEAR_BLACK)
            .with(&ButtonActiveBackground, Color::CLEAR_BLACK)
            .with(&ButtonActiveForeground, Color::RED)
            .with(&ButtonHoverForeground, Color::RED);

        let select_content = tab_label.clone()
            .into_label()
            .and(close_button)
            .into_columns()
            .gutter(Px::new(5))
            .pad_by(Edges::default().with_horizontal(Px::new(3)).with_top(Px::new(3)).with_bottom(Px::new(0)))
            .and(
                Space::default()
                    .height(Px::new(3))
                    .with(&WidgetBackground, self.active.map_each(move |active|{
                        let mut color = Color::CLEAR_BLACK;
                        if let Some(active) = active {
                            if active.eq(&tab_key) {
                                color = Color::SKYBLUE
                            }
                        }
                        color
                    }))
            )
            .into_rows()
            .gutter(Px::new(0));

        let select = self.active
            .new_select(Some(tab_key), select_content )
            // NOTE any less than 3 here breaks the keyboard focus for the select button, 0 = not visible, < 3 = too small
            .with(&IntrinsicPadding, Px::new(3))
            .with(&ButtonForeground, Color::LIGHTGRAY)
            .with(&ButtonHoverForeground, Color::WHITE)
            .with(&ButtonActiveBackground, Color::GRAY)
            .with(&SelectedColor, Color::GRAY)
            // TODO remove this workaround for the select button's background inheritance
            .with(&WidgetBackground, Color::CLEAR_BLACK);

        self.tab_items.lock().push(select);
        self.tab_items_keys.lock().push(tab_key);

        self.history.lock().push(tab_key);

        // manually drop the guard before activation
        drop(tabs);

        self.activate(tab_key);

        tab_key
    }

    pub fn close_all(&mut self) {
        self.active.set(None);
        self.tab_items.lock().clear();
        self.tab_items_keys.lock().clear();
        self.tabs.lock().clear();
        self.history.lock().clear();
    }

    pub fn close_tab(&mut self, tab_key: TabKey) {

        println!("closing tab. tab_key: {:?}", tab_key);

        let mut history = self.history.lock();
        history.retain(|&other_key| other_key != tab_key);
        history.dedup();
        let recent = history.pop();
        // drop the history guard now so we don't deadlock in other methods we call that use history
        drop(history);

        if let Some(recent_key) = recent {
            self.activate(recent_key);
        } else {
            let _previously_active = self.active.take();
        }

        let mut tab_items_keys = self.tab_items_keys.lock();
        let tab_key_index = tab_items_keys.iter().enumerate().find_map(|(i, key)| {
            if *key == tab_key {
                Some(i)
            } else {
                None
            }
        }).unwrap();

        println!("tab_key_index: {:?}", tab_key_index);

        let widgets = self.tab_items
            .take();
        let new_widgets = WidgetList::from_iter(
            widgets
                .iter()
                .zip(tab_items_keys.iter())
                .filter_map(|( widget, index_tab_key)|{
                    println!("index_tab_key: {:?}", index_tab_key);
                    if index_tab_key.eq(&tab_key) {
                        println!("removing");
                        None
                    } else {
                        Some(widget.clone())
                    }
                })
        );
        tab_items_keys.remove(tab_key_index);

        self.tab_items.replace(new_widgets);

        self.tabs.lock().remove(tab_key).expect(format!("should be able to remove tab. key: {:?}", tab_key).as_str());
    }

    pub fn make_widget(&self) -> WidgetInstance {

        println!("TabBar::make_widget");

        let callback = self.active.for_each({
            let tabs = self.tabs.clone();
            let content_area = self.content_area.clone();
            let previous_tab_key = self.previous_tab.clone();

            move |selected_tab_key|{

                println!("key: {:?}, previous_tab_key: {:?}", selected_tab_key, previous_tab_key.get());

                if let Some(tab_key) = selected_tab_key.clone() {
                    let mut tabs_binding = tabs.lock();
                    let previous_tab = match tabs_binding.get_mut(tab_key) {
                        Some((_tab, tab_state, _tab_label)) => {
                            let tab_state_value = tab_state.take();

                            match tab_state_value {
                                TabState::Hidden(content_widget) => {
                                    let previous_content_widget = content_area.replace(content_widget);

                                    let result = match previous_tab_key.lock().take() {
                                        Some(previous_tab_key) => Some((previous_tab_key, previous_content_widget)),
                                        None => None,
                                    };

                                    previous_tab_key.lock().replace(tab_key);

                                    tab_state.set(TabState::Active);

                                    result
                                }
                                TabState::Active => None,
                                // actually reachable, occurs when closing tabs.
                                TabState::Uninitialized => None,
                            }
                        }
                        _ => None
                    };

                    match previous_tab {
                        Some((tab_key, Some(widget))) => {
                            if let Some((_tab, tab_state, _tab_label)) = tabs_binding.get_mut(tab_key) {
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
        let _previously_active = self.active.lock().replace(tab_key);
        let mut history = self.history.lock();
        history.push(tab_key);
        history.dedup();
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

impl<'a, TK: Clone> Iterator for TabsIter<'a, TK> {
    type Item = (TabKey, TK);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];

            let binding = self.tab_bar.tabs.lock();
            let value = binding
                .get(key)
                .map(|(tab, _state, _tab_label) | (key, tab.clone()) );

            self.index += 1;

            value

        } else {
            None
        }
    }
}

impl<'a, TK: Clone> IntoIterator for &'a TabBar<TK> {
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