use cushy::reactive::value::Source;
use crate::Dynamic;
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use crate::config::Config;
use crate::context::Context;

#[derive(Clone, Copy, Default)]
pub struct HomeTab {}

impl HomeTab {

    pub fn create_label(&self) -> String {
        "Home".to_string()
    }

    pub fn create_content(&self, context: &mut Context) -> WidgetInstance {

        context.with_context::<Dynamic<Config>, _, _>(|config|{
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
}