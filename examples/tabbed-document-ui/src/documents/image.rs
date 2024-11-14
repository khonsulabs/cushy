use std::fs::File;
use crate::WidgetInstance;
use std::path::PathBuf;
use image::ImageReader;
use cushy::kludgine::{AnyTexture, LazyTexture};
use cushy::widget::MakeWidget;
use cushy::widgets::Image;

pub struct ImageDocument {
    pub path: PathBuf,
}

impl ImageDocument {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,
        }
    }

    pub fn new(path: PathBuf) -> Self {

        let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(256, 256);

        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (0.3 * x as f32) as u8;
            let b = (0.3 * y as f32) as u8;
            *pixel = image::Rgb([r, 0, b]);
        }

        let mut file = File::create_new(path.clone()).unwrap();
        imgbuf.write_to(&mut file, image::ImageFormat::Png).expect("should write to file");

        Self::from_path(path)
    }


    pub fn create_content(&self) -> WidgetInstance {
        println!("ImageDocument::create_content. path: {:?}", self.path);

        let dyn_image = ImageReader::open(&self.path).unwrap().decode().unwrap();

        let texture = LazyTexture::from_image(
            dyn_image,
            cushy::kludgine::wgpu::FilterMode::Linear
        );
        let image_widget = Image::new(AnyTexture::Lazy(texture))
            .make_widget();

        image_widget
    }
}