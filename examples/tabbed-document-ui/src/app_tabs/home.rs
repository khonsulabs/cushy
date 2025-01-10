use cushy::localize;
use cushy::value::Source;
use crate::Dynamic;
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use crate::action::Action;
use crate::config::Config;
use crate::context::Context;
use crate::widgets::tab_bar::{Tab, TabKey};

#[derive(Clone, Debug)]
pub enum HomeTabMessage {
    None,
}

impl Default for HomeTabMessage {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug)]
pub enum HomeTabAction {
    None
}

#[derive(Clone, Default)]
pub struct HomeTab {}

impl Tab<HomeTabMessage, HomeTabAction> for HomeTab {
    fn label(&self, _context: &Dynamic<Context>) -> String {
        "Home".to_string()
    }

    fn make_content(&self, context: &Dynamic<Context>, _tab_key: TabKey) -> WidgetInstance {

        context.lock().with_context::<Dynamic<Config>, _, _>(|config|{
            let config_guard = config.lock();
            let show_on_startup_value = Dynamic::new(config_guard.show_home_on_startup);
            let callback = show_on_startup_value.for_each_cloned({
                let config_binding = config.clone();

                move |value|{
                    println!("updating config, show_home_on_startup: {}", value);
                    let mut config_guard = config_binding.lock();
                    config_guard.show_home_on_startup = value;
                }
            });

            callback.persist();

            let home_label = localize!("home-banner")
                .xxxx_large()
                .centered()
                .make_widget();

            let show_on_startup_button= localize!("home-checkbox-label-show-on-startup")
                .into_checkbox(show_on_startup_value)
                .centered()
                .make_widget();

            [home_label, show_on_startup_button]
                .into_rows()
                // center all the children, not individually
                .centered()
                .make_widget()

        }).unwrap()
    }

    fn update(&mut self, _context: &Dynamic<Context>, _tab_key: TabKey, message: HomeTabMessage) -> Action<HomeTabAction> {
        match message {
            HomeTabMessage::None => {}
        }
        Action::new(HomeTabAction::None)
    }
}