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
    image::save_buffer_with_format(
        "examples/offscreen.png",
        recorder.bytes(),
        recorder.window.size().width.get(),
        recorder.window.size().height.get(),
        image::ColorType::Rgb8,
        image::ImageFormat::Png,
    )
    .unwrap();

    // Creating a recorder with alpha makes the virtual window transparent.
    let recorder = ui()
        .build_recorder()
        .with_alpha()
        .size(Size::new(320, 240))
        .finish()
        .unwrap();
    image::save_buffer_with_format(
        "examples/offscreen-transparent.png",
        recorder.bytes(),
        recorder.window.size().width.get(),
        recorder.window.size().height.get(),
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .unwrap();
}

#[test]
fn runs() {
    main();
}
