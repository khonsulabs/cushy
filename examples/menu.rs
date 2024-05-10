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
                    menu(true)
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

fn menu(top: bool) -> Menu<MenuOptions> {
    let mut third = MenuItem::build(MenuOptions::Third).text("Third");
    if top {
        third = third.submenu(menu(false));
    }
    Menu::new()
        .on_selected(|selected| {
            println!("Selected item: {selected:?}");
        })
        .with(MenuItem::new(MenuOptions::First, "First"))
        .with(MenuItem::new(MenuOptions::Second, "Second"))
        .with_separator()
        .with(
            MenuItem::build(MenuOptions::Second)
                .text("Disabled")
                .disabled(),
        )
        .with_separator()
        .with(third)
}
