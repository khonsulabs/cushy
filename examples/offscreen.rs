use cushy::figures::Size;
use cushy::widget::MakeWidget;
use cushy::window::VirtualRecorderError;

#[macro_use]
mod shared;

fn ui() -> impl MakeWidget {
    "Hello World".into_button().centered()
}

fn main() -> Result<(), VirtualRecorderError> {
    // The default recorder generated solid, rgb images.
    let recorder = ui().build_recorder().size(Size::new(320, 240)).finish()?;
    recorder.image().save("examples/offscreen.png").unwrap();

    // Creating a recorder with alpha makes the virtual window transparent.
    let recorder = ui()
        .build_recorder()
        .with_alpha()
        .size(Size::new(320, 240))
        .finish()?;
    recorder.image().save("examples/offscreen.png").unwrap();
    Ok(())
}

adapter_required_test!(main);
