use gooey::widget::{MakeWidget, MakeWidgetWithId, WidgetTag};
use gooey::widgets::layers::OverlayLayer;
use gooey::Run;

fn main() -> gooey::Result {
    let overlay = OverlayLayer::default();

    test_widget(&overlay)
        .centered()
        .and(overlay)
        .into_layers()
        .run()
}

fn test_widget(overlay: &OverlayLayer) -> impl MakeWidget {
    let (my_tag, my_id) = WidgetTag::new();
    let right = "Right".into_button().on_click({
        let overlay = overlay.clone();
        move |()| {
            overlay
                .build_overlay(test_widget(&overlay))
                .right_of(my_id)
                .show()
                .forget();
        }
    });
    let left = "Left".into_button().on_click({
        let overlay = overlay.clone();
        move |()| {
            overlay
                .build_overlay(test_widget(&overlay))
                .left_of(my_id)
                .show()
                .forget();
        }
    });
    let up = "Up".into_button().on_click({
        let overlay = overlay.clone();
        move |()| {
            overlay
                .build_overlay(test_widget(&overlay))
                .above(my_id)
                .show()
                .forget();
        }
    });
    let down = "Down".into_button().on_click({
        let overlay = overlay.clone();
        move |()| {
            overlay
                .build_overlay(test_widget(&overlay))
                .below(my_id)
                .show()
                .forget();
        }
    });

    up.centered()
        .and(left.and(right).into_columns())
        .and(down.centered())
        .into_rows()
        .contain()
        .pad()
        .make_with_id(my_tag)
}
