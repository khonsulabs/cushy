use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use cushy::figures::units::Px;
use cushy::figures::Size;
use cushy::widget::MakeWidget;
use cushy::widgets::container::ContainerShadow;
use cushy::window::{AnimationRecorder, Rgba8, VirtualRecorder, VirtualRecorderBuilder};

pub struct BookExampleBuilder {
    name: &'static str,
    recorder: VirtualRecorderBuilder<Rgba8>,
}

impl BookExampleBuilder {
    pub fn finish(self) -> BookExample {
        BookExample {
            name: self.name,
            recorder: self.recorder.finish().expect("error creating recorder"),
        }
    }

    pub fn untested_still_frame(self) {
        self.finish().untested_still_frame()
    }

    pub fn prepare_with<Prepare>(self, prepare: Prepare) -> BookExample
    where
        Prepare: FnOnce(&mut VirtualRecorder<Rgba8>),
    {
        self.finish().prepare_with(prepare)
    }

    pub fn still_frame<Test>(self, test: Test)
    where
        Test: FnOnce(&mut VirtualRecorder<Rgba8>),
    {
        self.finish().still_frame(test);
    }

    pub fn animated<Test>(self, test: Test)
    where
        Test: FnOnce(&mut AnimationRecorder<'_, Rgba8>),
    {
        self.finish().animated(test);
    }
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

pub struct BookExample {
    name: &'static str,
    recorder: VirtualRecorder<Rgba8>,
}

impl BookExample {
    pub fn build(name: &'static str, interface: impl MakeWidget) -> BookExampleBuilder {
        BookExampleBuilder {
            name,
            recorder: interface
                .contain()
                .shadow(ContainerShadow::drop(Px::new(16)))
                .width(Px::new(750))
                .build_recorder()
                .with_alpha()
                .resize_to_fit()
                .size(Size::new(750, 432)),
        }
    }

    pub fn untested_still_frame(self) {
        self.still_frame(|_| {});
    }

    pub fn prepare_with<Prepare>(mut self, prepare: Prepare) -> Self
    where
        Prepare: FnOnce(&mut VirtualRecorder<Rgba8>),
    {
        prepare(&mut self.recorder);
        self
    }

    pub fn still_frame<Test>(mut self, test: Test)
    where
        Test: FnOnce(&mut VirtualRecorder<Rgba8>),
    {
        let capture = std::env::var("CAPTURE").is_ok();
        let errored =
            std::panic::catch_unwind(AssertUnwindSafe(|| test(&mut self.recorder))).is_err();
        if errored || capture {
            let path = target_dir().join(format!("{}.png", self.name));
            self.recorder
                .image()
                .save(&path)
                .expect("error saving file");
            println!("Wrote {}", path.display());

            if errored {
                std::process::exit(-1);
            }
        }
    }

    pub fn animated<Test>(mut self, test: Test)
    where
        Test: FnOnce(&mut AnimationRecorder<'_, Rgba8>),
    {
        let mut animation = self.recorder.record_animated_png(60);
        let capture = std::env::var("CAPTURE").is_ok();
        let errored = std::panic::catch_unwind(AssertUnwindSafe(|| test(&mut animation))).is_err();
        if errored || capture {
            let path = target_dir().join(format!("{}.png", self.name));
            animation.write_to(&path).expect("error saving file");
            println!("Wrote {}", path.display());

            if errored {
                std::process::exit(-1);
            }
        }
    }
}

#[macro_export]
macro_rules! book_example {
    ($name:ident) => {
        guide_examples::BookExample::build(stringify!($name), $name())
    };
}
