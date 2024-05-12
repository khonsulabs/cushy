use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use cushy::figures::units::Px;
use cushy::figures::Size;
use cushy::widget::MakeWidget;
use cushy::widgets::container::ContainerShadow;
use cushy::window::{AnimationRecorder, Rgba8, VirtualRecorder, VirtualRecorderBuilder};

pub struct ExampleBuilder {
    name: &'static str,
    recorder: VirtualRecorderBuilder<Rgba8>,
}

impl ExampleBuilder {
    #[must_use]
    pub fn finish(self) -> Example {
        Example {
            name: self.name,
            recorder: self.recorder.finish().expect("error creating recorder"),
        }
    }

    pub fn untested_still_frame(self) {
        self.finish().untested_still_frame();
    }

    pub fn prepare_with<Prepare>(self, prepare: Prepare) -> Example
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
    let current_dir = std::env::current_dir().expect("missing current dir");
    let mut target_dir = current_dir.join("guide").join("src").join("examples");
    if !target_dir.is_dir() {
        target_dir = current_dir
            .parent()
            .expect("missing guide folder")
            .join("src")
            .join("examples");
    }
    assert!(
        target_dir.is_dir(),
        "current directory is not guide-examples or the root directory"
    );

    target_dir
}

pub struct Example {
    name: &'static str,
    recorder: VirtualRecorder<Rgba8>,
}

impl Example {
    pub fn build(
        name: &'static str,
        interface: impl MakeWidget,
        width: u16,
        height: Option<u16>,
    ) -> ExampleBuilder {
        let mut contents = interface
            .contain()
            .shadow(ContainerShadow::drop(Px::new(16)))
            .width(Px::new(i32::from(width)));
        if let Some(height) = height {
            contents = contents.height(Px::new(i32::from(height)));
        }
        ExampleBuilder {
            name,
            recorder: contents
                .build_recorder()
                .with_alpha()
                .resize_to_fit()
                .size(Size::new(
                    u32::from(width),
                    u32::from(height.unwrap_or(432)),
                )),
        }
    }

    pub fn untested_still_frame(self) {
        self.still_frame(|_| {});
    }

    #[must_use]
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
macro_rules! example {
    ($name:ident) => {
        $crate::example!($name, 750)
    };
    ($name:ident, $width:expr) => {
        $crate::example::Example::build(stringify!($name), $name(), $width, None)
    };
    ($name:ident, $width:expr, $height:expr) => {
        $crate::example::Example::build(stringify!($name), $name(), $width, Some($height))
    };
}
