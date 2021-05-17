use gooey::frontends::{
    rasterizer::Rasterizer,
    renderers::kludgine::{kludgine::prelude::*, Kludgine},
};

mod shared;

fn main() {
    SingleWindowApplication::run(GooeyExample {
        ui: Rasterizer::<Kludgine>::new(shared::ui()),
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
        self.ui.render(Kludgine::from(scene));
        Ok(())
    }
}
