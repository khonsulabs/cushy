use std::fs::File;
use crate::WidgetInstance;
use std::path::PathBuf;
use std::time::Duration;
use image::ImageReader;
use cushy::figures::units::Px;
use cushy::kludgine::{AnyTexture, LazyTexture};
use cushy::value::{Destination, Dynamic, Switchable};
use cushy::widget::MakeWidget;
use cushy::widgets::{Image, Space};
use crate::action::Action;
use crate::widgets::side_bar::{SideBar, SideBarItem};

#[derive(Clone, Debug)]
pub enum ImageDocumentMessage {
    Load,
    Loaded(LazyTexture),
    Create,
}

#[derive(Debug)]
pub enum ImageDocumentAction {
    None,
    Create,
    Load,
}

#[derive(Debug)]
pub enum ImageDocumentError {
    ErrorCreatingImage(PathBuf),
    ErrorLoadingImage(PathBuf),
}

pub struct ImageDocument {
    pub path: PathBuf,
    texture: Dynamic<Option<LazyTexture>>,
    side_bar: SideBar,
}

impl ImageDocument {
    fn new(path: PathBuf) -> ImageDocument {
        let mut side_bar = SideBar::default();

        let path_item = SideBarItem::new("Path".to_string(), Some(path.to_str().unwrap().to_string()));
        side_bar.push(path_item);

        Self {
            path,
            texture: Dynamic::new(None),
            side_bar,
        }
    }

    pub fn from_path(path: PathBuf) -> (Self, ImageDocumentMessage) {
        (
            Self::new(path),
            ImageDocumentMessage::Load,
        )
    }

    pub fn create_new(path: PathBuf) -> (Self, ImageDocumentMessage) {
        (
            Self::new(path),
            ImageDocumentMessage::Create,
        )
    }

    pub async fn create(path: PathBuf) -> Result<(), ImageDocumentError> {
        println!("creating image document. path: {:?}", path);
        let mut imgbuf = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(256, 256);

        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (0.3 * x as f32) as u8;
            let b = (0.3 * y as f32) as u8;
            *pixel = image::Rgb([r, 0, b]);
        }

        let mut file = File::create_new(&path).unwrap();

        // TODO improve error handling by using '_error'
        imgbuf.write_to(&mut file, image::ImageFormat::Png)
            .map_err(|_error|ImageDocumentError::ErrorCreatingImage(path))
    }

    pub async fn load(path: PathBuf) -> Result<LazyTexture, ImageDocumentError> {
        println!("loading image document. path: {:?}", path);
        // TODO improve error handling by using '_error'
        let reader = ImageReader::open(&path)
            .map_err(|_error|ImageDocumentError::ErrorLoadingImage(path.clone()))?;

        let dyn_image = reader.decode()
            .map_err(|_error|ImageDocumentError::ErrorLoadingImage(path))?;

        let texture = LazyTexture::from_image(
            dyn_image,
            cushy::kludgine::wgpu::FilterMode::Linear
        );

        // Simulate slow loading
        async_std::task::sleep(Duration::from_millis(500)).await;

        Ok(texture)
    }

    pub fn create_content(&self) -> WidgetInstance {
        println!("ImageDocument::create_content. path: {:?}", self.path);

        let side_bar_widget = self.side_bar.make_widget();

        let image_widget = self.texture.clone().switcher(|texture, _|
            match texture {
                None => Space::clear().make_widget(),
                Some(texture) => {
                    // TODO investigate if we should really be cloning here...
                    let texture = AnyTexture::Lazy(texture.clone());

                    let image_widget = Image::new(texture)
                        .make_widget();
                    image_widget
                }
            }
        )
            .expand()
            .make_widget();

        let document_widgets = side_bar_widget
            .and(image_widget)
            .into_columns()
            .gutter(Px::new(0))
            .expand();

        document_widgets
            .make_widget()
    }

    pub fn update(&mut self, message: ImageDocumentMessage) -> Action<ImageDocumentAction> {
        let action = match message {
            ImageDocumentMessage::Create => ImageDocumentAction::Create,
            ImageDocumentMessage::Load => ImageDocumentAction::Load,
            ImageDocumentMessage::Loaded(texture) => {
                self.texture.set(Some(texture));
                ImageDocumentAction::None
            }
        };

        Action::new(action)
    }
}