use gooey_core::Gooey;
use gooey_kludgine::Kludgine;
use gooey_widgets::button::Button;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(GooeyExample {
        ui: Kludgine::new(Gooey::new(Button {
            label: String::from("Hello"),
            disabled: false,
        })),
    });
}

struct GooeyExample {
    ui: Kludgine,
}

impl WindowCreator for GooeyExample {
    fn window_title() -> String {
        "Gooey - Kludgine".to_owned()
    }
}

#[async_trait]
impl Window for GooeyExample {
    async fn update(&mut self, _scene: &Target, window: &OpenWindow<Self>) -> KludgineResult<()> {
        if self.ui.update() {
            window.set_needs_redraw().await;
        }
        Ok(())
    }
    async fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        self.ui.render(scene).await;
        Ok(())
    }
}
