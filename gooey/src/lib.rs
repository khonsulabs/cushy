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
    use crate::{
        core::{Frontend, Gooey, Transmogrifiers, Widget, WidgetStorage},
        frontends::{
            rasterizer::{events::InputEvent as GooeyInputEvent, Rasterizer},
            renderers::kludgine::{kludgine::prelude::*, Kludgine},
        },
        widgets::rasterized::{default_transmogrifiers, register_transmogrifiers},
    };

    pub fn kludgine_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(
        mut transmogrifiers: Transmogrifiers<Rasterizer<Kludgine>>,
        initializer: C,
    ) {
        register_transmogrifiers(&mut transmogrifiers);
        let ui = Gooey::with(transmogrifiers, initializer);
        let ui = Rasterizer::<Kludgine>::new(ui);
        ui.process_widget_messages();

        SingleWindowApplication::run(GooeyWindow { ui });
    }

    pub fn kludgine_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) {
        kludgine_main_with(default_transmogrifiers(), initializer)
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

        fn process_input(
            &mut self,
            input: InputEvent,
            status: &mut RedrawStatus,
        ) -> KludgineResult<()> {
            let input = match input.event {
                Event::Keyboard {
                    scancode,
                    key,
                    state,
                } => GooeyInputEvent::Keyboard {
                    scancode,
                    key,
                    state,
                },
                Event::MouseButton { button, state } =>
                    GooeyInputEvent::MouseButton { button, state },
                Event::MouseMoved { position } => GooeyInputEvent::MouseMoved { position },
                Event::MouseWheel { delta, touch_phase } =>
                    GooeyInputEvent::MouseWheel { delta, touch_phase },
            };
            let result = self
                .ui
                .handle_event(gooey_rasterizer::events::WindowEvent::Input(input));
            self.ui.process_widget_messages();
            if result.needs_redraw || self.ui.needs_redraw() {
                status.set_needs_redraw();
            }
            Ok(())
        }
    }
}
#[cfg(feature = "frontend-kludgine")]
pub use kludgine::{kludgine_main, kludgine_main_with};

#[cfg(feature = "frontend-browser")]
mod browser {
    use crate::{
        core::{Frontend, Gooey, Transmogrifiers, Widget, WidgetStorage},
        frontends::browser::WebSys,
        widgets::browser::{default_transmogrifiers, register_transmogrifiers},
    };

    pub fn browser_main_with<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(
        mut transmogrifiers: Transmogrifiers<WebSys>,
        initializer: C,
    ) {
        register_transmogrifiers(&mut transmogrifiers);
        let ui = WebSys::new(Gooey::with(transmogrifiers, initializer));
        ui.process_widget_messages();
        ui.install_in_id("gooey")
    }

    pub fn browser_main<W: Widget + Send + Sync, C: FnOnce(&WidgetStorage) -> W>(initializer: C) {
        browser_main_with(default_transmogrifiers(), initializer)
    }
}

#[cfg(feature = "frontend-browser")]
pub use browser::{browser_main, browser_main_with};

cfg_if! {
    if #[cfg(feature = "frontend-browser")] {
        pub use browser_main as main;
        pub use browser_main_with as main_with;
        pub type ActiveFrontend = gooey_browser::WebSys;
    } else if #[cfg(feature = "frontend-kludgine")] {
        pub use kludgine_main as main;
        pub use kludgine_main_with as main_with;
        pub type ActiveFrontend = gooey_rasterizer::Rasterizer<gooey_kludgine::Kludgine>;
    }
}
