//! Types for loading fonts to use in Cushy.
use std::sync::Arc;

use alot::{LotId, Lots};
use kludgine::cosmic_text::fontdb::{self, Language};
use kludgine::cosmic_text::{Stretch, Style, Weight};

use crate::value::Dynamic;

/// A collection of fonts that can be loaded into Cushy.
#[derive(Clone, Default, PartialEq)]
pub struct FontCollection(pub(crate) Dynamic<FontCollectionData>);

impl FontCollection {
    /// Pushes a font that will be unloaded when the last clone of the
    /// [`LoadedFont`] is dropped.
    #[must_use]
    pub fn push_unloadable(&self, font_data: Vec<u8>) -> LoadedFont {
        LoadedFont(Arc::new(LoadedFontHandle {
            collection: self.clone(),
            id: self.push_inner(font_data),
        }))
    }

    /// Adds `font_data` to this collection and returns self.
    #[must_use]
    pub fn with(self, font_data: Vec<u8>) -> Self {
        self.push(font_data);
        self
    }

    /// Pushes `font_data` into this collection.
    pub fn push(&self, font_data: Vec<u8>) {
        self.push_inner(font_data);
    }

    fn push_inner(&self, font_data: Vec<u8>) -> LotId {
        self.0.lock().fonts.push(Arc::new(font_data))
    }
}

pub(crate) struct FontIter<'a> {
    collection: *const (),
    iter: alot::unordered::EntryIter<'a, Arc<Vec<u8>>>,
}

impl<'a> Iterator for FontIter<'a> {
    type Item = (LoadedFontId, &'a Arc<Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(id, data)| {
            (
                LoadedFontId {
                    collection: self.collection,
                    id,
                },
                data,
            )
        })
    }
}

#[derive(Default)]
pub(crate) struct FontCollectionData {
    fonts: Lots<Arc<Vec<u8>>>,
}

impl FontCollectionData {
    pub(crate) fn fonts(&self, collection: &FontCollection) -> FontIter<'_> {
        FontIter {
            collection: collection.0.as_ptr(),
            iter: self.fonts.entries(),
        }
    }
}

#[derive(PartialEq)]
struct LoadedFontHandle {
    collection: FontCollection,
    id: LotId,
}

impl Drop for LoadedFontHandle {
    fn drop(&mut self) {
        let mut collection = self.collection.0.lock();
        collection.fonts.remove(self.id);
    }
}

/// A handle to font data that has been loaded.
///
/// This type can be cloned and uses reference counting to track when to release
/// the underlying font data.
///
/// Font data is not parsed until it is loaded into a running Cushy window. To
/// find information about this handle, use
/// [`WidgetContext::loaded_font_faces()`](crate::context::WidgetContext::loaded_font_faces).
#[derive(PartialEq, Clone)]
pub struct LoadedFont(Arc<LoadedFontHandle>);

impl LoadedFont {
    pub(crate) fn id(&self) -> LoadedFontId {
        LoadedFontId {
            collection: self.0.collection.0.as_ptr(),
            id: self.0.id,
        }
    }
}
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub(crate) struct LoadedFontId {
    pub(crate) collection: *const (),
    pub(crate) id: LotId,
}

/// Information about a [`LoadedFont`].
#[derive(Debug)]
pub struct LoadedFontFace {
    /// The font database ID for this face.
    pub id: fontdb::ID,
    /// The names of the families contained in this face, and the corresponding
    /// language of the name.
    pub families: Vec<(String, Language)>,
    /// The weight of the font face.
    pub weight: Weight,
    /// The style of the font face.
    pub style: Style,
    /// The stretch of the font face.
    pub stretch: Stretch,
}
