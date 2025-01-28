use crate::documents::image::ImageDocument;
use slotmap::new_key_type;
use crate::documents::text::TextDocument;

pub mod text;
pub mod image;

new_key_type! {
    /// A key for a document
    pub struct DocumentKey;
}

pub enum DocumentKind {
    TextDocument(TextDocument),
    ImageDocument(ImageDocument),
}
