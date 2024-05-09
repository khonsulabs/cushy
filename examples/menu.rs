use cushy::widget::MakeWidget;
use cushy::widgets::layers::{OverlayLayer, Overlayable};
use cushy::widgets::menu::{Menu, MenuItem};
use cushy::Run;

#[derive(Clone, Copy, Debug)]
enum MenuOptions {
    First,
    Second,
    Third,
}

fn main() -> cushy::Result {
    let overlay = OverlayLayer::default();

    "Click Me"
        .into_button()
        .on_click({
            let overlay = overlay.clone();
            move |click| {
                if let Some(click) = click {
                    Menu::new()
                        .with(MenuItem::new(MenuOptions::First, "First"))
                        .with(MenuItem::new(MenuOptions::Second, "Second"))
                        .with(MenuItem::new(MenuOptions::Third, "Third"))
                        .on_selected(|selected| {
                            println!("Selected item: {selected:?}");
                        })
                        .overlay_in(&overlay)
                        .at(click.window_location)
                        .show();
                }
            }
        })
        .centered()
        .expand()
        .and(overlay)
        .into_layers()
        .run()
}
