use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetInstance, WidgetList};
use cushy::widgets::Space;
use cushy::Run;

#[derive(Default, PartialEq)]
struct TabBar {
    tabs: Vec<Tab>,
}

impl TabBar {
    pub fn add_tab(&mut self, tab: Tab) {
        self.tabs.push(tab);
    }
}

#[derive(PartialEq)]
enum Tab {
    Home,
    Text { title: String, contents: String },
}

impl Tab {
    pub fn label(&self) -> &str {
        match self {
            Tab::Home => "Home",
            Tab::Text { title, .. } => title,
        }
    }

    pub fn make_content(&self) -> WidgetInstance {
        println!("make_content");
        match self {
            Tab::Home => "This is the home tab".make_widget(),
            Tab::Text { contents, .. } => contents.make_widget(),
        }
    }
}

fn main() -> cushy::Result {
    let tabs = Dynamic::new(TabBar::default());
    let home = "Home".into_button().on_click({
        let tabs = tabs.clone();
        move |_| tabs.lock().add_tab(Tab::Home)
    });
    let second_button = "New Tab".into_button().on_click({
        let tabs = tabs.clone();
        let mut counter = 0;
        move |_| {
            counter += 1;
            tabs.lock().add_tab(Tab::Text {
                title: format!("Tab {counter}"),
                contents: format!("This is tab {counter}"),
            })
        }
    });

    // Create an empty area for the active tab to be displayed.
    let content_area = Dynamic::new(Space::clear().make_widget());

    home.and(second_button)
        .into_columns()
        .and(make_tab_bar(&tabs, &content_area))
        .and(content_area.expand())
        .into_rows()
        .expand()
        .run()
}

fn make_tab_bar(tabs: &Dynamic<TabBar>, content_area: &Dynamic<WidgetInstance>) -> impl MakeWidget {
    let content_area = content_area.clone();
    tabs.map_each(move |bar| {
        bar.tabs
            .iter()
            .map(|tab| {
                let content = tab.make_content();
                tab.label().into_button().on_click({
                    let content_area = content_area.clone();
                    move |_| content_area.set(content.clone())
                })
            })
            .collect::<WidgetList>()
    })
        .into_columns()
}