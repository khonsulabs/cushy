use gooey::{
    core::Gooey,
    frontends::{
        rasterizer::Rasterizer,
        renderers::kludgine::{kludgine::prelude::*, Kludgine},
    },
    widgets::button::Button,
};

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
    fn render(&mut self, scene: &Target) -> KludgineResult<()> {
        self.ui.render(&Kludgine::from(scene));
        Ok(())
    }
}
