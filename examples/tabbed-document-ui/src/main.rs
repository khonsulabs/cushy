use std::sync::{Arc, Mutex};
use cushy::figures::units::Px;
use cushy::Run;
use cushy::value::{Dynamic};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use crate::app_tabs::TabKind;
use crate::config::Config;
use crate::context::Context;

use crate::widgets::tab_bar::TabBar;

mod config;
mod widgets;
mod home;
mod global_context;
mod context;
mod app_tabs;

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>,
    config: Dynamic<Config>,
    context: Arc<Mutex<Context>>,
}

fn main() -> cushy::Result {

    let config = Dynamic::new(config::load());

    let mut context = Context::default();
    context.provide(config.clone());

    let tab_bar = Dynamic::new(make_tab_bar());
    let toolbar = make_toolbar(tab_bar.clone());

    let mut app_state = AppState {
        tab_bar: tab_bar.clone(),
        context: Arc::new(Mutex::new(context)),
        config,
    };

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(&mut app_state.context),
    ];

    let ui = ui_elements
        .into_rows()
        .width(Px::new(800)..)
        .height(Px::new(600)..)
        .fit_vertically()
        .fit_horizontally()
        .into_window()
        .on_close({
            let config = app_state.config.clone();
            move ||{
                let config = config.lock();
                println!("Saving config");
                config::save(&*config);
            }
        })
        .titled("Tabbed document UI");

    if app_state.config.lock().show_home_on_startup {
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
