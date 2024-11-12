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
    pub fn from_path(path: PathBuf) -> ImageDocument {

        Self {
            path,
        }
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