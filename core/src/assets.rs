#![allow(missing_docs)]

use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    styles::{FontStyle, Weight},
    AnyFrontend, AnySendSync, Callback,
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
    data: Arc<Mutex<Option<Box<dyn AnySendSync>>>>,
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
        frontend.load_image(self, on_loaded, on_error);
    }

    pub fn set_data<D: AnySendSync>(&self, new_data: D) {
        let mut data = self.data.lock().unwrap();
        *data = Some(Box::new(new_data) as Box<dyn AnySendSync>);
    }

    pub fn map_data<R, F: FnOnce(Option<&mut dyn AnySendSync>) -> R>(&self, callback: F) -> R {
        let mut data = self.data.lock().unwrap();
        callback(data.as_deref_mut())
    }
}

pub trait FrontendImage: AnySendSync {
    // TODO anyhow?
    fn load_data(&mut self, data: Vec<u8>) -> Result<(), String>;
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
