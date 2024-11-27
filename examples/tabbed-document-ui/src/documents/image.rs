use std::fs::File;
use crate::WidgetInstance;
use std::path::PathBuf;
use std::time::Duration;
use image::ImageReader;
use cushy::figures::Point;
use cushy::figures::units::Px;
use cushy::kludgine::{AnyTexture, LazyTexture};
use cushy::styles::Color;
use cushy::styles::components::{FocusColor, IntrinsicPadding, WidgetBackground};
use cushy::value::{Destination, Dynamic, Source, Switchable};
use cushy::widget::MakeWidget;
use cushy::widgets::{Container, Image, Space};
use cushy::widgets::button::{ButtonActiveBackground, ButtonActiveOutline, ButtonBackground, ButtonHoverBackground, ButtonHoverOutline, ButtonOutline};
use crate::action::Action;
use crate::widgets::side_bar::{SideBar, SideBarItem};

#[derive(Clone, Debug)]
pub enum ImageDocumentMessage {
    None,
    Load,
    Loaded(LazyTexture),
    Create,
    Clicked(Point<Px>),
}

impl Default for ImageDocumentMessage {
    fn default() -> Self {
        Self::None
    }
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
    message: Dynamic<ImageDocumentMessage>,
    last_clicked_location: Dynamic<Option<Point<Px>>>,
}

impl ImageDocument {
    fn new(path: PathBuf, message: Dynamic<ImageDocumentMessage>) -> ImageDocument {
        let mut side_bar = SideBar::default()
            .with_fixed_width_columns();

        let path_item = SideBarItem::new(
            "Path".to_string(),
            Dynamic::new(Some(path.to_str().unwrap().to_string()))
        );
        side_bar.push(path_item);

        let last_clicked_location = Dynamic::default();
        let last_clicked_location_item = SideBarItem::new(
            "Last clicked".to_string(),
            last_clicked_location
                .clone()
                .map_each(|&location: &Option<Point<Px>>|{
                    match location {
                        None => Some("None".to_string()),
                        Some(location) => {
                            Some(format!("x: {}, y: {}", location.x, location.y).to_string())
                        }
                    }
                })
        );
        side_bar.push(last_clicked_location_item);

        Self {
            path,
            texture: Dynamic::new(None),
            side_bar,
            message,
            last_clicked_location,
        }
    }

    pub fn from_path(path: PathBuf, message: Dynamic<ImageDocumentMessage>) -> (Self, ImageDocumentMessage) {
        (
            Self::new(path, message),
            ImageDocumentMessage::Load,
        )
    }

    pub fn create_new(path: PathBuf, message: Dynamic<ImageDocumentMessage>) -> (Self, ImageDocumentMessage) {
        (
            Self::new(path, message),
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

        let image_widget = self.texture.clone().switcher({
            let message = self.message.clone();
            move |texture, _|
                match texture {
                    None => Space::clear().make_widget(),
                    Some(texture) => {
                        // TODO investigate if we should really be cloning here...
                        let texture = AnyTexture::Lazy(texture.clone());

                        let image_widget = Image::new(texture)
                            // FIXME the button should be the same size as the image/texture
                            .into_button()
                            .on_click({
                                let message = message.clone();
                                move |event|{
                                    match event {
                                        None => {}
                                        Some(button_click) => {
                                            let location = button_click.location;
                                            message.force_set(ImageDocumentMessage::Clicked(location));
                                        }
                                    }
                                }
                            })
                            // FIXME Focus color is not being applied here.
                            .with(&FocusColor, Color::CLEAR_BLACK)
                            .with(&IntrinsicPadding, Px::new(0))
                            .with(&ButtonBackground, Color::CLEAR_BLACK)
                            .with(&ButtonActiveBackground, Color::CLEAR_BLACK)
                            .with(&ButtonHoverBackground, Color::CLEAR_BLACK)
                            .with(&ButtonOutline, Color::CLEAR_BLACK)
                            .with(&ButtonActiveOutline, Color::CLEAR_BLACK)
                            .with(&ButtonHoverOutline, Color::CLEAR_BLACK)
                            .make_widget();
                        image_widget
                    }
                }
        }
        )
            .with(&WidgetBackground, Color::BLUE)
            .make_widget();

        let image_container_widget = Container::new(image_widget)
            .background_color(Color::GREEN)
            .expand()
            .make_widget();

        let document_widgets = side_bar_widget
            .and(image_container_widget)
            .into_columns()
            .expand();

        document_widgets
            .make_widget()
    }

    pub fn update(&mut self, message: ImageDocumentMessage) -> Action<ImageDocumentAction> {
        let action = match message {
            ImageDocumentMessage::None => ImageDocumentAction::None,
            ImageDocumentMessage::Create => ImageDocumentAction::Create,
            ImageDocumentMessage::Load => ImageDocumentAction::Load,
            ImageDocumentMessage::Loaded(texture) => {
                self.texture.set(Some(texture));
                ImageDocumentAction::None
            }
            ImageDocumentMessage::Clicked(point) => {
                println!("image clicked, location: {:?}", point);
                self.last_clicked_location.set(Some(point));
                ImageDocumentAction::None
            }
        };

        Action::new(action)
    }
}