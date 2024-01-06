use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use cushy::figures::units::Px;
use cushy::figures::Size;
use cushy::widget::MakeWidget;
use cushy::widgets::container::ContainerShadow;
use cushy::window::{Rgba8, VirtualRecorder, VirtualRecorderBuilder};

pub struct BookExample {
    name: &'static str,
    recorder: VirtualRecorderBuilder<Rgba8>,
}

fn target_dir() -> PathBuf {
    let target_dir = std::env::current_dir()
        .expect("missing current dir")
        .parent()
        .expect("missing guide folder")
        .join("src")
        .join("examples");
    assert!(
        target_dir.is_dir(),
        "current directory is not guide-examples"
    );

    target_dir
}

impl BookExample {
    pub fn new(name: &'static str, interface: impl MakeWidget) -> Self {
        Self {
            name,
            recorder: interface
                .contain()
                .shadow(ContainerShadow::drop(Px::new(16), Px::new(32)))
                .width(Px::new(750))
                .build_recorder()
                .with_alpha()
                .resize_to_fit()
                .size(Size::new(750, 432)),
        }
    }

    pub fn still_frame<Test>(self, test: Test)
    where
        Test: FnOnce(&mut VirtualRecorder<Rgba8>),
    {
        let mut recorder = self.recorder.finish().unwrap();

        let capture = std::env::var("CAPTURE").is_ok();
        let errored = std::panic::catch_unwind(AssertUnwindSafe(|| test(&mut recorder))).is_err();
        if errored || capture {
            let path = target_dir().join(format!("{}.png", self.name));
            recorder.image().save(&path).expect("error saving file");
            println!("Wrote {}", path.display());

            if errored {
                std::process::exit(-1);
            }
        }
    }

    // pub fn animated<Test>(self, test: Test)
    // where
    //     Test: FnOnce(&mut AnimationRecorder<'_, Rgb8>),
    // {
    // }
}

#[macro_export]
macro_rules! book_example {
    ($name:ident) => {
        guide_examples::BookExample::new(stringify!($name), $name())
    };
}
