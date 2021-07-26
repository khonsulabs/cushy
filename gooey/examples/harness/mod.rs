#[cfg(test)]
use std::path::PathBuf;

use gooey::App;
use gooey_core::{StyledWidget, Transmogrifiers, Widget, WidgetStorage};

pub trait UserInterface {
    type Root: Widget;

    fn root_widget(storage: &WidgetStorage) -> StyledWidget<Self::Root>;

    #[allow(unused_variables)]
    fn transmogrifiers(transmogrifiers: &mut Transmogrifiers<gooey::ActiveFrontend>) {}

    fn run() {
        let mut transmogrifiers = Transmogrifiers::default();
        Self::transmogrifiers(&mut transmogrifiers);
        gooey::main_with(transmogrifiers, &|storage: &WidgetStorage| {
            Self::root_widget(storage)
        })
    }

    fn headless() -> gooey::Headless<gooey::ActiveFrontend> {
        let mut transmogrifiers = Transmogrifiers::default();
        Self::transmogrifiers(&mut transmogrifiers);
        App::new(
            &|storage: &WidgetStorage| Self::root_widget(storage),
            transmogrifiers,
        )
        .headless()
    }
}

/// Returns a path within the `target` directory. This function assumes the exe
/// running is an example.
#[cfg(test)]
pub fn snapshot_path(example: &str, name: &str) -> std::io::Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    let target_dir = exe_path
        .parent()
        .expect("examples dir")
        .parent()
        .expect("debug dir")
        .parent()
        .expect("target dir");

    let examples_dir = target_dir.join("snapshots").join(example);
    if !examples_dir.exists() {
        std::fs::create_dir_all(&examples_dir)?;
    }

    Ok(examples_dir.join(name))
}
