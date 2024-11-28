use std::cell::RefCell;
use std::default::Default;
use std::hash::Hash;
use std::marker::PhantomData;
use slotmap::{new_key_type, SlotMap};
use cushy::define_components;
use cushy::figures::units::Px;
use cushy::styles::{Color, ContainerLevel, Edges};
use cushy::styles::components::{ErrorColor, HighlightColor, IntrinsicPadding, OpaqueWidgetColor, WidgetBackground};
use cushy::value::{Destination, Dynamic, Source, Switchable};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::grid::Orientation;
use cushy::widgets::{Expand, Space, Stack};
use cushy::widgets::button::{ButtonActiveBackground, ButtonActiveForeground, ButtonBackground, ButtonForeground, ButtonHoverBackground, ButtonHoverForeground};
use cushy::widgets::label::{Displayable, LabelOverflow};
use crate::action::Action;
use crate::context::Context;

#[derive(Clone, Debug)]
pub enum TabMessage<TKM> {
    None,
    CloseTab(TabKey),
    TabKindMessage(TabKey, TKM),
}

impl<TKM> Default for TabMessage<TKM> {
    fn default() -> Self {
        TabMessage::None
    }
}

pub enum TabAction<TKA, TK> {
    TabSelected(TabKey),
    TabClosed(TabKey, TK),
    TabAction(TabKey, TKA),
    None,
}

pub trait Tab<TKM, TKA> {
    fn label(&self, context: &Dynamic<Context>) -> String;
    fn make_content(&self, context: &Dynamic<Context>, tab_key: TabKey) -> WidgetInstance;
    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: TKM) -> Action<TKA>;
}

struct TabState<TK> {
    tab: TK,
    widget: Dynamic<WidgetInstance>,
    label: Dynamic<String>,
}

pub struct TabBar<TK, TKM, TKA>
{
    /// holds the actual tab instances and activation state which includes the tab's content widget instance
    tabs: Dynamic<SlotMap<TabKey, TabState<TK>>>,
    /// tab bar buttons
    tab_buttons: Dynamic<WidgetList>,
    /// maintains an orders list of TabKeys, used when removing tabs from `tab_items` property.
    tab_button_keys: Vec<TabKey>,
    /// the active tab's content area switcher instance
    content_switcher: Dynamic<WidgetInstance>,
    /// the active tab's key, `None` when there are no tabs.
    active: Dynamic<Option<TabKey>>,
    /// tracks the most recently used tab, used when closing a tab.
    history: RefCell<Vec<TabKey>>,

    /// a message which is updated when interactions occur.
    message: Dynamic<TabMessage<TKM>>,
    _action: PhantomData<TKA>
}

new_key_type! {
    pub struct TabKey;
}

impl<TK: Tab<TKM, TKA> + Send + Clone + 'static, TKM: Send + 'static, TKA> TabBar<TK, TKM, TKA> {
    pub fn new(message: &Dynamic<TabMessage<TKM>>) -> Self {
        let tabs: Dynamic<SlotMap<TabKey, TabState<TK>>> = Dynamic::default();
        let active: Dynamic<Option<TabKey>> = Dynamic::new(None);
        let switcher = active.clone().switcher({
            let tabs = tabs.clone();
            move |tab_key, _|{
                match tab_key {
                    None => {
                        println!("switcher changed, no tabs");
                        Space::clear().make_widget()
                    }
                    Some(tab_key) => {
                        println!("switcher changed, tab_key: {:?}", tab_key);
                        let tabs = tabs.lock();
                        let state = tabs.get(*tab_key).unwrap();

                        state.widget.get()
                    }
                }
            }
        });

        Self {
            tabs,
            tab_buttons: Dynamic::new(WidgetList::new()),
            tab_button_keys: Vec::new(),
            content_switcher: Dynamic::new(switcher.make_widget()),
            active,
            history: RefCell::new(Vec::new()),
            message: message.clone(),
            _action: Default::default(),
        }
    }

    pub fn replace(&mut self, tab_key: TabKey, context: &Dynamic<Context>, tab: TK) {
        let mut tabs = self.tabs.lock();
        let tab_state = tabs.get_mut(tab_key).unwrap();

        let tab_content_widget = tab.make_content(context, tab_key).make_widget();
        let tab_label = tab.label(context);

        tab_state.tab = tab;
        tab_state.widget.set(tab_content_widget);
        tab_state.label.set(tab_label);

        // prevent deadlock in the switcher closure
        drop(tabs);

        match self.active.get() {
            Some(active_tab_key) if active_tab_key.eq(&tab_key) => {
                // the tab key is still the same, so it is required to remove and set the active tab
                // to force the switcher to update the content area.
                // importantly, this doesn't break the tab ordering or tab history.
                self.active.take();
                self.active.set(Some(tab_key));
            }
            _ => (),
        }
    }

