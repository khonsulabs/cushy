use cushy::widget::MakeWidget;
use figures::Size;

fn ui() -> impl MakeWidget {
    "Hello World".into_button().centered()
}

fn main() {
    // The default recorder generated solid, rgb images.
    let recorder = ui()
        .build_recorder()
        .size(Size::new(320, 240))
        .finish()
        .unwrap();
    recorder.image().save("examples/offscreen.png").unwrap();

    // Creating a recorder with alpha makes the virtual window transparent.
    let recorder = ui()
        .build_recorder()
        .with_alpha()
        .size(Size::new(320, 240))
        .finish()
        .unwrap();
    recorder.image().save("examples/offscreen.png").unwrap();
}

#[test]
fn runs() {
    main();
}
