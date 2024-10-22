use std::sync::Arc;
use cushy::figures::units::Px;
use cushy::Run;
use cushy::value::{Dynamic};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use crate::config::Config;
use crate::tabs::TabKind;
use crate::widgets::tab_bar::TabBar;

mod config;
mod widgets;
mod home;
mod app_context;

mod tabs {
    use std::sync::Arc;
    use cushy::value::Dynamic;
    use cushy::widget::{MakeWidget, WidgetInstance};
    use crate::app_context::with_context;
    use crate::config::Config;
    use crate::home;
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
                TabKind::Home => {
                     with_context::<Arc<Config>, _, _>(|config|{
                        let show_on_startup_value = Dynamic::new(config.show_home_on_startup);
                        home::create_content(show_on_startup_value)
                    }).unwrap()
                },
                TabKind::Document => "Document tab content".make_widget(),
            }
        }
    }
}

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>,
    pub config: Arc<Config>,
}

fn main() -> cushy::Result {

    let config = Arc::new(config::load());

    app_context::provide_context(config.clone());

    let tab_bar = Dynamic::new(make_tab_bar());
    let toolbar = make_toolbar(tab_bar.clone());

    let app_state = AppState {
        tab_bar: tab_bar.clone(),
        config,
    };

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(),
    ];

    let ui = ui_elements
        .into_rows()
        .width(Px::new(1024))
        .height(Px::new(768))
        .into_window()
        .on_close({
            let config = app_state.config.clone();
            move ||{
                println!("Saving config");
                config::save(&*config);
            }
        });

    if app_state.config.show_home_on_startup {
        add_home_tab(&app_state.tab_bar);
    }

    let cushy_result = ui.run();

    // FIXME control never returns here (at least on windows)

    cushy_result
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

                add_home_tab(&tab_bar);
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
        .into_button()
        .on_click({
            let tab_bar = tab_bar.clone();
            move |_| {
                println!("close all clicked");

                tab_bar.lock().close_all();
            }
        });


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

fn add_home_tab(tab_bar: &Dynamic<TabBar<TabKind>>) {
    tab_bar.lock().add_tab(TabKind::Home);
}