    pub fn add_tab(&mut self, context: &Dynamic<Context>, tab: TK) -> TabKey
    {

        let tab_key = self.tabs.lock().insert_with_key(|tab_key| {
            let tab_label = tab.label(context);

            let tab_content = tab.make_content(context, tab_key);

            let tab_state = TabState {
                tab,
                label: Dynamic::new(tab_label),
                widget: Dynamic::new(tab_content),
            };

            tab_state
        });

        println!("tab_key: {:?}", tab_key);

        let tabs = self.tabs.lock();
        let tab_state = tabs.get(tab_key).unwrap();

        let close_button = "X".into_button()
            .on_click({
                let message = self.message.clone();
                move |_event| message.force_set(TabMessage::CloseTab(tab_key))
            })
            .with(&ButtonBackground, Color::CLEAR_BLACK)
            .with(&ButtonActiveBackground, Color::CLEAR_BLACK)
            .with(&ButtonHoverBackground, Color::CLEAR_BLACK)
            .with_dynamic(&ButtonForeground, OpaqueWidgetColor)
            .with_dynamic(&ButtonActiveForeground, ErrorColor)
            .with_dynamic(&ButtonHoverForeground, ErrorColor);

        let select_content = tab_state.label.clone()
            .into_label()
            .overflow(LabelOverflow::Clip)
            .centered()
            .and(close_button)
            .into_columns()
            .gutter(Px::new(5))
            .pad_by(Edges::default().with_horizontal(Px::new(3)).with_top(Px::new(3)).with_bottom(Px::new(0)))
            .and(
                self.active.clone().switcher(move |active, _|{
                   match active {
                       Some(active_tab_key) if active_tab_key.eq(&tab_key) => {
                           Space::default()
                               .height(Px::new(3))
                               .with_dynamic(&WidgetBackground, TabBarActiveTabMarker)
                               .make_widget()

                       }
                       _ => {
                           Space::default()
                               .height(Px::new(3))
                               .make_widget()
                       }
                   }
                })
            )
            .into_rows()
            .gutter(Px::new(0));

        let select = self.active
            .new_select(Some(tab_key), select_content)
            // NOTE any less than 3 here breaks the keyboard focus for the select button, 0 = not visible, < 3 = too small
            .with(&IntrinsicPadding, Px::new(3))
            // TODO remove this workaround for the select button's background inheritance
            .with(&WidgetBackground, Color::CLEAR_BLACK);

        self.tab_buttons.lock().push(select);
        self.tab_button_keys.push(tab_key);

        self.history.borrow_mut().push(tab_key);

        // manually drop the guard before activation
        drop(tabs);

        self.activate(tab_key);

        tab_key
    }

    pub fn find_tab_by_label(&self, label: &str) -> Option<TabKey> {
        let tabs = self.tabs.lock();

        tabs.iter().find_map(|(tab_key, tab_state)| {
            if tab_state.label.get().eq(label) {
                Some(tab_key)
            } else {
                None
            }
        })
    }


    pub fn close_all(&mut self) -> Vec<(TabKey, TK)> {
        self.active.set(None);
        self.tab_buttons.lock().clear();
        self.tab_button_keys.clear();

        let closed_tabs = self.tabs.lock().drain().map(|(key,state)|(key, state.tab)).collect();
        self.history.borrow_mut().clear();

        closed_tabs
    }

