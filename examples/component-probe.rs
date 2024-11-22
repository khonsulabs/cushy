use cushy::styles::components::PrimaryColor;
use cushy::widget::MakeWidget;
use cushy::widgets::{ComponentProbe, Space};
use cushy::Run;
use kludgine::Color;

fn main() -> cushy::Result {
    let probe = ComponentProbe::new(PrimaryColor, Color::CLEAR_WHITE);

    Space::colored(probe.value().clone())
        .expand()
        .and(probe)
        .into_layers()
        .contain()
        .pad()
        .run()
}
