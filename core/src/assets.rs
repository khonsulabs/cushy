use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use url::Url;

use crate::{AnyFrontend, AnySendSync, Callback};

/// A loadable asset.
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
    /// Returns an asset [`Builder`].
    pub fn build() -> Builder {
        Builder::new()
    }

    /// Returns the path components.
    #[must_use]
    pub fn path(&self) -> &[Cow<'static, str>] {
        &self.data.path
    }
}

/// Builds an [`Asset`]
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

    /// Sets the relative path of this asset.
    pub fn path<S: Into<Cow<'static, str>>>(mut self, segments: Vec<S>) -> Self {
        self.asset.path = segments.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the sha1 checksum of the asset. This can be used by caches to
    /// ensure a file is up-to-date and valid.
    pub fn sha1sum(mut self, sha1sum: [u8; 20]) -> Self {
        self.asset.sha1sum = Some(sha1sum);
        self
    }

    /// Returns the build asset.
    pub fn finish(self) -> Asset {
        Asset {
            data: Arc::new(self.asset),
        }
    }
}

/// A loadable image asset.
#[derive(Debug, Clone)]
#[must_use]
pub struct Image {
    /// The asset definition for this image.
    pub asset: Asset,
    data: Arc<Mutex<Option<Box<dyn AnySendSync>>>>,
}

impl Image {
    /// Loads the image. When loaded successfully, `on_loaded` will be invoked.
    /// If an error occurs while loading, `on_error` will be invoked.
    #[allow(clippy::missing_panics_doc)]
    pub fn load(
        &self,
        on_loaded: Callback<Image>,
        on_error: Callback<String>,
        frontend: &dyn AnyFrontend,
    ) {
        frontend.load_image(self, on_loaded, on_error);
    }

    /// Sets the internal frontend data for this image. Should not be used outside of developing a frontend.
    #[allow(clippy::missing_panics_doc)]
    pub fn set_data<D: AnySendSync>(&self, new_data: D) {
        let mut data = self.data.lock().unwrap();
        *data = Some(Box::new(new_data) as Box<dyn AnySendSync>);
    }

    /// Maps the frontend data into `callback` returning the result of the
    /// function. This is generally only useful when developing a frontend.
    #[allow(clippy::missing_panics_doc)]
    pub fn map_data<R, F: FnOnce(Option<&mut dyn AnySendSync>) -> R>(&self, callback: F) -> R {
        let mut data = self.data.lock().unwrap();
        callback(data.as_deref_mut())
    }
}

impl From<Asset> for Image {
    fn from(asset: Asset) -> Self {
        Self {
            asset,
            data: Arc::default(),
        }
    }
}

/// Configuration for loading assets.
#[derive(Default, Debug)]
pub struct Configuration {
    /// The file path to load the assets from. If a relative path is provided,
    /// it will be relative to the cargo workspace root or the executable, if
    /// not executed by `cargo`.
    ///
    /// If not set, a folder named `assets` will be used.
    pub assets_path: Option<PathBuf>,

    /// The base url for assets to be loaded over http.
    pub asset_base_url: Option<Url>,
}