    pub fn close_tab(&mut self, tab_key: TabKey) -> TK {

        println!("closing tab. tab_key: {:?}", tab_key);

        let mut history = self.history.borrow_mut();
        println!("history (before): {:?}", history);
        history.retain(|&other_key| other_key != tab_key);
        history.dedup();
        let recent = history.pop();
        println!("history (after): {:?}, recent: {:?}", history, recent);
        // drop the history guard now so we don't deadlock in other methods we call that use history
        drop(history);

        if let Some(recent_key) = recent {
            self.activate(recent_key);
        } else {
            let _previously_active = self.active.take();
        }

        let tab_button_keys = &mut self.tab_button_keys;
        let tab_key_index = tab_button_keys.iter().enumerate().find_map(|(i, key)| {
            if *key == tab_key {
                Some(i)
            } else {
                None
            }
        }).unwrap();

        println!("tab_key_index: {:?}", tab_key_index);

        let widgets = self.tab_buttons
            .take();
        let new_widgets = WidgetList::from_iter(
            widgets
                .iter()
                .zip(tab_button_keys.iter())
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
        tab_button_keys.remove(tab_key_index);

        self.tab_buttons.replace(new_widgets);

        let tk = self.tabs.lock().remove(tab_key).expect(format!("should be able to remove tab. key: {:?}", tab_key).as_str());

        tk.tab
    }

    pub fn make_widget(&self) -> WidgetInstance {

        println!("TabBar::make_widget");

        let callback = self.active.for_each({
            let history = self.history.clone();
            move |selected_tab_key|{

                println!("key: {:?}", selected_tab_key);

                if let Some(tab_key) = selected_tab_key {
                    let mut history = history.borrow_mut();
                    history.push(tab_key.clone());
                    history.dedup();
                }
            }
        });
        callback.persist();

        let widget = TabBarWidget {
            tab_buttons: self.tab_buttons.clone(),
            content_switcher: self.content_switcher.clone()
        };

        widget.make_widget()
    }

    pub fn with_tabs<R, F>(&self, f: F) -> R
    where
        F: Fn(TabsIter<'_, TK, TKM, TKA>) -> R
    {
        let iter = self.into_iter();
        f(iter)
    }

    pub fn activate(&self, tab_key: TabKey) {
        let _previously_active = self.active.lock().replace(tab_key);
    }

    pub fn update(&mut self, context: &Dynamic<Context>, message: TabMessage<TKM>) -> Action<TabAction<TKA, TK>> {
        match message {
            TabMessage::None => Action::new(TabAction::None),
            TabMessage::CloseTab(tab_key) => {
                let tab = self.close_tab(tab_key);
                Action::new(TabAction::TabClosed(tab_key, tab))
            },
            TabMessage::TabKindMessage(tab_key, tab_kind_message) => {
                let mut guard = self.tabs.lock();
                let tab_state = guard.get_mut(tab_key).unwrap();
                let action = tab_state.tab
                    .update(context, tab_key, tab_kind_message)
                    .map(move |action|TabAction::TabAction(tab_key, action));

                action
            }
        }
    }
}

pub struct TabsIter<'a, TK, TKM, TKA> {
    tab_bar: &'a TabBar<TK, TKM, TKA>,
    keys: Vec<TabKey>,
    index: usize,
}

impl<'a, TK, TKM, TKA> TabsIter<'a, TK, TKM, TKA> {
    pub fn new(tab_bar: &'a TabBar<TK, TKM, TKA>) -> Self {
        let keys = tab_bar.tabs.lock().keys().collect();

        Self {
            tab_bar,
            keys,
            index: 0,
        }
    }
}

impl<'a, TK: Clone, TKM, TKA> Iterator for TabsIter<'a, TK, TKM, TKA> {
    type Item = (TabKey, TK);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() {
            let key = self.keys[self.index];

            let binding = self.tab_bar.tabs.lock();
            let value = binding
                .get(key)
                .map(|tab_state | (key, tab_state.tab.clone()) );

            self.index += 1;

            value

        } else {
            None
        }
    }
}

impl<'a, TK: Clone, TKM, TKA> IntoIterator for &'a TabBar<TK, TKM, TKA> {
    type Item = (TabKey, TK);
    type IntoIter = TabsIter<'a, TK, TKM, TKA>;

    fn into_iter(self) -> Self::IntoIter {
        TabsIter::new(
            self
        )
    }
}

// Intermediate widget, with only the things it needs, so that it's possible to call `make_widget` which consumes self.
struct TabBarWidget {
    tab_buttons: Dynamic<WidgetList>,
    content_switcher: Dynamic<WidgetInstance>,
}

impl MakeWidget for TabBarWidget {
    fn make_widget(self) -> WidgetInstance {
        let dyn_tab_buttons = self.tab_buttons.clone();

        let tab_bar_switcher = self.tab_buttons.switcher({

            move |tab_buttons, _|{
               if tab_buttons.is_empty() {
                   Space::clear().make_widget()
               } else {
                   let tab_bar = [
                       Stack::new(Orientation::Column, dyn_tab_buttons.clone())
                           .make_widget(),
                       Expand::empty()
                           .make_widget(),
                   ]
                       .into_columns()
                       .contain_level(ContainerLevel::High);

                   tab_bar.make_widget()
               }
            }
        });

        tab_bar_switcher
            .and(self.content_switcher.expand())
            .into_rows()
            .gutter(Px::new(3))
            .make_widget()
    }
}

define_components! {
    TabBar {
        /// The color of the active tab's marker.
        TabBarActiveTabMarker(Color, "active_tab_marker_color", @HighlightColor)
    }
}
