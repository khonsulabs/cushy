use cushy::value::Source;
use crate::Dynamic;
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use crate::config::Config;
use crate::context::Context;
use crate::task::Task;
use crate::widgets::tab_bar::{Tab, TabKey};

#[derive(Clone, PartialEq)]
pub enum HomeTabMessage {
    None,
}

impl Default for HomeTabMessage {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Default)]
pub struct HomeTab {}

impl Tab<HomeTabMessage> for HomeTab {
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

            let home_label = "Home tab content"
                // FIXME remove this alignment, currently labels are center aligned by default.
                .align_left()
                .make_widget();

            let show_on_startup_button= "Show on startup"
                .into_checkbox(show_on_startup_value)
                .make_widget();

            [home_label, show_on_startup_button]
                .into_rows()
                // center all the children, not individually
                .centered()
                .make_widget()

        }).unwrap()
    }

    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: HomeTabMessage) -> Task<HomeTabMessage> {
        todo!()
    }
}