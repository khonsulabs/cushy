pub mod frontends {

    #[cfg(feature = "frontend-browser")]
    #[doc(inline)]
    pub use gooey_browser as browser;
    #[cfg(feature = "gooey-rasterizer")]
    #[doc(inline)]
    pub use gooey_rasterizer as rasterizer;
    pub mod renderers {
        #[cfg(feature = "frontend-kludgine")]
        #[doc(inline)]
        pub use gooey_kludgine as kludgine;
    }
}
use cfg_if::cfg_if;
#[doc(inline)]
pub use gooey_core as core;
#[doc(inline)]
pub use gooey_widgets as widgets;

#[cfg(feature = "frontend-kludgine")]
mod kludgine {
    use gooey_core::{Transmogrifiers, Widget, WidgetStorage};
    use gooey_kludgine::kludgine::prelude::*;
    use gooey_widgets::rasterized::default_transmogrifiers;

    use crate::{
        core::Gooey,
        frontends::{rasterizer::Rasterizer, renderers::kludgine::Kludgine},
    };

    pub fn kludgine_main_with_transmogrifiers<
        W: Widget + Send + Sync,
        C: FnOnce(&WidgetStorage) -> W,
    >(
        mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
        initializer: C,
    ) {
        crate::widgets::rasterized::register_transmogrifiers(&mut transmogrifiers);
        let ui = Gooey::with(transmogrifiers, initializer);

        SingleWindowApplication::run(GooeyWindow {
            ui: Rasterizer::<Kludgine>::new(ui),
        });
    }

    pub fn kludgine_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) {
        kludgine_main_with_transmogrifiers(default_transmogrifiers(), initializer)
    }

    struct GooeyWindow {
        ui: Rasterizer<Kludgine>,
    }

    impl WindowCreator for GooeyWindow {
        fn window_title() -> String {
            "Gooey - Kludgine".to_owned()
        }
    }

    impl Window for GooeyWindow {
        fn render(&mut self, scene: &Target) -> KludgineResult<()> {
            self.ui.render(Kludgine::from(scene));
            Ok(())
        }
    }
}
#[cfg(feature = "frontend-kludgine")]
pub use kludgine::kludgine_main;

#[cfg(feature = "frontend-browser")]
mod browser {
    use crate::{
        core::{Gooey, Transmogrifiers, Widget, WidgetStorage},
        frontends::browser::WebSys,
        widgets::browser::{default_transmogrifiers, register_transmogrifiers},
    };

    pub fn browser_main_with_transmogrifiers<
        W: Widget + Send + Sync,
        C: FnOnce(&WidgetStorage) -> W,
    >(
        mut transmogrifiers: Transmogrifiers<WebSys>,
        initializer: C,
    ) {
        register_transmogrifiers(&mut transmogrifiers);
        WebSys::new(Gooey::with(transmogrifiers, initializer)).install_in_id("gooey")
    }

    pub fn browser_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) {
        browser_main_with_transmogrifiers(default_transmogrifiers(), initializer)
    }
}

#[cfg(feature = "frontend-browser")]
pub use browser::browser_main;

cfg_if! {
    if #[cfg(feature = "frontend-browser")] {
        pub use browser_main as main;
    } else if #[cfg(feature = "frontend-kludgine")] {
        pub use kludgine_main as main;
    }
}
