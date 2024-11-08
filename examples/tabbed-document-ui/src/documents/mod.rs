use slotmap::new_key_type;
use crate::documents::text::TextDocument;

pub mod text;

new_key_type! {
    /// A key for a document
    pub struct DocumentKey;
}

pub enum DocumentKind {
    TextDocument(TextDocument),
}
