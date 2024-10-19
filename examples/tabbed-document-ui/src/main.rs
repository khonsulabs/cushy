use cushy::figures::units::Px;
use cushy::Run;
use cushy::value::{Dynamic};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use crate::tabs::TabKind;
use crate::widgets::tab_bar::TabBar;

mod widgets;

mod tabs {
    use cushy::widget::{MakeWidget, WidgetInstance};
    use crate::widgets::tab_bar::Tab;

    #[derive(Hash, PartialEq, Eq, Clone)]
    pub enum TabKind {
        Home,
        Document,
    }

    impl Tab for TabKind {
        fn label(&self) -> String {
            match self {
                TabKind::Home => "Home".to_string(),
                TabKind::Document => "Document".to_string(),
            }
        }

        fn make_content(&self) -> WidgetInstance {
            match self {
                TabKind::Home => "Home tab content".make_widget(),
                TabKind::Document => "Document tab content".make_widget(),
            }
        }
    }
}

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>
}

fn main() -> cushy::Result {

    let tab_bar = Dynamic::new(make_tab_bar());
    let toolbar = make_toolbar(tab_bar.clone());

    let app_state = AppState {
        tab_bar: tab_bar.clone()
    };

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(),
    ];

    let ui = ui_elements
        .into_rows()
        .width(Px::new(1024))
        .height(Px::new(768));

    ui.run()
}

fn make_tab_bar() -> TabBar<TabKind> {
    TabBar::new()
}

fn make_toolbar(tab_bar: Dynamic<TabBar<TabKind>>) -> Stack {
    let home_button = "Home"
        .into_button()
        .on_click({
            let tab_bar = tab_bar.clone();
            move |_|{
                println!("home clicked");

                tab_bar.lock().add_tab(TabKind::Home);
            }
        });

    let new_button = "New"
        .into_button()
        .on_click({
            let tab_bar = tab_bar.clone();
            move |_|{
                println!("New clicked");

                tab_bar.lock().add_tab(TabKind::Document);
            }
        });

    let open_button = "Open"
        .into_button();

    let close_all_button = "Close all"
        .into_button();

    let toolbar_widgets: [WidgetInstance; 5] = [
        home_button.make_widget(),
        new_button.make_widget(),
        open_button.make_widget(),
        close_all_button.make_widget(),
        Expand::empty().make_widget(),
    ];

    let toolbar = toolbar_widgets.into_columns();
    toolbar
}
