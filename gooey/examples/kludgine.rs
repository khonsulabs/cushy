use gooey::{
    core::Gooey,
    frontends::{rasterizer::Rasterizer, renderers::kludgine::Kludgine},
};
use gooey_widgets::button::Button;
use kludgine::prelude::*;

fn main() {
    SingleWindowApplication::run(GooeyExample {
        ui: Rasterizer::<Kludgine>::new(Gooey::new(Button {
            label: String::from("Hello"),
            disabled: false,
        })),
    });
}

struct GooeyExample {
    ui: Rasterizer<Kludgine>,
}

impl WindowCreator for GooeyExample {
    fn window_title() -> String {
        "Gooey - Kludgine".to_owned()
    }
}

impl Window for GooeyExample {
    fn update(&mut self, _scene: &Target, status: &mut RedrawStatus) -> KludgineResult<()> {
        if self.ui.update() {
            status.set_needs_redraw();
        }
        Ok(())
    }

    fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        self.ui.render(&Kludgine::from(scene));
        Ok(())
    }
}
