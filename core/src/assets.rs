#![allow(missing_docs)]

use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use image::{ImageFormat, RgbaImage};

use crate::{
    styles::{FontStyle, Weight},
    AnyFrontend, Callback,
};

#[derive(Debug, Clone)]
#[must_use]
pub struct Asset {
    data: Arc<Data>,
}

#[derive(Debug)]
struct Data {
    path: Vec<Cow<'static, str>>,
    sha1sum: Option<[u8; 20]>,
}

impl Asset {
    pub fn build() -> Builder {
        Builder::new()
    }

    #[must_use]
    pub fn path(&self) -> &[Cow<'static, str>] {
        &self.data.path
    }
}

#[must_use]
pub struct Builder {
    asset: Data,
}

impl Builder {
    fn new() -> Self {
        Self {
            asset: Data {
                sha1sum: None,
                path: Vec::new(),
            },
        }
    }

    pub fn path<S: Into<Cow<'static, str>>>(mut self, segments: Vec<S>) -> Self {
        self.asset.path = segments.into_iter().map(Into::into).collect();
        self
    }

    pub fn sha1sum(mut self, sha1sum: [u8; 20]) -> Self {
        self.asset.sha1sum = Some(sha1sum);
        self
    }

    pub fn finish(self) -> Asset {
        Asset {
            data: Arc::new(self.asset),
        }
    }
}

pub struct Font {
    pub family: String,
    pub weight: Weight,
    pub style: FontStyle,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Image {
    pub asset: Asset,
    data: Arc<Mutex<Option<Arc<RgbaImage>>>>,
}

impl Image {
    pub fn new(asset: Asset) -> Self {
        Self {
            asset,
            data: Arc::default(),
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn load(
        &self,
        on_loaded: Callback<Image>,
        on_error: Callback<String>,
        frontend: &dyn AnyFrontend,
    ) {
        let callback_image = self.clone();
        let callback_error = on_error.clone();
        frontend.load_asset(
            &self.asset,
            Callback::new(move |data: Vec<u8>| {
                if let Err(err) = callback_image.load_data(&data) {
                    callback_error.invoke(err);
                } else {
                    on_loaded.invoke(callback_image.clone());
                }
            }),
            on_error,
        );
    }

    fn load_data(&self, data: &[u8]) -> Result<(), String> {
        let mut image_data = self.data.lock().unwrap();
        let format = ImageFormat::from_path(self.asset.data.path.last().unwrap().as_ref())
            .map_err(|err| format!("unknown image format: {:?}", err))?;
        let image = image::load_from_memory_with_format(data, format)
            .map_err(|err| format!("error parsing image: {:?}", err))?;
        *image_data = Some(Arc::new(image.to_rgba8()));
        Ok(())
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn as_rgba_image(&self) -> Option<Arc<RgbaImage>> {
        let data = self.data.lock().unwrap();
        data.as_ref().cloned()
    }
}

#[derive(Default, Debug)]
pub struct Configuration {
    /// The file path to load the assets from. If a relative path is provided,
    /// it will be relative to the cargo workspace root or the executable, if
    /// not executed by `cargo`.
    ///
    /// If not set, a folder named `assets` will be used.
    pub assets_path: Option<PathBuf>,

    /// The base url for assets to be loaded over http.
    pub asset_base_url: Option<String>,
}
