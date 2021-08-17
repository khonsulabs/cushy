use gooey::{core::DefaultWidget, App};
use gooey_widgets::navigator::Navigator;

mod widget_explorer_screens;

use widget_explorer_screens::{borders, input, main_menu::MainMenu, navigator, InfoPage, Page};

#[cfg(test)]
mod harness;

fn app() -> App {
    App::from_root(|storage| Navigator::<Page>::default_for(storage))
        .with_navigator::<Page>()
        .with_component::<InfoPage>()
        .with_component::<MainMenu>()
        .with_component::<navigator::Demo>()
        .with_component::<borders::Demo>()
        .with_component::<input::Demo>()
}

fn main() {
    app().run();
}
